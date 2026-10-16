//! Installer guard, run as `biank-desktop.exe --prepare-install <dir> [uninstall]`
//! from the NSIS hooks. Native on purpose: PowerShell can be disabled by policy,
//! Smart App Control or antivirus on customer machines.
//!
//! Exit codes for the hook: 0 ready, 2 the shell is busy (retryable), 1 files
//! stay locked. Anything else, including failing to start, means the guard is
//! unavailable and the installer continues with Tauri's own running-app check.

pub const FLAG: &str = "--prepare-install";
pub const READY: i32 = 0;
pub const LOCKED: i32 = 1;
pub const BUSY: i32 = 2;

/// Returns the exit code when the process was started as the installer guard.
pub fn from_args() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != FLAG {
        return None;
    }
    let directory = std::path::PathBuf::from(args.next()?);
    let uninstall = args.next().is_some_and(|mode| mode == "uninstall");
    Some(platform::prepare(&directory, uninstall))
}

#[cfg(not(windows))]
mod platform {
    pub fn prepare(_: &std::path::Path, _: bool) -> i32 {
        super::READY
    }
}

#[cfg(windows)]
mod platform {
    use super::{BUSY, LOCKED, READY};
    use std::{
        ffi::OsString,
        os::windows::{ffi::OsStringExt, fs::OpenOptionsExt},
        path::{Path, PathBuf},
        process::Command,
        thread::sleep,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
                PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            },
        },
    };
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const SHELL: &str = "biank-desktop.exe";

    fn report(message: &str) {
        println!("{message}");
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy()
            .trim_end_matches('\\')
            .to_lowercase()
    }

    /// Only Biank's own executables: the shell and everything under `runtime\`
    /// (node, codex, Chromium, cloudflared). Never every process under the
    /// directory, which may be a shared folder such as `AppData\Local`.
    struct Owned {
        shell: String,
        runtime: String,
    }

    impl Owned {
        fn new(directory: &Path) -> Self {
            Self {
                shell: key(&directory.join(SHELL)),
                runtime: key(&directory.join("runtime")) + "\\",
            }
        }
        fn contains(&self, executable: &str) -> bool {
            executable == self.shell || executable.starts_with(&self.runtime)
        }
    }

    fn processes() -> Vec<(u32, String)> {
        let mut found = Vec::new();
        let own = std::process::id();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return found;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry) != 0;
            while more {
                let pid = entry.th32ProcessID;
                if pid != own && pid != 0 {
                    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                    if !process.is_null() {
                        let mut buffer = [0u16; 32768];
                        let mut size = buffer.len() as u32;
                        if QueryFullProcessImageNameW(
                            process,
                            PROCESS_NAME_WIN32,
                            buffer.as_mut_ptr(),
                            &mut size,
                        ) != 0
                        {
                            let path = OsString::from_wide(&buffer[..size as usize]);
                            found.push((pid, key(Path::new(&path))));
                        }
                        CloseHandle(process);
                    }
                }
                more = Process32NextW(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
        }
        found
    }

    fn owned(scope: &Owned, only_shell: bool) -> Vec<u32> {
        processes()
            .into_iter()
            .filter(|(_, path)| {
                if only_shell {
                    *path == scope.shell
                } else {
                    scope.contains(path)
                }
            })
            .map(|(pid, _)| pid)
            .collect()
    }

    fn wait_gone(scope: &Owned, only_shell: bool, limit: Duration) -> Vec<u32> {
        let deadline = Instant::now() + limit;
        loop {
            let left = owned(scope, only_shell);
            if left.is_empty() || Instant::now() >= deadline {
                return left;
            }
            sleep(Duration::from_millis(250));
        }
    }

    fn terminate(pid: u32) {
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, 0, pid);
            if !process.is_null() {
                TerminateProcess(process, 1);
                WaitForSingleObject(process, 5000);
                CloseHandle(process);
            }
        }
    }

    fn files(directory: &Path) -> Vec<PathBuf> {
        let mut found = vec![directory.join(SHELL)];
        let mut pending = vec![directory.join("runtime")];
        while let Some(folder) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => pending.push(path),
                    Ok(kind) if kind.is_file() => {
                        let extension = path
                            .extension()
                            .map(|value| value.to_string_lossy().to_lowercase());
                        if matches!(extension.as_deref(), Some("exe" | "dll" | "node")) {
                            found.push(path);
                        }
                    }
                    _ => {}
                }
            }
        }
        found.retain(|path| path.is_file());
        found
    }

    /// First file that cannot be opened exclusively. A read-only attribute is
    /// cleared instead: the installer is about to overwrite the file anyway.
    fn locked(directory: &Path) -> Option<(PathBuf, std::io::Error)> {
        for path in files(directory) {
            if let Ok(metadata) = std::fs::metadata(&path) {
                let mut permissions = metadata.permissions();
                if permissions.readonly() {
                    #[allow(clippy::permissions_set_readonly_false)]
                    permissions.set_readonly(false);
                    let _ = std::fs::set_permissions(&path, permissions);
                }
            }
            if let Err(error) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&path)
            {
                return Some((path, error));
            }
        }
        None
    }

    fn release(directory: &Path) -> i32 {
        let scope = Owned::new(directory);
        let shell = directory.join(SHELL);
        if !owned(&scope, true).is_empty() {
            // Single-instance IPC asks the live shell to drain; active work keeps
            // it open and the user decides when to retry. A live shell is never killed.
            if let Ok(mut request) = Command::new(&shell).arg("--installer-close").spawn() {
                let deadline = Instant::now() + Duration::from_secs(10);
                while Instant::now() < deadline && matches!(request.try_wait(), Ok(None)) {
                    sleep(Duration::from_millis(100));
                }
                let _ = request.kill();
            }
            if !wait_gone(&scope, true, Duration::from_secs(30)).is_empty() {
                report("Biank sigue abierto o terminando un trabajo.");
                return BUSY;
            }
        }
        // Without its shell nothing drains the runtime: whatever is left (crash,
        // uninstall while open, older versions) only holds files.
        for pid in wait_gone(&scope, false, Duration::from_secs(5)) {
            terminate(pid);
        }
        if !wait_gone(&scope, false, Duration::from_secs(10)).is_empty() {
            report("Quedaron procesos de Biank que no se pudieron cerrar.");
            return LOCKED;
        }
        // Antivirus and indexers hold freshly touched binaries for a moment.
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match locked(directory) {
                None => break,
                Some((path, error)) if Instant::now() >= deadline => {
                    report(&format!("Archivo en uso: {} ({error})", path.display()));
                    return LOCKED;
                }
                Some(_) => sleep(Duration::from_millis(500)),
            }
        }
        if !owned(&scope, false).is_empty() {
            report("Biank se volvio a abrir durante la preparacion.");
            return BUSY;
        }
        READY
    }

    struct Legacy {
        key: String,
        name: String,
        location: PathBuf,
        uninstaller: PathBuf,
        arguments: String,
    }

    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    /// The retired Electron build registered its own per-user uninstall entry and
    /// folder, so it survives next to this installation.
    fn legacy(directory: &Path) -> Vec<Legacy> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let Ok(root) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(UNINSTALL) else {
            return Vec::new();
        };
        let target = key(directory);
        root.enum_keys()
            .flatten()
            .filter(|name| name != "Biank")
            .filter_map(|name| {
                let entry = root.open_subkey(&name).ok()?;
                let display: String = entry.get_value("DisplayName").ok()?;
                let location: String = entry.get_value("InstallLocation").ok()?;
                let location = PathBuf::from(location.trim().trim_matches('"'));
                let folder = location.file_name()?.to_string_lossy().to_lowercase();
                // Only folders that are themselves a Biank installation are touched.
                if !display.to_lowercase().starts_with("biank")
                    || !folder.contains("biank")
                    || key(&location) == target
                    || !location.is_dir()
                {
                    return None;
                }
                let command: String = entry
                    .get_value("QuietUninstallString")
                    .or_else(|_| {
                        entry
                            .get_value::<String, _>("UninstallString")
                            .map(|value| format!("{value} /S"))
                    })
                    .ok()?;
                let rest = command.trim().strip_prefix('"')?;
                let (program, arguments) = rest.split_once('"')?;
                let uninstaller = PathBuf::from(program);
                if !key(&uninstaller).starts_with(&(key(&location) + "\\")) || !uninstaller.is_file() {
                    return None;
                }
                Some(Legacy {
                    key: name,
                    name: display,
                    location,
                    uninstaller,
                    arguments: arguments.trim().to_string(),
                })
            })
            .collect()
    }

    fn retire(legacy: &Legacy) -> Result<(), String> {
        let prefix = key(&legacy.location) + "\\";
        let pids = || {
            processes()
                .into_iter()
                .filter(|(_, path)| path.starts_with(&prefix))
                .map(|(pid, _)| pid)
                .collect::<Vec<_>>()
        };
        for pid in pids() {
            terminate(pid);
        }
        if !pids().is_empty() {
            return Err("sigue abierta".into());
        }
        // Silent uninstall keeps user data, which the vault imports on first start.
        let mut command = Command::new(&legacy.uninstaller);
        if !legacy.arguments.is_empty() {
            use std::os::windows::process::CommandExt;
            command.raw_arg(&legacy.arguments);
        }
        command.status().map_err(|error| error.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(60);
        let present = || {
            winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
                .open_subkey(format!("{UNINSTALL}\\{}", legacy.key))
                .is_ok()
        };
        while present() && Instant::now() < deadline {
            sleep(Duration::from_millis(500));
        }
        if present() {
            return Err("el desinstalador no termino".into());
        }
        Ok(())
    }

    pub fn prepare(directory: &Path, uninstall: bool) -> i32 {
        if !directory.is_absolute() || directory.parent().is_none() {
            report("Ruta de instalacion invalida.");
            return LOCKED;
        }
        let code = release(directory);
        if code == READY && !uninstall {
            for old in legacy(directory) {
                // A leftover old version never blocks the new one; it is reported.
                match retire(&old) {
                    Ok(()) => report(&format!("Version anterior retirada: {}", old.name)),
                    Err(error) => report(&format!("No se pudo retirar {}: {error}", old.name)),
                }
            }
        }
        code
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::process::Child;

        struct Fixture {
            root: PathBuf,
            children: Vec<Child>,
        }
        impl Fixture {
            fn new() -> Self {
                let root = std::env::temp_dir().join(format!("biank-guard-{}", rand::random::<u64>()));
                std::fs::create_dir_all(root.join("installed/runtime/node")).unwrap();
                std::fs::create_dir_all(root.join("unrelated")).unwrap();
                Self { root, children: Vec::new() }
            }
            fn idle(&mut self, executable: PathBuf) -> u32 {
                std::fs::copy(r"C:\Windows\System32\PING.EXE", &executable).unwrap();
                let child = Command::new(executable)
                    .args(["-n", "600", "127.0.0.1"])
                    .stdout(std::process::Stdio::null())
                    .spawn()
                    .unwrap();
                let pid = child.id();
                self.children.push(child);
                pid
            }
            fn alive(&mut self, pid: u32) -> bool {
                self.children
                    .iter_mut()
                    .find(|child| child.id() == pid)
                    .is_some_and(|child| matches!(child.try_wait(), Ok(None)))
            }
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                for child in &mut self.children {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let _ = std::fs::remove_dir_all(&self.root);
            }
        }

        #[test]
        fn installer_guard_contract() {
            let mut fixture = Fixture::new();
            let installed = fixture.root.join("installed");
            let unrelated = fixture.idle(fixture.root.join("unrelated/node.exe"));
            // A sibling of the installation that is not Biank's own executable.
            let neighbour = fixture.idle(installed.join("other-app.exe"));
            assert_eq!(release(&installed), READY);

            let orphan = fixture.idle(installed.join("runtime/node/node.exe"));
            assert_eq!(release(&installed), READY, "orphan runtime must not block");
            assert!(!fixture.alive(orphan), "orphan runtime must be ended");
            assert!(fixture.alive(unrelated) && fixture.alive(neighbour), "only Biank is touched");

            // A shell that does not drain (active work) is never killed.
            let shell = fixture.idle(installed.join(SHELL));
            assert_eq!(release(&installed), BUSY);
            assert!(fixture.alive(shell), "busy shell must survive");
            fixture.children.retain_mut(|child| {
                if child.id() == shell {
                    let _ = child.kill();
                    let _ = child.wait();
                    false
                } else {
                    true
                }
            });

            let dll = installed.join("runtime/node/locked.dll");
            std::fs::write(&dll, "fixture").unwrap();
            let hold = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&dll)
                .unwrap();
            assert_eq!(release(&installed), LOCKED);
            drop(hold);
            let mut permissions = std::fs::metadata(&dll).unwrap().permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&dll, permissions).unwrap();
            assert_eq!(release(&installed), READY, "read-only is not a lock");
            assert_eq!(std::fs::read_to_string(&dll).unwrap(), "fixture");
            assert_eq!(prepare(Path::new("relative"), false), LOCKED);
            assert_eq!(prepare(Path::new(r"C:\"), false), LOCKED);
        }

        #[test]
        fn legacy_electron_is_retired() {
            use winreg::{enums::HKEY_CURRENT_USER, RegKey};
            let mut fixture = Fixture::new();
            let location = fixture.root.join("biank-desktop");
            std::fs::create_dir_all(&location).unwrap();
            let name = format!("biank-guard-test-{}", rand::random::<u64>());
            let app = fixture.idle(location.join("Biank Desktop.exe"));
            let uninstaller = location.join("Uninstall biank-desktop.exe");
            std::fs::copy(r"C:\Windows\System32\cmd.exe", &uninstaller).unwrap();
            let (entry, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey(format!("{UNINSTALL}\\{name}"))
                .unwrap();
            entry.set_value("DisplayName", &"biank-desktop 0.2.31").unwrap();
            entry.set_value("InstallLocation", &location.to_string_lossy().to_string()).unwrap();
            entry
                .set_value(
                    "QuietUninstallString",
                    &format!(
                        "\"{}\" /c reg delete \"HKCU\\{UNINSTALL}\\{name}\" /f",
                        uninstaller.display()
                    ),
                )
                .unwrap();
            let installed = fixture.root.join("installed");
            let found: Vec<_> = legacy(&installed).into_iter().filter(|item| item.key == name).collect();
            assert_eq!(found.len(), 1, "legacy install must be detected");
            assert!(legacy(&location).iter().all(|item| item.key != name), "target is not legacy");
            let result = retire(&found[0]);
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(format!("{UNINSTALL}\\{name}"));
            result.unwrap();
            assert!(!fixture.alive(app), "legacy app must be closed");
        }
    }
}

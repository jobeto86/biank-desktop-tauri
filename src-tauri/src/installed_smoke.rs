//! CUR160-SMOKE/1. Pure validation first; never use the manifest as authority
//! for an ordinary installation. No key, arbitrary vault reference or resolver.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const CONTEXT_ENV: &str = "BIANK_INSTALLED_SMOKE_CONTEXT";
pub fn os_environment_key(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "path"
            | "pathext"
            | "systemroot"
            | "windir"
            | "comspec"
            | "temp"
            | "tmp"
            | "tmpdir"
            | "home"
            | "userprofile"
            | "localappdata"
            | "appdata"
            | "programfiles"
            | "programfiles(x86)"
            | "commonprogramfiles"
            | "commonprogramfiles(x86)"
            | "programdata"
            | "username"
            | "userdomain"
            | "user"
            | "logname"
            | "shell"
            | "lang"
            | "display"
            | "wayland_display"
            | "xauthority"
            | "xdg_runtime_dir"
            | "dbus_session_bus_address"
            | "number_of_processors"
            | "processor_architecture"
            | "__cf_user_text_encoding"
    ) || (name.starts_with("LC_")
        && name[3..]
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c == b'_'))
}
const PURPOSE: &str = "cur160-installed-smoke";
const INVALID: &str = "CUR160_CONTEXT_INVALID";
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub schema_version: u8,
    pub purpose: String,
    pub root: String,
    pub run_id: String,
    pub nonce: String,
    pub boot: u8,
    pub candidate_sha: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Initial {
    schema_version: u8,
    purpose: String,
    root: String,
    run_id: String,
    initial_nonce: String,
    candidate_sha: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultReceipt {
    schema_version: u8,
    kind: String,
    run_id: String,
    nonce: String,
    boot: u8,
    candidate_sha: String,
    root_sha256: String,
    engine_instance_id: String,
    engine_pid: u64,
    operation: String,
    digest_sha256: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UiReport {
    nonce: String,
    health: bool,
    login: bool,
    account_required: bool,
    unauthenticated: bool,
    bridge: bool,
}
#[derive(Clone)]
pub struct Validated {
    pub context: Context,
    context_raw: Vec<u8>,
    initial_raw: Vec<u8>,
    initial_nonce: String,
}
pub struct Session {
    pub validated: Validated,
    pub engine_instance: String,
    pub engine_pid: u32,
    pub origin: String,
    pub reported: std::sync::atomic::AtomicBool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyAccounts {
    version: u8,
    active: (),
    users: Vec<()>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EmptyProfiles {
    version: u8,
    legacy_owner: (),
    legacy_stores: (),
    backup: (),
}
pub struct BoundCandidate {
    sha: String,
}
impl BoundCandidate {
    pub fn sha(&self) -> &str {
        &self.sha
    }
}
fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn canonical(path: &Path) -> Result<PathBuf> {
    let actual = fs::canonicalize(path).map_err(|_| INVALID)?;
    let actual = dunce::simplified(&actual).to_path_buf();
    if !path.is_absolute() || path.as_os_str() != actual.as_os_str() {
        return Err(INVALID.into());
    }
    Ok(actual)
}
fn regular(path: &Path) -> Result<()> {
    let info = fs::symlink_metadata(path).map_err(|_| INVALID)?;
    if !info.is_file() || info.file_type().is_symlink() {
        return Err(INVALID.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if info.nlink() != 1 {
            return Err(INVALID.into());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
        if info.file_attributes() & 0x400 != 0 {
            return Err(INVALID.into());
        }
        // Stable std does not yet expose nNumberOfLinks on Windows.
        #[repr(C)]
        struct FileInformation {
            attributes: u32,
            times: [u32; 6],
            volume: u32,
            size_high: u32,
            size_low: u32,
            links: u32,
            index_high: u32,
            index_low: u32,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetFileInformationByHandle(
                handle: *mut std::ffi::c_void,
                info: *mut FileInformation,
            ) -> i32;
        }
        let file = fs::File::open(path).map_err(|_| INVALID)?;
        let mut out = std::mem::MaybeUninit::<FileInformation>::uninit();
        // The API initializes the complete documented BY_HANDLE_FILE_INFORMATION
        // layout on success; file owns the handle for the duration of this call.
        let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), out.as_mut_ptr()) };
        if ok == 0 {
            return Err(INVALID.into());
        }
        let out = unsafe { out.assume_init() };
        if out.links != 1 || out.attributes & 0x400 != 0 {
            return Err(INVALID.into());
        }
    }
    Ok(())
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<(T, Vec<u8>)> {
    regular(path)?;
    canonical(path)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| INVALID)?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| INVALID)?;
    if bytes.len() > 4096 || bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(INVALID.into());
    }
    let value = serde_json::from_slice(&bytes).map_err(|_| INVALID)?;
    Ok((value, bytes))
}
fn names(path: &Path) -> Result<Vec<String>> {
    let mut values = fs::read_dir(path)
        .map_err(|_| INVALID)?
        .map(|entry| {
            entry
                .map_err(|_| INVALID.to_string())?
                .file_name()
                .into_string()
                .map_err(|_| INVALID.to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    Ok(values)
}
fn tree(root: &Path, path: &Path, depth: usize, count: &mut usize) -> Result<()> {
    *count += 1;
    if *count > 4096 || depth > 32 {
        return Err(INVALID.into());
    }
    let info = fs::symlink_metadata(path).map_err(|_| INVALID)?;
    if info.file_type().is_symlink() {
        return Err(INVALID.into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if info.file_attributes() & 0x400 != 0 {
            return Err(INVALID.into());
        }
    }
    if !canonical(path)?.starts_with(root) {
        return Err(INVALID.into());
    }
    let rel = path.strip_prefix(root).map_err(|_| INVALID)?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if (rel == Path::new("config/access.env") || rel == Path::new("config/google-client.secret"))
        && (!info.is_file() || info.len() != 0)
    {
        return Err(INVALID.into());
    }
    if rel.starts_with("data/profiles")
        || matches!(
            name,
            "auth.json"
                | "google-identity-session.json"
                | "cbsuite-identity-session.json"
                | "personal-profile.json"
                | "vault.key"
                | "vault.key.sealed"
                | "master.key"
        )
    {
        return Err(INVALID.into());
    }
    if info.is_dir() {
        for child in fs::read_dir(path).map_err(|_| INVALID)? {
            tree(root, &child.map_err(|_| INVALID)?.path(), depth + 1, count)?;
        }
    } else {
        regular(path)?;
        if rel == Path::new("data/product-accounts.json")
            || rel == Path::new("data/identity/product-accounts.json")
        {
            let (accounts, _): (EmptyAccounts, _) = read_json(path)?;
            if accounts.version != 1 || !accounts.users.is_empty() {
                return Err(INVALID.into());
            }
            let _ = accounts.active;
        } else if rel.starts_with("data/identity") {
            if !matches!(
                name,
                "vault.db" | "vault.db-wal" | "vault.db-shm" | "vault.db-journal"
            ) {
                return Err(INVALID.into());
            }
        }
        if rel == Path::new("data/account-profiles.json") {
            let (profiles, _): (EmptyProfiles, _) = read_json(path)?;
            if profiles.version != 1 {
                return Err(INVALID.into());
            }
            let _ = (
                profiles.legacy_owner,
                profiles.legacy_stores,
                profiles.backup,
            );
        }
    }
    Ok(())
}
pub fn preflight(development: bool) -> Result<Option<Validated>> {
    validate(std::env::var_os(CONTEXT_ENV), development, |key| {
        std::env::var_os(key)
    })
}
// Absence returns before consulting root, tmp, env overrides or any filesystem.
pub fn validate(
    context_path: Option<OsString>,
    development: bool,
    env: impl Fn(&str) -> Option<OsString>,
) -> Result<Option<Validated>> {
    let Some(context_path) = context_path else {
        return Ok(None);
    };
    if development
        || [
            "BIANK_TAURI_TEST_ROOT",
            "BIANK_TAURI_TEST_RESOURCE_ROOT",
            "BIANK_DESKTOP_VAULT_KEY_FILE",
        ]
        .iter()
        .any(|key| env(key).is_some())
    {
        return Err(INVALID.into());
    }
    if env("BIANK_DESKTOP_DEV").as_deref() != Some(std::ffi::OsStr::new("0"))
        || env("BIANK_STANDALONE").as_deref() != Some(std::ffi::OsStr::new("1"))
    {
        return Err(INVALID.into());
    }
    let context_path = PathBuf::from(context_path);
    canonical(&context_path)?;
    // Constrain the location BEFORE reading any caller-selected bytes.
    let qa = context_path.parent().ok_or(INVALID)?;
    let inferred_root = qa.parent().ok_or(INVALID)?;
    let temp = fs::canonicalize(std::env::temp_dir()).map_err(|_| INVALID)?;
    let basename = inferred_root
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(INVALID)?;
    if context_path.file_name().and_then(|n| n.to_str()) != Some("context.json")
        || qa.file_name().and_then(|n| n.to_str()) != Some("qa")
        || inferred_root.parent() != Some(dunce::simplified(&temp))
        || !basename
            .strip_prefix("biank-installed-smoke-")
            .is_some_and(|s| hex(s, 32))
        || env("BIANK_DATA_ROOT").as_deref() != Some(inferred_root.as_os_str())
    {
        return Err(INVALID.into());
    }
    let (context, context_raw): (Context, _) = read_json(&context_path)?;
    if context.schema_version != 1
        || context.purpose != PURPOSE
        || !hex(&context.run_id, 32)
        || !hex(&context.nonce, 64)
        || !hex(&context.candidate_sha, 40)
        || !matches!(context.boot, 1 | 2)
    {
        return Err(INVALID.into());
    }
    let root = canonical(Path::new(&context.root))?;
    if root.parent() != Some(dunce::simplified(&temp))
        || root.file_name().and_then(|n| n.to_str())
            != Some(format!("biank-installed-smoke-{}", context.run_id).as_str())
        || context_path != root.join("qa/context.json")
    {
        return Err(INVALID.into());
    }
    if env("BIANK_DATA_ROOT").as_deref() != Some(root.as_os_str()) {
        return Err(INVALID.into());
    }
    for (key, expected) in [
        ("BIANK_DESKTOP_DATA_ROOT", root.join("data")),
        ("BIANK_DESKTOP_CONFIG_ROOT", root.join("config")),
        ("BIANK_ACCESS_ENV_FILE", root.join("config/access.env")),
        (
            "BIANK_GOOGLE_CLIENT_SECRET_FILE",
            root.join("config/google-client.secret"),
        ),
    ] {
        if env(key).as_deref() != Some(expected.as_os_str()) {
            return Err(INVALID.into());
        }
    }
    let (initial, initial_raw): (Initial, _) = read_json(&root.join("qa/initial.json"))?;
    if initial.schema_version != 1
        || initial.purpose != PURPOSE
        || initial.root != context.root
        || initial.run_id != context.run_id
        || initial.candidate_sha != context.candidate_sha
        || !hex(&initial.initial_nonce, 64)
        || (context.boot == 1) != (context.nonce == initial.initial_nonce)
    {
        return Err(INVALID.into());
    }
    tree(&root, &root, 0, &mut 0)?;
    if context.boot == 1
        && (names(&root)? != ["qa"] || names(&root.join("qa"))? != ["context.json", "initial.json"])
    {
        return Err(INVALID.into());
    }
    let validated = Validated {
        context,
        context_raw,
        initial_raw,
        initial_nonce: initial.initial_nonce,
    };
    if validated.context.boot == 2 {
        let first = validated.read_vault(1)?;
        if first.nonce != validated.initial_nonce {
            return Err(INVALID.into());
        }
    }
    Ok(Some(validated))
}
impl Validated {
    pub fn root(&self) -> &Path {
        Path::new(&self.context.root)
    }
    pub fn revalidate(&self) -> Result<()> {
        tree(self.root(), self.root(), 0, &mut 0)?;
        let (_, context): (Context, _) = read_json(&self.root().join("qa/context.json"))?;
        let (_, initial): (Initial, _) = read_json(&self.root().join("qa/initial.json"))?;
        if context != self.context_raw || initial != self.initial_raw {
            return Err(INVALID.into());
        }
        Ok(())
    }
    // Only a hash in the COMPILED resource manifest can bind runtime metadata.
    // Staging must supply coreSha; a context's expected SHA is never provenance.
    pub fn bind_candidate(
        &self,
        compiled_manifest: &str,
        runtime: &Path,
    ) -> Result<BoundCandidate> {
        let manifest: Value =
            serde_json::from_str(compiled_manifest).map_err(|_| "CUR160_CANDIDATE_UNBOUND")?;
        let file = runtime.join("runtime-config.json");
        let (config, bytes): (Value, _) = read_json(&file)?;
        if manifest["files"]["runtime-config.json"]["sha256"].as_str()
            != Some(digest(&bytes).as_str())
            || config["coreSha"].as_str() != Some(self.context.candidate_sha.as_str())
        {
            return Err("CUR160_CANDIDATE_UNBOUND".into());
        }
        Ok(BoundCandidate {
            sha: config["coreSha"]
                .as_str()
                .ok_or("CUR160_CANDIDATE_UNBOUND")?
                .to_owned(),
        })
    }
    pub fn probe_script(&self, template: &str, instance: &str, origin: &str) -> Result<String> {
        let port = origin
            .strip_prefix("http://127.0.0.1:")
            .and_then(|s| s.parse::<u16>().ok());
        if port.is_none_or(|p| p == 0)
            || template.matches("__CUR160_SMOKE_CONTEXT__").count() != 1
            || !(16..=128).contains(&instance.len())
            || !instance
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err("CUR160_PROBE_INVALID".into());
        }
        Ok(template.replace(
            "__CUR160_SMOKE_CONTEXT__",
            &json!({"nonce":self.context.nonce,"engineInstanceId":instance,"origin":origin})
                .to_string(),
        ))
    }
    // Called only after pristine preflight and candidate binding. It must never
    // run on absence of CONTEXT_ENV or on an ordinary profile.
    pub fn prepare_config(&self) -> Result<()> {
        self.revalidate()?;
        fs::create_dir_all(self.root().join("config")).map_err(|_| INVALID)?;
        for name in ["access.env", "google-client.secret"] {
            let file = self.root().join("config").join(name);
            if self.context.boot == 1 {
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(file)
                    .map_err(|_| INVALID)?;
            } else {
                regular(&file)?;
                if fs::metadata(file).map_err(|_| INVALID)?.len() != 0 {
                    return Err(INVALID.into());
                }
            }
        }
        Ok(())
    }
    pub fn child_context(
        &self,
        cmd: &mut std::process::Command,
        candidate: &BoundCandidate,
    ) -> Result<()> {
        self.revalidate()?;
        if candidate.sha != self.context.candidate_sha {
            return Err("CUR160_CANDIDATE_UNBOUND".into());
        }
        // Values contain context and status only. The key remains exclusively
        // on the existing vault stdin path; no key bytes reach an attestation.
        cmd.env(CONTEXT_ENV,self.root().join("qa/context.json"))
            .env("BIANK_INSTALLED_SMOKE_CORE_SHA",candidate.sha())
            .env("BIANK_INSTALLED_SMOKE_SHELL_ATTESTATION",json!({"schemaVersion":1,
                "runId":self.context.run_id,"nonce":self.context.nonce,"boot":self.context.boot,
                "candidateSha":self.context.candidate_sha,"rootSha256":digest(self.context.root.as_bytes()),
                "nativeKeyring":true,"development":false,"keyFileOverride":false,"resourceOverride":false,"stdinKey":true}).to_string())
            .env("BIANK_DESKTOP_CONFIG_ROOT",self.root().join("config"))
            .env("BIANK_ACCESS_ENV_FILE",self.root().join("config/access.env"))
            .env("BIANK_GOOGLE_CLIENT_SECRET_FILE",self.root().join("config/google-client.secret"));
        Ok(())
    }
    pub fn read_vault(&self, boot: u8) -> Result<VaultReceipt> {
        let (value, _): (VaultReceipt, _) =
            read_json(&self.root().join(format!("qa/vault-boot-{boot}.json")))?;
        if value.schema_version != 1
            || value.kind != "vault"
            || value.run_id != self.context.run_id
            || value.boot != boot
            || value.candidate_sha != self.context.candidate_sha
            || value.root_sha256 != digest(self.context.root.as_bytes())
            || !hex(&value.nonce, 64)
            || !hex(&value.digest_sha256, 64)
            || value.engine_pid == 0
            || value.engine_pid > 9007199254740991
            || !(16..=128).contains(&value.engine_instance_id.len())
            || !value
                .engine_instance_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || value.operation != if boot == 1 { "put" } else { "resolve" }
        {
            return Err("CUR160_VAULT_RECEIPT_INVALID".into());
        }
        Ok(value)
    }
    pub fn identity(&self, instance: &str, pid: u32, kind: &str) -> Value {
        json!({"schemaVersion":1,"kind":kind,"runId":self.context.run_id,"nonce":self.context.nonce,
            "boot":self.context.boot,"candidateSha":self.context.candidate_sha,"rootSha256":digest(self.context.root.as_bytes()),
            "engineInstanceId":instance,"enginePid":pid,"shellPid":std::process::id()})
    }
    fn publish(&self, kind: &str, value: Value) -> Result<()> {
        self.revalidate()?;
        let path = self
            .root()
            .join(format!("qa/{kind}-boot-{}.json", self.context.boot));
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|_| "CUR160_RECEIPT_EXISTS")?;
        file.write_all(value.to_string().as_bytes())
            .map_err(|_| "CUR160_RECEIPT_WRITE")?;
        file.sync_all().map_err(|_| "CUR160_RECEIPT_WRITE")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        root: PathBuf,
        context: Context,
    }
    impl Fixture {
        fn new() -> Self {
            let id = format!(
                "{:032x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let temp = fs::canonicalize(std::env::temp_dir()).unwrap();
            let root = dunce::simplified(&temp).join(format!("biank-installed-smoke-{id}"));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("qa")).unwrap();
            let context = Context {
                schema_version: 1,
                purpose: PURPOSE.into(),
                root: root.to_str().unwrap().into(),
                run_id: id,
                nonce: "a".repeat(64),
                boot: 1,
                candidate_sha: "b".repeat(40),
            };
            fs::write(root.join("qa/initial.json"),json!({"schemaVersion":1,"purpose":PURPOSE,"root":context.root,
                "runId":context.run_id,"initialNonce":context.nonce,"candidateSha":context.candidate_sha}).to_string()).unwrap();
            let fixture = Self { root, context };
            fixture.write_context();
            fixture
        }
        fn write_context(&self) {
            fs::write(
                self.root.join("qa/context.json"),
                serde_json::to_vec(&self.context).unwrap(),
            )
            .unwrap();
        }
        fn env(&self, key: &str) -> Option<OsString> {
            match key {
                "BIANK_DATA_ROOT" => Some(self.root.as_os_str().to_owned()),
                "BIANK_DESKTOP_DEV" => Some("0".into()),
                "BIANK_STANDALONE" => Some("1".into()),
                "BIANK_DESKTOP_DATA_ROOT" => Some(self.root.join("data").into_os_string()),
                "BIANK_DESKTOP_CONFIG_ROOT" => Some(self.root.join("config").into_os_string()),
                "BIANK_ACCESS_ENV_FILE" => {
                    Some(self.root.join("config/access.env").into_os_string())
                }
                "BIANK_GOOGLE_CLIENT_SECRET_FILE" => Some(
                    self.root
                        .join("config/google-client.secret")
                        .into_os_string(),
                ),
                _ => None,
            }
        }
        fn preflight(&self) -> Result<Validated> {
            validate(
                Some(self.root.join("qa/context.json").into_os_string()),
                false,
                |k| self.env(k),
            )
            .map(|v| v.unwrap())
        }
        fn vault(&self, boot: u8, instance: &str, pid: u32) {
            fs::write(self.root.join(format!("qa/vault-boot-{boot}.json")),json!({"schemaVersion":1,"kind":"vault",
                "runId":self.context.run_id,"nonce":if boot==1 {"a".repeat(64)} else {self.context.nonce.clone()},"boot":boot,
                "candidateSha":self.context.candidate_sha,"rootSha256":digest(self.context.root.as_bytes()),"engineInstanceId":instance,
                "enginePid":pid,"operation":if boot==1 {"put"}else{"resolve"},"digestSha256":"c".repeat(64)}).to_string()).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    #[test]
    fn absent_context_has_no_io_or_env_fallback() {
        assert!(
            validate(None, true, |_| panic!("must not consult environment"))
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn preflight_is_read_only_and_pristine_precedes_config_or_logs() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        assert_eq!(names(&f.root).unwrap(), vec!["qa"]);
        v.prepare_config().unwrap();
        assert!(f.preflight().is_err());
        v.revalidate().unwrap();
        assert!(!f.root.join("logs").exists());
        assert!(!f.root.join("data").exists());
        fs::write(f.root.join("config/access.env"), "not empty").unwrap();
        assert!(v.revalidate().is_err());
    }
    #[test]
    fn duplicate_unknown_null_or_coerced_fields_are_rejected() {
        let f = Fixture::new();
        let raw = fs::read_to_string(f.root.join("qa/context.json")).unwrap();
        for bad in [
            raw.replacen('{', "{\"boot\":1,", 1),
            raw.replacen('{', "{\"extra\":1,", 1),
            raw.replace("\"boot\":1", "\"boot\":\"1\""),
            raw.replace("\"boot\":1", "\"boot\":null"),
            format!("\u{feff}{raw}"),
        ] {
            fs::write(f.root.join("qa/context.json"), bad).unwrap();
            assert!(f.preflight().is_err());
        }
        fs::write(f.root.join("qa/context.json"), " ".repeat(4097)).unwrap();
        assert!(f.preflight().is_err());
    }
    #[test]
    fn overrides_development_and_external_roots_fail_before_writes() {
        let f = Fixture::new();
        let path = Some(f.root.join("qa/context.json").into_os_string());
        assert!(validate(path.clone(), true, |k| f.env(k)).is_err());
        for key in [
            "BIANK_DESKTOP_VAULT_KEY_FILE",
            "BIANK_TAURI_TEST_ROOT",
            "BIANK_TAURI_TEST_RESOURCE_ROOT",
            "BIANK_DESKTOP_CONFIG_ROOT",
            "BIANK_ACCESS_ENV_FILE",
            "BIANK_GOOGLE_CLIENT_SECRET_FILE",
        ] {
            assert!(validate(path.clone(), false, |k| if k == key {
                Some("".into())
            } else {
                f.env(k)
            })
            .is_err());
        }
        assert!(validate(
            Some(f.root.join("qa/initial.json").into_os_string()),
            false,
            |k| f.env(k)
        )
        .is_err());
        assert_eq!(names(&f.root).unwrap(), vec!["qa"]);
    }
    #[test]
    fn actual_candidate_requires_compiled_hash_and_actual_core_sha() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        let runtime = f.root.join("runtime");
        fs::create_dir(&runtime).unwrap();
        let config = json!({"coreSha":f.context.candidate_sha,"googleIdentity":null}).to_string();
        fs::write(runtime.join("runtime-config.json"), &config).unwrap();
        let manifest =
            json!({"files":{"runtime-config.json":{"sha256":digest(config.as_bytes())}}})
                .to_string();
        v.bind_candidate(&manifest, &runtime).unwrap();
        assert!(v.bind_candidate("{}", &runtime).is_err());
        fs::write(
            runtime.join("runtime-config.json"),
            config.replace(&"b".repeat(40), &"d".repeat(40)),
        )
        .unwrap();
        assert!(v.bind_candidate(&manifest, &runtime).is_err());
        let no_sha = "{\"googleIdentity\":null}";
        fs::write(runtime.join("runtime-config.json"), no_sha).unwrap();
        let manifest =
            json!({"files":{"runtime-config.json":{"sha256":digest(no_sha.as_bytes())}}})
                .to_string();
        assert!(v.bind_candidate(&manifest, &runtime).is_err());
    }
    #[test]
    fn second_boot_requires_first_receipt_and_a_fresh_nonce() {
        let mut f = Fixture::new();
        f.context.boot = 2;
        f.write_context();
        assert!(f.preflight().is_err());
        f.context.nonce = "d".repeat(64);
        f.write_context();
        assert!(f.preflight().is_err());
        f.vault(1, "first_engine_identity", 42);
        f.preflight().unwrap();
    }
    fn report(nonce: &str) -> UiReport {
        serde_json::from_value(json!({"nonce":nonce,"health":true,"login":true,"accountRequired":true,"unauthenticated":true,"bridge":true})).unwrap()
    }
    #[test]
    fn ui_is_typed_bound_to_first_party_nonce_and_real_engine_receipt_and_replay_fails() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        f.vault(1, "current_engine_id", 42);
        let s = Session {
            validated: v,
            engine_instance: "current_engine_id".into(),
            engine_pid: 42,
            origin: "http://127.0.0.1:1234".into(),
            reported: false.into(),
        };
        assert!(s
            .accept_ui("startup", &s.origin, &report(&f.context.nonce))
            .is_err());
        assert!(s
            .accept_ui("main", "https://foreign.test", &report(&f.context.nonce))
            .is_err());
        assert!(s
            .accept_ui("main", &s.origin, &report(&"0".repeat(64)))
            .is_err());
        let mut bad = report(&f.context.nonce);
        bad.login = false;
        assert!(s.accept_ui("main", &s.origin, &bad).is_err());
        assert!(serde_json::from_value::<UiReport>(json!({"nonce":"x","health":1,"login":true,"accountRequired":true,"unauthenticated":true,"bridge":true})).is_err());
        assert!(serde_json::from_value::<UiReport>(json!({"nonce":"x","health":true,"login":true,"accountRequired":true,"unauthenticated":true,"bridge":true,"enginePid":42})).is_err());
        s.accept_ui("main", &s.origin, &report(&f.context.nonce))
            .unwrap();
        let (ui, _): (Value, _) = read_json(&f.root.join("qa/ui-boot-1.json")).unwrap();
        assert_eq!(ui.as_object().unwrap().len(), 15);
        assert_eq!(ui["kind"], "ui");
        assert_eq!(ui["shellPid"], std::process::id());
        assert!(s
            .accept_ui("main", &s.origin, &report(&f.context.nonce))
            .is_err());
        s.record_shutdown().unwrap();
        assert!(s.record_shutdown().is_err());
        let (shutdown, _): (Value, _) = read_json(&f.root.join("qa/shutdown-boot-1.json")).unwrap();
        assert_eq!(shutdown.as_object().unwrap().len(), 12);
        assert_eq!(shutdown["kind"], "shutdown");
    }
    #[test]
    fn context_changes_and_wrong_engine_receipts_fail_without_ui_evidence() {
        let mut f = Fixture::new();
        let v = f.preflight().unwrap();
        f.vault(1, "other_engine_identity", 43);
        let s = Session {
            validated: v,
            engine_instance: "current_engine_id".into(),
            engine_pid: 42,
            origin: "http://127.0.0.1:1234".into(),
            reported: false.into(),
        };
        assert!(s
            .accept_ui("main", &s.origin, &report(&f.context.nonce))
            .is_err());
        f.context.nonce = "0".repeat(64);
        f.write_context();
        assert!(s.validated.revalidate().is_err());
        assert!(!f.root.join("qa/ui-boot-1.json").exists());
    }
    #[test]
    fn probe_injects_only_public_context_at_exactly_one_placeholder() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        let script = v
            .probe_script(
                "const ctx=__CUR160_SMOKE_CONTEXT__;",
                "current_engine_id",
                "http://127.0.0.1:1234",
            )
            .unwrap();
        assert!(script.contains("\"origin\":\"http://127.0.0.1:1234\""));
        assert!(!script.contains("candidateSha"));
        assert!(v
            .probe_script(
                "__CUR160_SMOKE_CONTEXT__",
                "current_engine_id",
                "https://foreign.test"
            )
            .is_err());
        assert!(v
            .probe_script("none", "current_engine_id", "http://127.0.0.1:1234")
            .is_err());
        let integrated = v
            .probe_script(
                include_str!("../../tools/installed-smoke-bridge.js"),
                "current_engine_id",
                "http://127.0.0.1:1234",
            )
            .unwrap();
        assert!(!integrated.contains("__CUR160_SMOKE_CONTEXT__"));
        assert!(integrated.contains("installed_smoke_report"));
    }
    #[test]
    fn hardlinks_and_special_product_state_are_rejected() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        fs::hard_link(
            f.root.join("qa/initial.json"),
            f.root.join("qa/linked.json"),
        )
        .unwrap();
        assert!(v.revalidate().is_err());
        fs::remove_file(f.root.join("qa/linked.json")).unwrap();
        fs::create_dir_all(f.root.join("data/profiles")).unwrap();
        assert!(v.revalidate().is_err());
    }
    #[test]
    fn only_closed_empty_account_state_and_empty_profile_registry_are_allowed() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        fs::create_dir_all(f.root.join("data/identity")).unwrap();
        let accounts = f.root.join("data/identity/product-accounts.json");
        fs::write(&accounts, r#"{"version":1,"active":null,"users":[]}"#).unwrap();
        v.revalidate().unwrap();
        for bad in [
            r#"{"version":1,"active":null,"users":[],"owner":null}"#,
            r#"{"version":1,"active":"user","users":[]}"#,
            r#"{"version":1,"active":null,"users":[null]}"#,
            r#"{"version":1,"users":[]}"#,
            r#"{"version":1,"active":null,"users":[],"version":1}"#,
        ] {
            fs::write(&accounts, bad).unwrap();
            assert!(v.revalidate().is_err());
        }
        fs::remove_file(accounts).unwrap();
        fs::write(
            f.root.join("data/product-accounts.json"),
            r#"{"version":1,"active":null,"users":[]}"#,
        )
        .unwrap();
        fs::write(
            f.root.join("data/account-profiles.json"),
            r#"{"version":1,"legacyOwner":null,"legacyStores":null,"backup":null}"#,
        )
        .unwrap();
        v.revalidate().unwrap();
        fs::write(
            f.root.join("data/identity/google-identity-session.json"),
            "{}",
        )
        .unwrap();
        assert!(v.revalidate().is_err());
    }
    #[test]
    fn two_boot_ui_receipts_require_new_engine_and_reuse_only_the_digest() {
        let mut f = Fixture::new();
        let first = f.preflight().unwrap();
        f.vault(1, "first_engine_identity", 42);
        let session = Session {
            validated: first,
            engine_instance: "first_engine_identity".into(),
            engine_pid: 42,
            origin: "http://127.0.0.1:1234".into(),
            reported: false.into(),
        };
        session
            .accept_ui("main", &session.origin, &report(&f.context.nonce))
            .unwrap();
        session.record_shutdown().unwrap();
        f.context.boot = 2;
        f.context.nonce = "d".repeat(64);
        f.write_context();
        let second = f.preflight().unwrap();
        f.vault(2, "second_engine_identity", 43);
        let session = Session {
            validated: second,
            engine_instance: "second_engine_identity".into(),
            engine_pid: 43,
            origin: "http://127.0.0.1:1235".into(),
            reported: false.into(),
        };
        session
            .accept_ui("main", &session.origin, &report(&f.context.nonce))
            .unwrap();
        session.record_shutdown().unwrap();
        assert!(f.root.join("qa/ui-boot-2.json").is_file());
        assert!(f.root.join("qa/shutdown-boot-2.json").is_file());
    }
    #[test]
    fn child_environment_whitelist_excludes_credentials_and_ci_grants() {
        for key in [
            "PATH",
            "SystemRoot",
            "HOME",
            "DBUS_SESSION_BUS_ADDRESS",
            "LC_ALL",
        ] {
            assert!(os_environment_key(key));
        }
        for key in [
            "BIANK_WEB_TOKEN",
            "BIANK_CORE_READ_SSH_KEY",
            "OPENAI_API_KEY",
            "ACTIONS_RUNTIME_TOKEN",
            "GITHUB_TOKEN",
            "NODE_OPTIONS",
            "CODEX_HOME",
        ] {
            assert!(!os_environment_key(key));
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_rejected() {
        let f = Fixture::new();
        let v = f.preflight().unwrap();
        std::os::unix::fs::symlink(std::env::temp_dir(), f.root.join("escape")).unwrap();
        assert!(v.revalidate().is_err());
    }
}
impl Session {
    pub fn accept_ui(&self, label: &str, origin: &str, report: &UiReport) -> Result<()> {
        if label != "main"
            || origin != self.origin
            || report.nonce != self.validated.context.nonce
            || !(report.health
                && report.login
                && report.account_required
                && report.unauthenticated
                && report.bridge)
        {
            return Err("CUR160_UI_REJECTED".into());
        }
        self.validated.revalidate()?;
        let vault = self.validated.read_vault(self.validated.context.boot)?;
        if vault.nonce != report.nonce
            || vault.engine_pid != u64::from(self.engine_pid)
            || vault.engine_instance_id != self.engine_instance
        {
            return Err("CUR160_VAULT_RECEIPT_INVALID".into());
        }
        if self.validated.context.boot == 2 {
            let first = self.validated.read_vault(1)?;
            if first.digest_sha256 != vault.digest_sha256
                || first.engine_instance_id == vault.engine_instance_id
                || first.engine_pid == vault.engine_pid
                || first.nonce == vault.nonce
            {
                return Err("CUR160_VAULT_RECEIPT_INVALID".into());
            }
        }
        if self
            .reported
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err("CUR160_UI_REPLAY".into());
        }
        let mut value = self
            .validated
            .identity(&self.engine_instance, self.engine_pid, "ui");
        for field in [
            "health",
            "login",
            "accountRequired",
            "unauthenticated",
            "bridge",
        ] {
            value[field] = true.into();
        }
        let result = self.validated.publish("ui", value);
        if result.is_err() {
            self.reported
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
        result
    }
    pub fn record_shutdown(&self) -> Result<()> {
        if !self.reported.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("CUR160_UI_MISSING".into());
        }
        let mut value = self
            .validated
            .identity(&self.engine_instance, self.engine_pid, "shutdown");
        value["drained"] = true.into();
        value["engineExited"] = true.into();
        self.validated.publish("shutdown", value)
    }
}

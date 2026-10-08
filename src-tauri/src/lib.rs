pub mod engine;
mod imports;
mod resources;
pub mod transport;
mod updates;
pub mod vault;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;
struct Boot {
    preparing: AtomicBool,
}
fn shell_log(message: &str) {
    use std::io::Write;
    eprintln!("{message}");
    // GUI-subsystem Windows binaries have no console. Keep startup evidence in
    // the installation's own log; callers pass status text, never credentials.
    if let Ok((root, _)) = data_root() {
        let directory = root.join("logs");
        if std::fs::create_dir_all(&directory).is_ok() {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("shell.log"))
            {
                let _ = writeln!(file, "{message}");
            }
        }
    }
}
struct Desktop {
    connection: engine::Connection,
    _child: Mutex<Option<std::process::Child>>,
    exiting: AtomicBool,
    zoom: Mutex<f64>,
}
impl Desktop {
    async fn shutdown(&self) -> Result<(), String> {
        self.connection.drain().await?;
        for _ in 0..150 {
            let exited = {
                let mut child = self
                    ._child
                    .lock()
                    .map_err(|_| "No se pudo observar el motor")?;
                match child.as_mut() {
                    Some(process) => process
                        .try_wait()
                        .map_err(|_| "No se pudo observar el motor")?
                        .is_some(),
                    None => true,
                }
            };
            if exited {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err("El motor todavía está cerrando; no se reemplazó el runtime.".into())
    }
}
#[tauri::command]
fn get_desktop_info() -> Value {
    json!({"app":"Biank Desktop","edition":"Tauri v2","version":env!("CARGO_PKG_VERSION")})
}
#[tauri::command]
fn qa_report(state: Value) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("QA sólo está disponible en el build de desarrollo".into());
    }
    let root = std::env::var_os("BIANK_TAURI_TEST_ROOT")
        .map(PathBuf::from)
        .ok_or("QA requiere un almacén aislado")?;
    if !root.is_absolute() {
        return Err("Almacén QA inválido".into());
    }
    let evidence = json!({"authBrowser":state["authBrowser"].as_bool().unwrap_or(false),"bridge":state["bridge"].as_bool().unwrap_or(false),"health":state["health"].as_bool().unwrap_or(false),"login":state["login"].as_bool().unwrap_or(false)});
    std::fs::write(root.join("qa-ui.json"), evidence.to_string())
        .map_err(|_| "No se pudo guardar la evidencia QA".into())
}
#[tauri::command]
fn focus_main(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Biank está iniciando")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let url: tauri::Url = url.parse().map_err(|_| "Dirección inválida")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Dirección externa no permitida".into());
    }
    #[allow(deprecated)]
    app.shell()
        .open(url.as_str(), None)
        .map_err(|_| "No se pudo abrir el navegador del sistema".into())
}
#[tauri::command]
async fn browser_action(app: AppHandle, payload: Value) -> Result<Value, String> {
    if payload["action"] != "open" {
        return Err("Acción del navegador no admitida".into());
    }
    let state = app.state::<Desktop>();
    state
        .connection
        .request(
            "/api/agent-browser/open",
            Some(json!({"provider":payload["provider"],"id":payload["id"]})),
        )
        .await
}
#[tauri::command]
fn zoom_view(app: AppHandle, action: String) -> Result<f64, String> {
    let state = app.state::<Desktop>();
    let mut zoom = state.zoom.lock().map_err(|_| "Zoom no disponible")?;
    *zoom = match action.as_str() {
        "in" => (*zoom + 0.1).min(2.0),
        "out" => (*zoom - 0.1).max(0.5),
        "reset" => 1.0,
        _ => return Err("Zoom inválido".into()),
    };
    app.get_webview_window("main")
        .ok_or("Biank está iniciando")?
        .set_zoom(*zoom)
        .map_err(|e| e.to_string())?;
    Ok(*zoom)
}
#[tauri::command]
fn show_about(app: AppHandle) {
    app.dialog()
        .message(format!(
            "Biank Desktop {}\nTauri v2 · CB Inteligencia",
            env!("CARGO_PKG_VERSION")
        ))
        .title("Acerca de Biank")
        .show(|_| {});
}
#[tauri::command]
fn set_theme(app: AppHandle, theme: String) -> Result<(), String> {
    let theme = match theme.as_str() {
        "dark" => tauri::Theme::Dark,
        "light" => tauri::Theme::Light,
        _ => return Err("Tema inválido".into()),
    };
    app.get_webview_window("main")
        .ok_or("Biank está iniciando")?
        .set_theme(Some(theme))
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn request_exit(app: AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<Desktop>() else {
        if !app.state::<Boot>().preparing.load(Ordering::SeqCst) {
            app.exit(0);
            return Ok(());
        }
        return Err("Biank sigue preparando el motor; espera a que termine antes de salir.".into());
    };
    if state.exiting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    if let Err(error) = state.shutdown().await {
        state.exiting.store(false, Ordering::SeqCst);
        return Err(error);
    }
    // Only normal exit may apply a previously verified, downloaded update.
    if let Err(error) = updates::install_ready(&app) {
        state.exiting.store(false, Ordering::SeqCst);
        return Err(error);
    }
    app.exit(0);
    Ok(())
}
fn data_root() -> Result<(PathBuf, bool), String> {
    if cfg!(debug_assertions) {
        if let Some(root) = std::env::var_os("BIANK_TAURI_TEST_ROOT") {
            let root = PathBuf::from(root);
            if !root.is_absolute() {
                return Err("El almacén de QA debe ser absoluto".into());
            }
            return Ok((root, true));
        }
    }
    if let Some(root) = std::env::var_os("BIANK_DATA_ROOT") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err("El almacén de Biank debe ser absoluto".into());
        }
        return Ok((root, false));
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or("LOCALAPPDATA no disponible")?
    } else {
        dirs::config_dir()
            .ok_or("No se pudo resolver el almacén existente")?
            .join("Biank")
    };
    Ok((base.join("Biank"), false))
}
async fn start(app: AppHandle) -> Result<(), String> {
    shell_log("biank-shell: verificando runtime");
    let (root, development) = data_root()?;
    let resources = if cfg!(debug_assertions) {
        match std::env::var_os("BIANK_TAURI_TEST_RESOURCE_ROOT") {
            Some(path)
                if std::env::var_os("BIANK_DATA_ROOT").is_some()
                    && PathBuf::from(&path).is_absolute() =>
            {
                PathBuf::from(path)
            }
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        }
    } else {
        app.path()
            .resource_dir()
            .map_err(|_| "Recursos no disponibles")?
    };
    // Tauri canonicalizes the installed executable on Windows (\\?\ paths).
    // Keep a compatible equivalent when crossing into Node and its URL/path APIs.
    let resources = dunce::simplified(&resources).to_path_buf();
    resources::verify(&resources.join("runtime"))?;
    shell_log("biank-shell: runtime verificado");
    let key = vault::open(&root, development)?;
    shell_log("biank-shell: llave disponible");
    let supervisor = engine::Supervisor::start(
        engine::Config {
            root,
            resources,
            version: env!("CARGO_PKG_VERSION").into(),
            development,
        },
        key,
    )
    .await?;
    shell_log("biank-shell: motor verificado");
    let connection = supervisor.connection.clone();
    app.manage(Desktop {
        connection: supervisor.connection,
        _child: Mutex::new(Some(supervisor.child)),
        exiting: AtomicBool::new(false),
        zoom: Mutex::new(1.0),
    });
    let transport = transport::Transport::start(connection).await?;
    let origin = transport.origin.clone();
    let url: tauri::Url = format!("{origin}/")
        .parse()
        .map_err(|_| "Transporte inválido")?;
    let capability = json!({"identifier":"local-runtime","windows":["main"],"remote":{"urls":[format!("{origin}/*")]},"permissions":["core:default","allow-get-desktop-info","allow-qa-report","allow-show-about","allow-set-theme","allow-focus-main","allow-open-external","allow-browser-action","allow-zoom-view","allow-request-exit","allow-import-select","allow-import-apply","allow-update-state","allow-check-update","allow-apply-update"]});
    app.add_capability(capability.to_string())
        .map_err(|_| "No se pudo restringir el bridge local")?;
    let handle = app.clone();
    let mut bridge = include_str!("../../tools/bridge.js").to_string();
    if development {
        bridge.push_str(include_str!("../../tools/qa-bridge.js"));
    }
    app.run_on_main_thread(move || {
        let result = (|| -> tauri::Result<()> {
            let allowed_origin = origin.clone();
            let window =
                WebviewWindowBuilder::new(&handle, "main", WebviewUrl::App("startup.html".into()))
                    .title("Biank Desktop")
                    .inner_size(1280.0, 850.0)
                    .min_inner_size(960.0, 640.0)
                    .initialization_script(&bridge)
                    .on_navigation(move |target| {
                        target.scheme() == "tauri"
                            || target.host_str() == Some("tauri.localhost")
                            || target.origin().ascii_serialization() == allowed_origin
                    })
                    .build()?;
            let cookie = tauri::webview::Cookie::build((transport::COOKIE, transport.capability))
                .domain("127.0.0.1")
                .path("/")
                .http_only(true)
                .same_site(cookie::SameSite::Strict)
                .build();
            window.set_cookie(cookie)?;
            window.navigate(url)?;
            shell_log("biank-shell: ventana principal lista");
            let app = handle.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            });
            if let Some(startup) = handle.get_webview_window("startup") {
                startup.close()?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            shell_log("biank-shell: no se pudo crear la ventana principal");
            let _ = handle.emit("biank-startup-error", error.to_string());
        }
    })
    .map_err(|_| "No se pudo crear la ventana")?;
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = focus_main(app.clone());
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Boot {
            preparing: AtomicBool::new(true),
        })
        .manage(updates::Updates::default())
        .manage(imports::Imports::default())
        .setup(|app| {
            shell_log("biank-shell: configurando bandeja");
            let show = MenuItem::with_id(app, "show", "Abrir Biank", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Salir de Biank", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Biank Desktop")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = focus_main(app.clone());
                    }
                    "quit" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = request_exit(app.clone()).await {
                                let _ = app.emit("biank-shell-error", error);
                                let _ = focus_main(app);
                            }
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            shell_log("biank-shell: bandeja lista");
            let updater = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let _ = updates::check_update(updater.clone()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(21600)).await;
                }
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result = start(handle.clone()).await;
                handle
                    .state::<Boot>()
                    .preparing
                    .store(false, Ordering::SeqCst);
                if let Err(error) = result {
                    shell_log(&format!("biank-shell: {error}"));
                    if let Some(window) = handle.get_webview_window("startup") {
                        let script = format!(
                            "document.getElementById('status').textContent={}",
                            serde_json::to_string(&error).unwrap()
                        );
                        let _ = window.eval(&script);
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_desktop_info,
            qa_report,
            show_about,
            set_theme,
            focus_main,
            open_external,
            browser_action,
            zoom_view,
            request_exit,
            imports::import_select,
            imports::import_apply,
            updates::update_state,
            updates::check_update,
            updates::apply_update
        ])
        .build(tauri::generate_context!())
        .expect("No se pudo iniciar Biank Desktop")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = request_exit(handle.clone()).await {
                        let _ = handle.emit("biank-shell-error", error);
                    }
                });
            }
        });
}

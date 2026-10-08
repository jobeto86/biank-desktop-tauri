use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
pub struct Updates {
    busy: AtomicBool,
    state: Mutex<Value>,
    ready: Mutex<Option<(Update, Vec<u8>)>>,
}
impl Default for Updates {
    fn default() -> Self {
        Self {
            busy: AtomicBool::new(false),
            state: Mutex::new(
                json!({"phase":"idle","detail":"Actualizaciones pendientes de comprobar.","currentVersion":env!("CARGO_PKG_VERSION"),"availableVersion":null,"platform":std::env::consts::OS,"action":null}),
            ),
            ready: Mutex::new(None),
        }
    }
}
fn publish(app: &AppHandle, phase: &str, detail: &str, version: Option<String>) {
    let state = app.state::<Updates>();
    let value = json!({"phase":phase,"detail":detail,"currentVersion":env!("CARGO_PKG_VERSION"),"availableVersion":version,"platform":std::env::consts::OS,"action":if phase=="ready"{Some("install")}else{None}});
    *state.state.lock().unwrap() = value.clone();
    let _ = app.emit("biank-updates:state", value);
}
#[tauri::command]
pub fn update_state(app: AppHandle) -> Value {
    app.state::<Updates>().state.lock().unwrap().clone()
}
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Value, String> {
    let state = app.state::<Updates>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(update_state(app.clone()));
    }
    if state.ready.lock().unwrap().is_some() {
        state.busy.store(false, Ordering::SeqCst);
        return Ok(update_state(app.clone()));
    }
    publish(&app, "checking", "Buscando actualizaciones…", None);
    let result = async {
        let config = app.config().plugins.0.get("updater");
        if config
            .and_then(|v| v.get("pubkey"))
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .is_none()
        {
            return Err("El canal firmado de actualización no está configurado");
        }
        let updater = app
            .updater()
            .map_err(|_| "El canal firmado de actualización no está configurado")?;
        if let Some(update) = updater
            .check()
            .await
            .map_err(|_| "No se pudo verificar el canal de actualización")?
        {
            let version = update.version.clone();
            publish(
                &app,
                "downloading",
                "Descargando actualización firmada…",
                Some(version.clone()),
            );
            let bytes = update
                .download(|_, _| {}, || {})
                .await
                .map_err(|_| "La descarga no superó la verificación de firma")?;
            *state.ready.lock().unwrap() = Some((update, bytes));
            publish(
                &app,
                "ready",
                "La actualización se instalará después de terminar el trabajo activo.",
                Some(version),
            );
        } else {
            publish(&app, "idle", "Biank está actualizado.", None);
        }
        Ok::<(), &str>(())
    }
    .await;
    state.busy.store(false, Ordering::SeqCst);
    if let Err(error) = result {
        publish(&app, "error", error, None);
        return Err(error.into());
    }
    Ok(update_state(app.clone()))
}
pub fn install_ready(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Updates>();
    let ready = state.ready.lock().unwrap();
    if let Some((update, bytes)) = ready.as_ref() {
        update
            .install(bytes)
            .map_err(|_| "No se pudo instalar la actualización verificada")?;
    }
    Ok(())
}
#[tauri::command]
pub async fn apply_update(app: AppHandle) -> Result<(), String> {
    if app.state::<Updates>().ready.lock().unwrap().is_none() {
        return Err("No hay actualización verificada lista".into());
    }
    let state = app.state::<super::Desktop>();
    state.shutdown().await?;
    install_ready(&app)?;
    app.restart();
}

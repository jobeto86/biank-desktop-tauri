use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
#[derive(Default)]
struct Selection {
    root: String,
    transcripts: Vec<String>,
    plan: String,
}
#[derive(Default)]
pub struct Imports {
    busy: AtomicBool,
    selection: Mutex<Selection>,
}
struct Guard<'a>(&'a AtomicBool);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
#[tauri::command]
pub async fn import_select(app: AppHandle, kind: String) -> Result<Option<Value>, String> {
    if !matches!(
        kind.as_str(),
        "root" | "transcript-file" | "transcript-folder"
    ) {
        return Err("Selección inválida".into());
    }
    let state = app.state::<Imports>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Hay una importación en curso".into());
    }
    let _guard = Guard(&state.busy);
    if kind != "root" && state.selection.lock().unwrap().root.is_empty() {
        return Err("Primero selecciona la carpeta de Biank antigua".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let picker = app.dialog().file().set_title("Importar Biank");
    if kind == "transcript-file" {
        picker.pick_file(move |path| {
            let _ = tx.send(path);
        });
    } else {
        picker.pick_folder(move |path| {
            let _ = tx.send(path);
        });
    }
    let Some(path) = rx.await.map_err(|_| "No se pudo seleccionar el origen")? else {
        return Ok(None);
    };
    let selected = path
        .into_path()
        .map_err(|_| "Origen no local")?
        .to_string_lossy()
        .to_string();
    let (root, transcripts) = {
        let prior = state.selection.lock().unwrap();
        if kind == "root" {
            (selected, Vec::new())
        } else {
            let mut paths = prior.transcripts.clone();
            if !paths.contains(&selected) {
                paths.push(selected);
            }
            (prior.root.clone(), paths)
        }
    };
    let preview = app
        .state::<super::Desktop>()
        .connection
        .request(
            "/api/legacy-import/preview",
            Some(json!({"root":root,"transcripts":transcripts})),
        )
        .await?;
    *state.selection.lock().unwrap() = Selection {
        root,
        transcripts,
        plan: preview["id"].as_str().unwrap_or_default().into(),
    };
    Ok(Some(preview))
}
#[tauri::command]
pub async fn import_apply(app: AppHandle, id: String) -> Result<Value, String> {
    let state = app.state::<Imports>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Hay una importación en curso".into());
    }
    let _guard = Guard(&state.busy);
    if id.is_empty() || state.selection.lock().unwrap().plan != id {
        return Err("Revisa de nuevo la carpeta antes de importar".into());
    }
    let result = app
        .state::<super::Desktop>()
        .connection
        .request("/api/legacy-import/apply", Some(json!({"id":id})))
        .await;
    state.selection.lock().unwrap().plan.clear();
    result
}

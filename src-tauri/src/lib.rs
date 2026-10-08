pub mod engine;
pub mod vault;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

#[tauri::command]
fn toggle_companion(app: AppHandle) -> Result<bool, String> {
    if let Some(companion) = app.get_webview_window("companion") {
        let is_visible = companion.is_visible().map_err(|e| e.to_string())?;
        if is_visible {
            companion.hide().map_err(|e| e.to_string())?;
            Ok(false)
        } else {
            companion.show().map_err(|e| e.to_string())?;
            companion.set_focus().map_err(|e| e.to_string())?;
            Ok(true)
        }
    } else {
        Err("Ventana companion no encontrada".to_string())
    }
}

#[tauri::command]
fn get_desktop_info() -> serde_json::Value {
    serde_json::json!({
        "app": "Biank Desktop",
        "edition": "Tauri v2 (Rust)",
        "version": env!("CARGO_PKG_VERSION"),
        "lightweight": true
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Configuración del menú de la bandeja del sistema (Tray)
            let quit_i = MenuItem::with_id(app, "quit", "Salir de Biank", true, None::<&str>)?;
            let toggle_companion_i = MenuItem::with_id(app, "toggle_companion", "Mascota Marina (Mostrar/Ocultar)", true, None::<&str>)?;
            let show_main_i = MenuItem::with_id(app, "show_main", "Abrir Biank", true, None::<&str>)?;

            let tray_menu = Menu::with_items(app, &[&show_main_i, &toggle_companion_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Biank Desktop")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show_main" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle_companion" => {
                        let _ = toggle_companion(app.clone());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![toggle_companion, get_desktop_info])
        .run(tauri::generate_context!())
        .expect("error while running Biank Desktop application");
}

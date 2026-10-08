fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_desktop_info",
            "qa_report",
            "show_about",
            "set_theme",
            "focus_main",
            "open_external",
            "browser_action",
            "zoom_view",
            "request_exit",
            "import_select",
            "import_apply",
            "update_state",
            "check_update",
            "apply_update",
        ]),
    ))
    .expect("No se pudo generar el ACL");
}

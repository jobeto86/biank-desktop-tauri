// QA only: download the current published release through the real updater.
// It verifies signature/version and delivery bytes, and never installs anything.
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri_plugin_updater::UpdaterExt;

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "Usage: validate_update_feed TARGET PUBLIC_KEY_FILE SHA256"
    );
    let target = args[1].clone();
    assert!(matches!(
        target.as_str(),
        "windows-x86_64" | "darwin-aarch64"
    ));
    let public_key = fs::read_to_string(&args[2]).expect("Public updater identity required");
    let expected_hash = args[3].clone();
    assert!(expected_hash.len() == 64 && expected_hash.bytes().all(|b| b.is_ascii_hexdigit()));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().plugins.0.insert("updater".into(), serde_json::json!({
        "pubkey": public_key.trim(), "requireSignedVersion": true,
        "endpoints": ["https://github.com/jobeto86/biank-desktop/releases/latest/download/latest.json"]
    }));
    let result = Arc::new(Mutex::new(None));
    let output = result.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let checked = async {
                    // QA deliberately downloads the current version, so it can
                    // check production delivery without publishing a fake upgrade.
                    let update = handle.updater_builder().target(&target)
                        .timeout(Duration::from_secs(180))
                        .version_comparator(|_, _| true).build()
                        .map_err(|e| e.to_string())?.check().await
                        .map_err(|e| e.to_string())?.ok_or("Published release missing")?;
                    if update.version != env!("CARGO_PKG_VERSION") {
                        return Err("Published version differs from the validated release".into());
                    }
                    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
                    if format!("{:x}", Sha256::digest(&bytes)) != expected_hash {
                        return Err("Published bytes differ from the verified artifact".into());
                    }
                    Ok::<_, String>(format!("Native updater manifest/download/signature/version/SHA256: PASS ({target})"))
                }.await;
                // Tauri may terminate the process from its event loop rather
                // than return from app.run. Emit evidence and status first.
                let code = if checked.is_ok() { 0 } else { 1 };
                match &checked {
                    Ok(message) => println!("{message}"),
                    Err(error) => eprintln!("Updater QA failed: {error}"),
                }
                *output.lock().unwrap() = Some(checked);
                handle.exit(code);
            });
            Ok(())
        }).build(context).expect("QA updater application failed to initialize");
    app.run(|_, _| {});
    match result
        .lock()
        .unwrap()
        .take()
        .expect("Updater QA did not finish")
    {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("Updater QA failed: {error}");
            std::process::exit(1);
        }
    };
}

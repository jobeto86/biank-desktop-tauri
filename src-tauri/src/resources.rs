use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};
/// The manifest is bound to the compiled shell, not trusted from mutable disk.
pub fn verify(runtime: &Path) -> Result<(), String> {
    let manifest: Value = serde_json::from_str(include_str!("../runtime/resources-manifest.json"))
        .map_err(|_| "Manifest del runtime inválido")?;
    verify_manifest(runtime, &manifest)
}
fn verify_manifest(runtime: &Path, manifest: &Value) -> Result<(), String> {
    let root = runtime.canonicalize().map_err(|_| "Runtime ausente")?;
    let files = manifest["files"]
        .as_object()
        .ok_or("Manifest del runtime inválido")?;
    for (relative, metadata) in files {
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("Manifest con ruta inválida".into());
        }
        let file = runtime.join(relative);
        let actual = file
            .canonicalize()
            .map_err(|_| "Recurso del runtime ausente")?;
        if !actual.starts_with(&root) {
            return Err("Recurso redirigido fuera del runtime".into());
        }
        if let Some(expected) = metadata["link"].as_str() {
            if fs::read_link(&file)
                .map_err(|_| "Enlace del runtime modificado")?
                .to_string_lossy()
                != expected
            {
                return Err("Enlace del runtime modificado".into());
            }
        } else {
            if fs::symlink_metadata(&file)
                .map_err(|_| "Recurso ausente")?
                .file_type()
                .is_symlink()
            {
                return Err("Recurso del runtime redirigido".into());
            }
            let mut input = fs::File::open(&file).map_err(|_| "Recurso del runtime no legible")?;
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65536];
            loop {
                let count = input
                    .read(&mut buffer)
                    .map_err(|_| "Recurso del runtime no legible")?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            if metadata["sha256"].as_str() != Some(format!("{:x}", hash.finalize()).as_str()) {
                return Err(
                    "El runtime empaquetado no supera la verificación de integridad".into(),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_tampered_resources_and_traversal() {
        let root =
            std::env::temp_dir().join(format!("biank-integrity-{}", crate::engine::random_id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("fixture"), b"original").unwrap();
        let manifest = serde_json::json!({"files":{"fixture":{"sha256":format!("{:x}",Sha256::digest(b"original"))}}});
        assert!(verify_manifest(&root, &manifest).is_ok());
        fs::write(root.join("fixture"), b"modified").unwrap();
        assert!(verify_manifest(&root, &manifest).is_err());
        assert!(verify_manifest(&root, &serde_json::json!({"files":{"../escape":{}}})).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

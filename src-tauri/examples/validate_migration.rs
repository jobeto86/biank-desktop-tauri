use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};
use zeroize::Zeroize;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::args().nth(1).ok_or("synthetic root required")?);
    if !root.is_absolute() || !root.join("migration-proof.json").exists() {
        return Err("synthetic fixture required".into());
    }
    let sealed = fs::read(root.join("secrets/vault.key.sealed"))?;
    let history = fs::read(root.join("data/history-preserved.txt"))?;
    let proof: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("migration-proof.json"))?)?;
    let mut key = biank_desktop_lib::vault::open(&root, false)?.ok_or("migration key absent")?;
    let nonce = STANDARD.decode(proof["nonce"].as_str().ok_or("nonce absent")?)?;
    let encrypted = STANDARD.decode(proof["encrypted"].as_str().ok_or("ciphertext absent")?)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "invalid key")?;
    let mut plain = cipher
        .decrypt(Nonce::from_slice(&nonce), encrypted.as_slice())
        .map_err(|_| "migration lost vault access")?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&plain)),
        proof["expected"].as_str().unwrap()
    );
    key.zeroize();
    plain.zeroize();
    assert_eq!(fs::read(root.join("secrets/vault.key.sealed"))?, sealed);
    assert_eq!(fs::read(root.join("data/history-preserved.txt"))?, history);
    // A second open proves the native credential store persists the migrated key.
    let mut reopened = biank_desktop_lib::vault::open(&root, false)?.ok_or("reopen key absent")?;
    let cipher = Aes256Gcm::new_from_slice(&reopened).map_err(|_| "invalid persisted key")?;
    let mut plain = cipher
        .decrypt(Nonce::from_slice(&nonce), encrypted.as_slice())
        .map_err(|_| "persisted key changed")?;
    reopened.zeroize();
    plain.zeroize();
    println!("Electron → native vault, encrypted data, reopen and rollback material: PASS");
    Ok(())
}

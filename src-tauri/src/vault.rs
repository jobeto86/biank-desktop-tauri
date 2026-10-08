//! Master key stays in the OS credential store and crosses only the child's stdin.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use zeroize::Zeroize;

fn decode(raw: &[u8]) -> Result<Vec<u8>, String> {
    let bytes = if raw.len() == 32 {
        raw.to_vec()
    } else {
        URL_SAFE_NO_PAD
            .decode(String::from_utf8_lossy(raw).trim())
            .map_err(|_| "Llave existente inválida")?
    };
    if bytes.len() != 32 {
        return Err("Llave existente inválida; se conservan los datos".into());
    }
    Ok(bytes)
}
pub fn open(root: &Path, development: bool) -> Result<Option<Vec<u8>>, String> {
    if std::env::var_os("BIANK_DESKTOP_VAULT_KEY_FILE").is_some() {
        return Ok(None);
    }
    // Isolated development lets the existing coordinator use its own synthetic key file.
    if development {
        return Ok(None);
    }
    let account = format!(
        "installation-{:x}",
        Sha256::digest(root.to_string_lossy().as_bytes())
    );
    let entry = keyring::Entry::new("Biank Desktop Vault", &account)
        .map_err(|_| "Llavero del sistema no disponible")?;
    match entry.get_secret() {
        Ok(key) => return decode(&key).map(Some),
        Err(keyring::Error::NoEntry) => {}
        Err(_) => {
            return Err("No se pudo abrir el llavero del sistema. No se creó otra llave.".into())
        }
    }
    let sealed = root.join("secrets/vault.key.sealed");
    let config = std::env::var_os("BIANK_DESKTOP_CONFIG_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"))
                .join("biank-desktop")
        });
    let candidates = [
        config.join("vault/master.key"),
        root.join("secrets/vault.key"),
        root.join("data/secrets/vault.key"),
    ];
    let mut key = if sealed.is_file() {
        decode(&unseal_electron(
            &fs::read(&sealed).map_err(|_| "No se pudo leer la llave sellada")?,
        )?)?
    } else if let Some(file) = candidates.iter().find(|p| p.is_file()) {
        if fs::symlink_metadata(file)
            .map_err(|_| "Llave existente no legible")?
            .file_type()
            .is_symlink()
        {
            return Err("La llave existente no es un archivo regular".into());
        }
        decode(&fs::read(file).map_err(|_| "Llave existente no legible")?)?
    } else {
        // Existing encrypted stores must never receive a newly generated key.
        if root.join("data/vault.db").exists()
            || root.join("data/identity/vault.db").exists()
            || root.join("vault.db").exists()
            || root.join("data/account-profiles.json").exists()
        {
            return Err("Existe una bóveda sin llave accesible. Se conservan los datos.".into());
        }
        let mut bytes = vec![0; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    };
    if entry.set_secret(&key).is_err() {
        key.zeroize();
        return Err("No se pudo sellar la llave en el llavero del sistema".into());
    }
    let mut check = entry
        .get_secret()
        .map_err(|_| "No se pudo verificar la llave sellada")?;
    let valid = check == key;
    check.zeroize();
    if !valid {
        key.zeroize();
        return Err("El llavero no conserva la llave original".into());
    }
    // Keep old encrypted material for rollback; no destructive migration here.
    Ok(Some(key))
}
#[cfg(windows)]
fn unseal_electron(raw: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    if raw.starts_with(b"v10") {
        if raw.len() < 3 + 12 + 16 {
            return Err("Llave Electron truncada; se conservan los datos".into());
        }
        let user_data = std::env::var_os("BIANK_ELECTRON_USER_DATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("Biank")))
            .filter(|p| p.is_absolute())
            .ok_or("No se encontró el perfil original de Electron")?;
        let state: serde_json::Value = serde_json::from_slice(
            &fs::read(user_data.join("Local State"))
                .map_err(|_| "No se pudo leer Local State de Electron")?,
        )
        .map_err(|_| "Local State de Electron inválido")?;
        let wrapped = base64::engine::general_purpose::STANDARD
            .decode(
                state["os_crypt"]["encrypted_key"]
                    .as_str()
                    .ok_or("Falta la llave original de Electron")?,
            )
            .map_err(|_| "Llave original de Electron inválida")?;
        let wrapped = wrapped
            .strip_prefix(b"DPAPI")
            .ok_or("Formato original de Electron desconocido")?;
        let mut key = unprotect_dpapi(wrapped)?;
        let result = (|| {
            let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Llave Electron inválida")?;
            cipher
                .decrypt(Nonce::from_slice(&raw[3..15]), &raw[15..])
                .map_err(|_| {
                    "No se pudo autenticar la llave Electron; se conservan los datos".into()
                })
        })();
        key.zeroize();
        return result;
    }
    unprotect_dpapi(raw.strip_prefix(b"DPAPI").unwrap_or(raw))
}
#[cfg(windows)]
fn unprotect_dpapi(raw: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: raw.len() as u32,
        pbData: raw.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    unsafe {
        if CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        ) == 0
        {
            return Err("No se pudo abrir la llave Electron con DPAPI".into());
        }
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        Ok(bytes)
    }
}
#[cfg(target_os = "macos")]
fn unseal_electron(raw: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
    if !raw.starts_with(b"v10") {
        return Err("Formato de llave Electron desconocido; se conserva".into());
    }
    // Electron uses SecItemCopyMatching across the user's Keychain search list.
    // Do not constrain the legacy lookup to one default keychain/domain.
    let mut password =
        security_framework::passwords::get_generic_password("Biank Safe Storage", "Biank")
            .map_err(|error| {
                format!(
                    "No se pudo recuperar la identidad Electron de Keychain (OSStatus {})",
                    error.code()
                )
            })?;
    let mut key = [0; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(&password, b"saltysalt", 1003, &mut key);
    password.zeroize();
    let result = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &[b' '; 16].into())
        .decrypt_padded_vec_mut::<Pkcs7>(&raw[3..])
        .map_err(|_| "No se pudo abrir la llave Electron");
    key.zeroize();
    result.map_err(Into::into)
}
#[cfg(not(any(windows, target_os = "macos")))]
fn unseal_electron(_raw: &[u8]) -> Result<Vec<u8>, String> {
    Err("La llave sellada existente requiere recuperación del llavero original".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_length_and_preserves_raw_and_encoded_keys() {
        assert!(decode(b"placeholder").is_err());
        let key = vec![7; 32];
        assert_eq!(decode(&key).unwrap(), key);
        assert_eq!(
            decode(URL_SAFE_NO_PAD.encode(&key).as_bytes()).unwrap(),
            key
        );
    }
}

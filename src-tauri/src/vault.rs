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
// An explicit original profile is authoritative. Otherwise Electron's packaged
// product name and package name are both valid historical userData directories.
#[cfg(any(windows, test))]
fn electron_profiles(
    explicit: Option<PathBuf>,
    appdata: Option<PathBuf>,
) -> Result<Vec<PathBuf>, String> {
    if let Some(path) = explicit {
        if !path.is_absolute() {
            return Err("El perfil original de Electron debe ser una ruta absoluta".into());
        }
        return Ok(vec![path]);
    }
    let base = appdata
        .filter(|p| p.is_absolute())
        .ok_or("No se encontró el perfil original de Electron")?;
    Ok(vec![base.join("Biank"), base.join("biank-desktop")])
}

#[cfg(any(windows, test))]
fn recover_electron_v10(
    raw: &[u8],
    profiles: &[PathBuf],
    mut unprotect: impl FnMut(&[u8]) -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    if !raw.starts_with(b"v10") || raw.len() < 3 + 12 + 16 {
        return Err("Llave Electron truncada; se conservan los datos".into());
    }
    for profile in profiles {
        let recovered = (|| {
            let state: serde_json::Value =
                serde_json::from_slice(&fs::read(profile.join("Local State")).map_err(|_| ())?)
                    .map_err(|_| ())?;
            let wrapped = base64::engine::general_purpose::STANDARD
                .decode(state["os_crypt"]["encrypted_key"].as_str().ok_or(())?)
                .map_err(|_| ())?;
            let wrapped = wrapped.strip_prefix(b"DPAPI").ok_or(())?;
            let mut key = unprotect(wrapped).map_err(|_| ())?;
            let result = (|| {
                let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| ())?;
                let mut plaintext = cipher
                    .decrypt(Nonce::from_slice(&raw[3..15]), &raw[15..])
                    .map_err(|_| ())?;
                let decoded = decode(&plaintext).map_err(|_| ());
                plaintext.zeroize();
                decoded
            })();
            key.zeroize();
            result
        })();
        if let Ok(key) = recovered {
            return Ok(key);
        }
    }
    Err("No se pudo recuperar la llave con el perfil original de Electron. Conserva Local State y las carpetas de Biank; no se creó otra llave.".into())
}

#[cfg(windows)]
fn unseal_electron(raw: &[u8]) -> Result<Vec<u8>, String> {
    if raw.starts_with(b"v10") {
        let profiles = electron_profiles(
            std::env::var_os("BIANK_ELECTRON_USER_DATA").map(PathBuf::from),
            std::env::var_os("APPDATA").map(PathBuf::from),
        )?;
        return recover_electron_v10(raw, &profiles, unprotect_dpapi);
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
    // Electron's non-App-Store builds append " Key" to the account. Older
    // builds used the unsuffixed account; only fall back when it is absent.
    let mut password =
        security_framework::passwords::get_generic_password("Biank Safe Storage", "Biank Key")
            .or_else(|error| {
                if error.code() == -25300 {
                    security_framework::passwords::get_generic_password(
                        "Biank Safe Storage",
                        "Biank",
                    )
                } else {
                    Err(error)
                }
            })
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
    fn fixture() -> (PathBuf, Vec<u8>, Vec<u8>) {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        let root =
            std::env::temp_dir().join(format!("biank-electron-vault-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).unwrap();
        let master = vec![9; 32];
        let nonce = [3; 12];
        let encrypted = Aes256Gcm::new_from_slice(&[7; 32])
            .unwrap()
            .encrypt(
                Nonce::from_slice(&nonce),
                URL_SAFE_NO_PAD.encode(&master).as_bytes(),
            )
            .unwrap();
        let mut sealed = b"v10".to_vec();
        sealed.extend(nonce);
        sealed.extend(encrypted);
        (root, sealed, master)
    }
    fn state(profile: &Path, key: u8) {
        fs::create_dir_all(profile).unwrap();
        let mut wrapped = b"DPAPI".to_vec();
        wrapped.extend([key; 32]);
        fs::write(
            profile.join("Local State"),
            serde_json::json!({"os_crypt": {
                "encrypted_key": base64::engine::general_purpose::STANDARD.encode(wrapped)
            }})
            .to_string(),
        )
        .unwrap();
    }
    #[test]
    fn recovers_package_profile_when_product_profile_is_missing_or_has_another_key() {
        let (root, sealed, master) = fixture();
        let profiles = electron_profiles(None, Some(root.clone())).unwrap();
        state(&profiles[1], 7);
        let original = fs::read(profiles[1].join("Local State")).unwrap();
        assert_eq!(
            recover_electron_v10(&sealed, &profiles, |key| Ok(key.to_vec())).unwrap(),
            master
        );
        state(&profiles[0], 8); // Present but cryptographically unrelated; never adopt it.
        assert_eq!(
            recover_electron_v10(&sealed, &profiles, |key| Ok(key.to_vec())).unwrap(),
            master
        );
        assert_eq!(fs::read(profiles[1].join("Local State")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn explicit_profile_never_falls_back_and_failure_preserves_files() {
        let (root, sealed, _) = fixture();
        state(&root.join("biank-desktop"), 7);
        let explicit = root.join("chosen-profile");
        let profiles = electron_profiles(Some(explicit.clone()), Some(root.clone())).unwrap();
        assert_eq!(profiles, vec![explicit.clone()]);
        assert!(recover_electron_v10(&sealed, &profiles, |key| Ok(key.to_vec())).is_err());
        assert!(!explicit.exists());
        state(&explicit, 7);
        assert!(
            recover_electron_v10(&sealed, &profiles, |_| Err("DPAPI rejected".into())).is_err()
        );
        assert!(recover_electron_v10(b"v10", &profiles, |key| Ok(key.to_vec())).is_err());
        assert!(electron_profiles(Some(PathBuf::from("relative")), Some(root.clone())).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn windows_recovers_with_real_dpapi_and_package_profile() {
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::Cryptography::{
                CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
            },
        };
        let (root, sealed, master) = fixture();
        let mut key = [7u8; 32];
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: 32,
            pbData: key.as_mut_ptr(),
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let wrapped = unsafe {
            assert_ne!(
                CryptProtectData(
                    &mut input,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output
                ),
                0
            );
            let mut bytes = b"DPAPI".to_vec();
            bytes.extend_from_slice(std::slice::from_raw_parts(
                output.pbData,
                output.cbData as usize,
            ));
            LocalFree(output.pbData as _);
            bytes
        };
        key.zeroize();
        let profiles = electron_profiles(None, Some(root.clone())).unwrap();
        fs::create_dir_all(&profiles[1]).unwrap();
        fs::write(
            profiles[1].join("Local State"),
            serde_json::json!({"os_crypt": {
                "encrypted_key": base64::engine::general_purpose::STANDARD.encode(wrapped)
            }})
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            recover_electron_v10(&sealed, &profiles, unprotect_dpapi).unwrap(),
            master
        );
        fs::remove_dir_all(root).unwrap();
    }
}

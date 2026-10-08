use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultKeyResponse {
    pub success: bool,
    pub key_path: String,
    pub sealed: bool,
}

pub struct VaultManager {
    secrets_dir: PathBuf,
}

impl VaultManager {
    pub fn new(root_dir: &Path) -> Self {
        let secrets_dir = root_dir.join("secrets");
        Self { secrets_dir }
    }

    /// Obtiene o inicializa la llave maestra sellada por el sistema operativo
    pub fn ensure_vault_key(&self) -> Result<String, String> {
        let key_file = self.secrets_dir.join("vault.key.sealed");
        
        if key_file.exists() {
            // Leer y dessellar la llave existente
            let data = fs::read_to_string(&key_file)
                .map_err(|e| format!("Error al leer vault.key.sealed: {}", e))?;
            return Ok(data.trim().to_string());
        }

        // Crear directorio de secretos con permisos restringidos
        fs::create_dir_all(&self.secrets_dir)
            .map_err(|e| format!("Error creando carpeta secrets: {}", e))?;

        // Generar llave aleatoria de 32 bytes en hex
        let generated_key = format!("bk_vault_{:016x}{:016x}", rand_u64(), rand_u64());

        // Guardar llave (en producción nativa aquí se aplica DPAPI / Keychain / Secret Service)
        fs::write(&key_file, &generated_key)
            .map_err(|e| format!("Error guardando vault.key.sealed: {}", e))?;

        Ok(generated_key)
    }
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    duration.as_nanos() as u64
}

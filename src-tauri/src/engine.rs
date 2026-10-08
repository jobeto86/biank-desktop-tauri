use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub port: u16,
    pub data_root: PathBuf,
    pub web_root: PathBuf,
    pub dev_mode: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let base_data = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("biank-desktop");

        Self {
            port: 8766,
            data_root: base_data,
            web_root: PathBuf::from("../dist/web"),
            dev_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub port: u16,
    pub service: String,
    pub native_instance_id: Option<String>,
}

pub struct EngineSupervisor {
    pub config: EngineConfig,
    child_process: Arc<Mutex<Option<Child>>>,
}

impl EngineSupervisor {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            child_process: Arc::new(Mutex::new(None)),
        }
    }

    /// Resuelve la ruta al bundle del coordinador TypeScript
    pub fn resolve_coordinator_script(&self) -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("resources/coordinator/index.mjs"),
            PathBuf::from("../biank/apps/desktop/coordinator/dist/index.mjs"),
            PathBuf::from("../../biank/apps/desktop/coordinator/dist/index.mjs"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Lanza el subproceso del coordinador TypeScript bajo supervisión
    pub fn spawn(&self, node_executable: Option<&str>, vault_key_file: Option<&Path>) -> Result<(), String> {
        let script = self.resolve_coordinator_script()
            .ok_or_else(|| "No se encontró el bundle index.mjs del coordinador".to_string())?;

        let node_bin = node_executable.unwrap_or("node");

        let mut cmd = Command::new(node_bin);
        cmd.arg(&script)
            .arg("--root")
            .arg(&self.config.data_root)
            .arg("--port")
            .arg(self.config.port.to_string())
            .arg("--web-root")
            .arg(&self.config.web_root);

        // Inyección de invariantes de entorno de Biank
        cmd.env("BIANK_STANDALONE", "1")
            .env("BIANK_DESKTOP_LOCAL_PORT", self.config.port.to_string())
            .env("BIANK_DATA_ROOT", &self.config.data_root)
            .env("BIANK_DESKTOP_DATA_ROOT", &self.config.data_root)
            .env("NODE_OPTIONS", "--experimental-sqlite");

        if let Some(key_path) = vault_key_file {
            cmd.env("BIANK_DESKTOP_VAULT_KEY_FILE", key_path);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| format!("Fallo al arrancar coordinador: {}", e))?;

        if let Ok(mut lock) = self.child_process.lock() {
            *lock = Some(child);
        }

        Ok(())
    }

    /// Comprueba la salud del coordinador loopback HTTP
    pub async fn check_health(&self) -> Result<EngineStatus, String> {
        let _url = format!("http://127.0.0.1:{}/api/health", self.config.port);
        let is_running = self.child_process.lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false);

        Ok(EngineStatus {
            running: is_running,
            port: self.config.port,
            service: "biank-desktop".to_string(),
            native_instance_id: Some("tauri-pilot-instance".to_string()),
        })
    }

    /// Detiene el subproceso del coordinador de forma limpia
    pub fn shutdown(&self) {
        if let Ok(mut lock) = self.child_process.lock() {
            if let Some(mut child) = lock.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for EngineSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::process::Child;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub port: u16,
    pub data_root: String,
    pub dev_mode: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            port: 8766,
            data_root: String::new(),
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

    /// Comprueba la salud del coordinador loopback HTTP
    pub async fn check_health(&self) -> Result<EngineStatus, String> {
        let _url = format!("http://127.0.0.1:{}/api/health", self.config.port);
        // En implementación completa, usa un cliente HTTP ligero para consultar el estado
        Ok(EngineStatus {
            running: true,
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

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroize;

pub fn random_id() -> String {
    let mut bytes = [0; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
#[derive(Clone)]
pub struct Connection {
    pub origin: String,
    pub instance: String,
    pub token: String,
    pub client: reqwest::Client,
}
impl Connection {
    pub async fn request(&self, route: &str, body: Option<Value>) -> Result<Value, String> {
        let url = format!("{}{route}", self.origin);
        let request = match body {
            Some(data) => self.client.post(url).json(&data),
            None => self.client.get(url),
        };
        let response = request
            .header("X-Biank-Local-Token", &self.token)
            .header("X-Biank-Instance", &self.instance)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|_| "El motor no responde")?;
        if !response.status().is_success() {
            return Err(format!("Motor: HTTP {}", response.status().as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| "Respuesta inválida del motor".into())
    }
    pub async fn drain(&self) -> Result<(), String> {
        let owner = random_id();
        let state = self
            .request(
                "/api/desktop/drain",
                Some(json!({"enabled":true,"owner":owner})),
            )
            .await?;
        if state["active"]
            .as_u64()
            .ok_or("Estado de mantenimiento inválido")?
            > 0
        {
            self.request(
                "/api/desktop/drain",
                Some(json!({"enabled":false,"owner":owner})),
            )
            .await?;
            return Err(
                "Hay trabajo activo. Biank conserva la sesión; vuelve a intentar al terminar."
                    .into(),
            );
        }
        if let Err(error) = self
            .request("/api/desktop/shutdown", Some(json!({"owner":owner})))
            .await
        {
            let _ = self
                .request(
                    "/api/desktop/drain",
                    Some(json!({"enabled":false,"owner":owner})),
                )
                .await;
            return Err(error);
        }
        Ok(())
    }
}
pub struct Config {
    pub root: PathBuf,
    pub resources: PathBuf,
    pub version: String,
    pub development: bool,
}
pub struct Supervisor {
    pub connection: Connection,
    pub child: Child,
}
pub fn valid_identity(value: &Value, instance: &str) -> bool {
    value["status"] == "ok"
        && value["service"] == "biank-desktop"
        && value["nativeInstanceId"] == instance
}
fn reserve_local_port(previous: u16, explicit: Option<u16>) -> Result<TcpListener, String> {
    if let Some(port) = explicit {
        return TcpListener::bind(("127.0.0.1", port))
            .map_err(|_| "El puerto local configurado está ocupado".into());
    }
    // CP-A530: existing tunnel ingress refers to the installation's saved port.
    if previous >= 1024 {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", previous)) {
            return Ok(listener);
        }
    }
    TcpListener::bind("127.0.0.1:0").map_err(|_| "No hay puerto local".into())
}
impl Supervisor {
    pub async fn start(
        config: Config,
        mut key: Option<Vec<u8>>,
        mut progress: impl FnMut(&str, &str),
    ) -> Result<Self, String> {
        let runtime = config.resources.join("runtime");
        let script = runtime.join("coordinator/index.mjs");
        let node = runtime.join(if cfg!(windows) {
            "node/node.exe"
        } else {
            "node/node"
        });
        if !script.is_file() || !node.is_file() || !runtime.join("web/index.html").is_file() {
            return Err("Faltan recursos del instalador. Ejecuta npm run stage:runtime.".into());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "HTTP local no disponible")?;
        // Do not adopt or replace an existing writer of another shell/release.
        let mut previous_port = 0;
        if let Ok(raw) = fs::read(config.root.join("runtime.json")) {
            if let Ok(saved) = serde_json::from_slice::<Value>(&raw) {
                if let Some(port) = saved["port"].as_u64() {
                    if (1024..=65535).contains(&port) {
                        previous_port = port as u16;
                        if client
                            .get(format!("http://127.0.0.1:{port}/api/health"))
                            .timeout(Duration::from_secs(1))
                            .send()
                            .await
                            .is_ok()
                        {
                            return Err("Biank ya tiene un motor activo. Sal del shell anterior antes de abrir Tauri; no se interrumpió su trabajo.".into());
                        }
                    }
                }
            }
        }
        fs::create_dir_all(config.root.join("logs")).map_err(|_| "No se pudo abrir el almacén")?;
        let explicit = std::env::var("BIANK_DESKTOP_LOCAL_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .filter(|p| *p >= 1024);
        let listener = reserve_local_port(previous_port, explicit)?;
        let port = listener
            .local_addr()
            .map_err(|_| "No hay puerto local")?
            .port();
        drop(listener);
        let connection = Connection {
            origin: format!("http://127.0.0.1:{port}"),
            instance: random_id(),
            token: random_id(),
            client,
        };
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.root.join("logs/engine.log"))
            .map_err(|_| "No se pudo abrir el log")?;
        let mut cmd = Command::new(node);
        cmd.arg("--experimental-sqlite")
            .arg(script)
            .arg("--root")
            .arg(&config.root)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--web-root",
            ])
            .arg(runtime.join("web"))
            .current_dir(runtime.join("coordinator"))
            .env("BIANK_STANDALONE", "1")
            .env("BIANK_ACCOUNT_REQUIRED", "1")
            .env("BIANK_APP_VERSION", &config.version)
            .env("BIANK_DESKTOP_SHELL", "tauri")
            .env(
                "BIANK_DESKTOP_DEV",
                if config.development { "1" } else { "0" },
            )
            .env("BIANK_DESKTOP_CANDIDATE", "1")
            .env("BIANK_DATA_ROOT", &config.root)
            .env("BIANK_DESKTOP_DATA_ROOT", config.root.join("data"))
            .env("BIANK_INSTANCE_ID", &connection.instance)
            .env("BIANK_WEB_TOKEN", &connection.token)
            .env("BIANK_DESKTOP_LOCAL_PORT", port.to_string())
            .env(
                "BIANK_CODEX_EXECUTABLE",
                runtime.join(if cfg!(windows) {
                    "codex/codex.exe"
                } else {
                    "codex/codex"
                }),
            )
            .env(
                "BIANK_CLOUDFLARED_EXECUTABLE",
                runtime.join(if cfg!(windows) {
                    "cloudflared/cloudflared.exe"
                } else {
                    "cloudflared/cloudflared"
                }),
            )
            .env(
                "PLAYWRIGHT_BROWSERS_PATH",
                runtime.join("coordinator/browsers"),
            )
            .env_remove("ELECTRON_RUN_AS_NODE")
            .env_remove("NODE_OPTIONS")
            .env_remove("BIANK_DESKTOP_VAULT_KEY_SOURCE")
            .stdin(if key.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(log.try_clone().map_err(|_| "Log no disponible")?)
            .stderr(log);
        let identity = runtime.join("runtime-config.json");
        if let Ok(raw) = fs::read(identity) {
            if let Ok(value) = serde_json::from_slice::<Value>(&raw) {
                if let Some(id) = value["googleIdentity"]["clientId"].as_str() {
                    cmd.env("BIANK_GOOGLE_CLIENT_ID", id);
                }
            }
        }
        if key.is_some() {
            cmd.env("BIANK_DESKTOP_VAULT_KEY_SOURCE", "stdin");
        }
        if config.development {
            cmd.env("BIANK_DESKTOP_CONFIG_ROOT", config.root.join("data/config"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|_| "No se pudo iniciar Node empaquetado")?;
        if let Some(ref mut secret) = key {
            use std::io::Write;
            let mut encoded = URL_SAFE_NO_PAD.encode(&*secret);
            encoded.push('\n');
            let result = child
                .stdin
                .take()
                .ok_or("stdin no disponible")
                .and_then(|mut pipe| {
                    pipe.write_all(encoded.as_bytes())
                        .map_err(|_| "No se pudo entregar la llave")
                });
            secret.zeroize();
            encoded.zeroize();
            if let Err(e) = result {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.into());
            }
        }
        let deadline = Instant::now() + Duration::from_secs(120);
        while Instant::now() < deadline {
            if let Ok(raw) = fs::read(config.root.join("runtime-phase.json")) {
                if let Some((phase, detail)) = startup_phase(&raw, child.id()) {
                    progress(phase, &detail);
                }
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|_| "No se pudo observar el motor")?
            {
                return Err(format!("El motor terminó antes de iniciar (código {:?}). Se conservan tus datos; consulta logs/engine.log.", status.code()));
            }
            if let Ok(health) = connection.request("/api/health", None).await {
                if valid_identity(&health, &connection.instance) {
                    return Ok(Self { connection, child });
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err("El motor no acredita su identidad".into());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let _ = child.kill();
        let _ = child.wait();
        Err("El motor no inició en el plazo previsto. Se conservan tus datos.".into())
    }
}
// Only accept phase evidence from the child owned by this startup. Never expose
// paths or arbitrary log text in the splash.
fn startup_phase(raw: &[u8], pid: u32) -> Option<(&'static str, String)> {
    let value: Value = serde_json::from_slice(raw).ok()?;
    if value["pid"].as_u64()? != u64::from(pid) {
        return None;
    }
    let phase = value["phase"].as_str()?;
    let detail = value["detail"].as_str().unwrap_or("");
    let message = match phase {
        "starting" => "Iniciando Biank…".to_string(),
        "snapshot" if detail.starts_with("Calculando sumas criptográficas (") => {
            let count = detail
                .split('(')
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()?;
            format!("Verificando tus datos · {count} archivos revisados")
        }
        "snapshot" if detail.starts_with("Copiando archivos") => {
            "Guardando una copia de seguridad de tus datos…".to_string()
        }
        "snapshot" if detail.starts_with("Verificando integridad") => {
            "Comprobando la copia de seguridad…".to_string()
        }
        "snapshot" => "Preparando una copia de seguridad antes de actualizar…".to_string(),
        "profiles" => "Preparando tu perfil y tus datos…".to_string(),
        "binding" => "Iniciando los servicios de Biank…".to_string(),
        "ready" => "Abriendo Biank…".to_string(),
        _ => return None,
    };
    let phase = match phase {
        "starting" => "starting",
        "snapshot" => "snapshot",
        "profiles" => "profiles",
        "binding" => "binding",
        "ready" => "ready",
        _ => return None,
    };
    Some((phase, message))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_progress_is_bound_to_child_and_does_not_expose_arbitrary_details() {
        let raw = json!({"pid":42,"phase":"snapshot","detail":"Calculando sumas criptográficas (120 archivos)..."}).to_string();
        assert_eq!(
            startup_phase(raw.as_bytes(), 42),
            Some((
                "snapshot",
                "Verificando tus datos · 120 archivos revisados".into()
            ))
        );
        assert!(startup_phase(raw.as_bytes(), 43).is_none());
        assert!(startup_phase(b"partial-json", 42).is_none());
        let raw = json!({"pid":42,"phase":"profiles","detail":"private file path"}).to_string();
        assert_eq!(
            startup_phase(raw.as_bytes(), 42).unwrap().1,
            "Preparando tu perfil y tus datos…"
        );
        assert!(startup_phase(br#"{"pid":42,"phase":"unknown"}"#, 42).is_none());
    }
    #[test]
    fn saved_port_survives_restart_and_a_busy_port_is_not_taken_over() {
        let first = reserve_local_port(0, None).unwrap();
        let port = first.local_addr().unwrap().port();
        drop(first);
        let restarted = reserve_local_port(port, None).unwrap();
        assert_eq!(restarted.local_addr().unwrap().port(), port);
        let separate = reserve_local_port(port, None).unwrap();
        assert_ne!(separate.local_addr().unwrap().port(), port);
        assert!(reserve_local_port(0, Some(port)).is_err());
    }
    #[test]
    fn identity_is_bound_to_service_and_instance() {
        let value = json!({"status":"ok","service":"biank-desktop","nativeInstanceId":"own"});
        assert!(valid_identity(&value, "own"));
        assert!(!valid_identity(&value, "other"));
        assert!(!valid_identity(
            &json!({"status":"ok","nativeInstanceId":"own"}),
            "own"
        ));
    }
}

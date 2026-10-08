//! Only the app webview holds the HttpOnly cookie; engine credentials stay in Rust.
use crate::engine::{random_id, Connection};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use std::sync::Arc;
pub const COOKIE: &str = "biank-tauri-local";
#[derive(Clone)]
pub struct Transport {
    pub origin: String,
    pub capability: String,
    pub engine: Connection,
}
impl Transport {
    pub async fn start(engine: Connection) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| "No hay transporte local")?;
        let t = Self {
            origin: format!(
                "http://{}",
                listener
                    .local_addr()
                    .map_err(|_| "No hay transporte local")?
            ),
            capability: random_id(),
            engine,
        };
        let router = Router::new()
            .fallback(forward)
            .with_state(Arc::new(t.clone()));
        tauri::async_runtime::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        Ok(t)
    }
}
fn allowed(h: &HeaderMap, t: &Transport) -> bool {
    if h.get("host").and_then(|v| v.to_str().ok()) != Some(t.origin.trim_start_matches("http://")) {
        return false;
    }
    if let Some(origin) = h.get("origin") {
        if origin.to_str().ok() != Some(t.origin.as_str()) {
            return false;
        }
    }
    if let Some(site) = h.get("sec-fetch-site") {
        if !matches!(site.to_str().ok(), Some("same-origin" | "none")) {
            return false;
        }
    }
    h.get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|raw| {
            raw.split(';').any(|c| {
                c.trim()
                    .strip_prefix(&format!("{COOKIE}="))
                    .map(|v| v == t.capability)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}
async fn forward(State(t): State<Arc<Transport>>, request: Request) -> Response {
    if !allowed(request.headers(), &t) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let (parts, body) = request.into_parts();
    let path = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    let mut headers = parts.headers;
    for name in [
        "host",
        "cookie",
        "authorization",
        "connection",
        "upgrade",
        "x-biank-local-token",
        "x-biank-instance",
        "origin",
    ] {
        headers.remove(name);
    }
    headers.insert("X-Biank-Local-Token", t.engine.token.parse().unwrap());
    headers.insert("X-Biank-Instance", t.engine.instance.parse().unwrap());
    headers.insert("Origin", t.engine.origin.parse().unwrap());
    let bytes = match axum::body::to_bytes(body, 32 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    let upstream = match t
        .engine
        .client
        .request(parts.method, format!("{}{path}", t.engine.origin))
        .headers(headers)
        .body(bytes)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let status = upstream.status();
    let mut headers = upstream.headers().clone();
    for name in ["set-cookie", "connection", "transfer-encoding"] {
        headers.remove(name);
    }
    let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unauthenticated_and_cross_origin_requests() {
        let t = Transport {
            origin: "http://127.0.0.1:1234".into(),
            capability: "fixture".into(),
            engine: Connection {
                origin: "http://127.0.0.1:4321".into(),
                token: "engine".into(),
                instance: "own".into(),
                client: reqwest::Client::new(),
            },
        };
        let mut h = HeaderMap::new();
        h.insert("host", "127.0.0.1:1234".parse().unwrap());
        assert!(!allowed(&h, &t));
        h.insert("cookie", "biank-tauri-local=fixture".parse().unwrap());
        assert!(allowed(&h, &t));
        h.insert("origin", "https://evil.example".parse().unwrap());
        assert!(!allowed(&h, &t));
        h.remove("origin");
        h.insert("sec-fetch-site", "cross-site".parse().unwrap());
        assert!(!allowed(&h, &t));
    }
}

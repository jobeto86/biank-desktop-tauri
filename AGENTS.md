# biank-desktop-tauri — Reglas y Herramientas para Agentes

Shell nativo **Tauri v2** para el core privado de Biank (`jobeto86/biank`, spec canónico en `../biank/apps/desktop/SPEC.md`).

## 1. Reglas e Invariantes

- **Frontera de repositorios**:
  - `jobeto86/biank`: core privado (`coordinator`, `web`, `resources`).
  - `jobeto86/biank-desktop-tauri`: shell público Rust/Tauri v2 y workflows de build para Windows/macOS.
  - `jobeto86/biank-desktop`: canal de distribución y feed `latest.json`.
- **Seguridad y Secretos**:
  - `BIANK_CORE_READ_SSH_KEY` es únicamente un grant de lectura del repo privado en CI; nunca debe incorporarse al artefacto empaquetado.
  - Los tokens privados del motor viven en Rust; la UI recibe únicamente una cookie `HttpOnly` local.
  - El bridge nativo (`tools/bridge.js`) se inyecta sólo en el webview first-party de Biank; páginas externas en Chromium no reciben bridge.
- **Ciclo de vida y actualizaciones**:
  - El supervisor comprueba identidad y salud HTTP del motor hijo y nunca reemplaza un motor vivo de otra instalación.
  - Conserva la ruta de datos de instalaciones previas y migra la llave existente al almacén nativo del SO sin borrar el origen.
  - Aplicar una actualización o cerrar la aplicación exige drain y espera al cierre limpio del motor.
  - Los workflows de build retienen candidatos revisables; la promoción a release oficial es siempre explícita (`tools/publish-release.mjs`).

## 2. Comandos de Desarrollo y Verificación

```bash
npm ci
npm run stage:runtime       # Prepara recursos del core y manifiesto SHA-256
npm run check               # cargo check sobre src-tauri/Cargo.toml
npm run test                # cargo test sobre src-tauri/Cargo.toml
node --test tools/*.test.mjs
```

## 3. Catálogo de herramientas del repositorio (`tools/`)

Los scripts de empaquetado, staging, firma y verificación viven en `tools/`:

- **Staging y empaquetado**:
  - `tools/stage-runtime.mjs` — Empaqueta el runtime completo del core (`coordinator`, `web`, Node, Chromium, `cloudflared`) y genera el manifiesto SHA-256 ligado al shell.
  - `tools/stage-codex.mjs` — Prepara el binario nativo de OpenAI Codex según target (`x86_64-pc-windows-msvc` / `aarch64-apple-darwin`).
  - `tools/runtime-links.mjs` — Copia el runtime preservando symlinks/aliases de frameworks en macOS y rechazando enlaces que escapen de la raíz.
  - `tools/sync-web.mjs` — Sincroniza los activos estáticos web hacia `dist/`.
  - `tools/configure-updater.mjs` — Configura la clave pública `BIANK_TAURI_PUBLIC_KEY` del updater firmado antes de compilar un candidato publicable.
- **Release y distribución**:
  - `tools/publish-release.mjs` — Ceremonia explícita de publicación: verifica artefactos Windows/macOS, firmas e integridad SHA-256 y publica en el canal `jobeto86/biank-desktop` (soporta `--sin-publicar`).
- **Smoke, QA y verificación de instaladores**:
  - `tools/smoke-tauri.mjs` — Smoke test local del binario Tauri compilado y su coordinación con el motor.
  - `tools/validate-installed.mjs` — Valida una instalación real del paquete nativo y su arranque end-to-end.
  - `tools/verify-macos-candidate.py` — Verifica todos los recursos y hashes dentro del archivo `.tar.gz` del updater de macOS sin instalarlo.
  - `tools/assess-macos.mjs` — Evalúa el estado de firma y Gatekeeper en macOS (`darwin`).
  - `tools/electron-vault-fixture.cjs` — Fixture para pruebas de migración de llaves desde instalaciones Electron previas.
- **Bridges de Webview**:
  - `tools/bridge.js` — Bridge first-party inyectado en el webview principal de Tauri (`window.biankDesktop`, `window.biankBrowser`).
  - `tools/qa-bridge.js` — Sonda de diagnóstico inyectada únicamente en corridas aisladas de QA/debug, nunca en releases.
- **Tests de herramientas (`tools/*.test.*`)**:
  - `tools/auth-bridge.test.mjs`, `tools/runtime-links.test.mjs`, `tools/startup.test.mjs`, `tools/windows-installer-hook.test.ps1` (guarda nativa `src-tauri/src/install_guard.rs` vía hooks NSIS).

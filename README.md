# Biank Desktop — Tauri

Shell nativo Tauri v2 para el core privado de Biank. Tauri aloja la interfaz;
Chromium empaquetado abre el navegador del agente en ventana propia, bajo demanda.
Persona y MCP operan las mismas páginas. Cada agente conserva su sesión y cada
conversación sus pestañas. Las rutinas pueden solicitar headless explícitamente.

El instalador incluye Node, coordinador, UI, Codex, Chromium y cloudflared.
El tamaño y RAM del conjunto se miden sobre instaladores reales; no se promete
una workstation de 18 MB ni menos de 50 MB de RAM.

## Repositorios

- `jobeto86/biank`: core privado y spec canónico `apps/desktop/SPEC.md`.
- `jobeto86/biank-desktop-tauri`: shell público y builds alojados Windows/macOS.
- `jobeto86/biank-desktop`: canal existente de distribución; no se cambia aquí.

La build manual fija un SHA completo del core. `BIANK_CORE_READ_SSH_KEY` es un
grant de lectura del repo privado, nunca incorporado al artefacto. El workflow
retiene candidatos revisables; no promueve releases automáticamente.

## Desarrollo

Requiere Node 22, Rust y dependencias nativas Tauri. Primero construye el core,
web y payload standalone, incluyendo Codex, Chromium y cloudflared. Luego:

```sh
npm ci
npm run stage:runtime
npm run tauri dev
npm run test
```

`stage:runtime` prepara recursos completos y un manifiesto SHA-256 ligado al
shell compilado. La UI recibe un cookie HttpOnly local; tokens privados del motor
quedan en Rust. El supervisor comprueba identidad y salud HTTP del hijo y nunca
reemplaza un motor vivo de otra instalación. Conserva la ruta de datos Electron.
La llave existente se migra al almacén del SO sin borrar el origen.

El updater exige firmas. `npm run configure:updater` usa la variable pública
`BIANK_TAURI_PUBLIC_KEY`; las claves privadas se resuelven únicamente en CI.
Un build de QA sin canal firmado muestra la falta de configuración y puede arrancar.
Aplicar una actualización o salir exige drain y espera al cierre del motor.

## Validación

Consulta [el reporte de validación](docs/validation-2026-10-07.md) y
[la decisión de navegador](docs/browser-design.md). No está certificado el
reemplazo de instalaciones Electron existentes ni se ha publicado esta versión.

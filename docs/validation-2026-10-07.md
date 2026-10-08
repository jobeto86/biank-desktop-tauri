# Validación Tauri — 2026-10-07

Estado: **arranque e integración local PASS; release y migración pendientes**.
Roberto aprobó Tauri con Chromium del agente en ventana propia.

## Implementación

El shell arranca Node empaquetado, verifica SHA-256 de todos los recursos y
comprueba identidad/health del motor. El payload contiene core, UI, Node, Codex,
Chromium y cloudflared. Conserva la ruta de datos existente y rechaza sustituir
un motor vivo. El proxy autentica la UI con cookie HttpOnly; el token privado
queda en Rust. El bridge sólo se concede al origen exacto del proxy local.
Archivos conserva el panel lateral; Navegador abre o enfoca Chromium propio.
La mascota conserva la pausa vigente en la UI; no se activa una ventana vacía.

La llave maestra se entrega por stdin desde el llavero del SO. Hay código de
migración DPAPI/Keychain para Electron, sin borrar material anterior. Si hay
bóvedas existentes sin llave accesible, falla antes de generar otra llave.
Eso **no acredita aún** recuperación de una instalación Windows/macOS real.

El updater consulta y descarga independientemente del login y exige firma.
Aplicar requiere drain, cierre del motor y sólo entonces instalación/restart.
Sin clave pública configurada informa un canal no configurado. No se afirma
que exista un canal firmado operativo.

## Evidencia local Linux ARM64

- Core TypeScript y web TypeScript/Vite: build PASS.
- Regresiones seleccionadas del core: 24 PASS, 0 FAIL, 0 SKIP.
- Chromium real con Playwright/MCP: DOM compartido con interacción manual,
  login persistente tras reiniciar, cookies compartidas y pestañas aisladas por
  conversación; cierre canónico conserva otras conversaciones: PASS.
- Adquisición explícita headless en Tauri: PASS (HeadlessChrome observado);
  adquisición ordinaria visible: PASS (sin HeadlessChrome).
- Rust: compilación y cuatro tests PASS: identidad, autenticación/cross-origin,
  formato de llave y rechazo de recursos modificados/traversal.
- Ventana real: login visible, bridge IPC, health autenticado y salida nativa
  con drain y espera al hijo PASS. Evidencia sintética:
  `/tmp/biank-tauri-smoke-rgRi8u/result.json` y `tauri-login.png`.
- El fixture de cierre inicialmente usó un owner demasiado corto; se corrigió
  y el motor QA restante se cerró por su API, verificando su identidad.
- La verificación SHA usa lectura por bloques y sha2 optimizado también en dev.
  WebKitGTK en este host CIX requiere Mesa del sistema y CPU rendering en Xvfb;
  estas opciones pertenecen al fixture, no al build de producción.

Logs locales: `/tmp/biank-tauri-core-tests.log`,
`/tmp/biank-tauri-browser-test.log`, `/tmp/biank-tauri-test.log`,
`/tmp/biank-tauri-web-build.log`, `/tmp/biank-tauri-smoke.log`.

## Antes de reemplazar Electron en clientes

1. Configurar grant read-only del core en CI; fijar SHA aprobado y ejecutar
   instaladores alojados Windows x64/macOS ARM64. No requiere runners privados.
2. Validar instalación limpia sin Node, Chrome ni herramientas externas,
   ventana/foco/descargas/cierre y almacén nativo en ambos sistemas.
3. Validar migración de bóveda y perfiles/cookies de Electron con datos
   sintéticos existentes; no crear credenciales nuevas sobre datos cifrados.
4. Provisionar identidad de firma, certificados/notarización según plataforma
   y dos candidatos firmados. Probar firma inválida, actualización sin login,
   drain con trabajo activo, instalación y arranque posterior.
5. Validar Electron → Tauri y continuidad del canal `jobeto86/biank-desktop`.
   El workflow actual retiene artefactos; no publica ni promueve latest.

No se configuraron secretos, publicaron releases ni reiniciaron servicios de
producción. CUR-252, CUR-256, CUR-259 y CUR-260 siguen abiertos: la evidencia
local no acredita reparación de la instalación del cliente.

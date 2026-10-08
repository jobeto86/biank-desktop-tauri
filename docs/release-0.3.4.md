# Biank Desktop 0.3.4 — primer acceso a ChatGPT

El perfil personal nuevo no preparaba su workspace. Codex elegía esa carpeta
como cwd y fallaba con ENOENT antes de generar la URL de autorización: el usuario
veía un error genérico y ningún navegador. Se reprodujo con un perfil aislado
y Codex real; crear workspace permitió consultar la cuenta sin error.

AccountProfiles.forUser prepara ahora esa carpeta antes de iniciar el motor.
También repara perfiles anteriores y conserva el contenido existente. No cambia
OAuth, las cuentas ni la política del toggle. CP-A397 se actualizó antes del código.

## Fuente y validación

Core 26c2bb8d1dbbb1656219e2ff9002581936f256db, tag desktop-core-v0.3.4.
Shell 1174aefa3328508ed5aa1fd0f3dd031bf24a4862, tag v0.3.4.
[Run 37833157720](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37833157720)
SUCCESS en Windows x64 y macOS ARM64. El nuevo gate usa Codex 0.159.2 empaquetado:
crea un perfil vacío, consulta account/read, obtiene URL oficial de autorización
y cancela. El gate no abre navegador ni completa OAuth con una cuenta real.
La apertura del navegador mediante el bridge nativo, Chromium visible/headless
y arranque desde NSIS instalado / DMG montado y copiado también pasaron.

Local: 18 pruebas de perfiles/login, 13 de host/cambio/arranque y una integración
con Codex real PASS. Build TypeScript y examples Rust 0.3.4 PASS.
Integridad de 4.579 recursos Windows y 4.606 Mac PASS. Se conserva una única
copia física del framework Chromium y se omite headless shell adicional.
DMG: 397.995.200 bytes; NSIS final: 438.997.312; tar updater: 409.284.556.
Windows recibió Authenticode de desarrollo con timestamp RFC3161 verificado;
la firma updater se aplica después, vinculada a versión 0.3.4.

## Límites

No se certifica login completo de Rafa: queda pendiente comprobarlo en su equipo.
Mac tiene firma ad hoc deep/strict válida, pero Gatekeeper con cuarentena rechaza
el paquete (exit 3): falta Developer ID/notarización. CUR-261 sigue abierto.
Windows usa certificado de desarrollo, no confianza comercial pública.

## Actualización

Se distribuye por el updater existente. Buscar actualizaciones y usar Reiniciar
y actualizar cuando esté disponible; verificar 0.3.4 antes de intentar ChatGPT.
Si ya había una versión anterior descargada, puede requerirse instalarla,
volver a abrir y completar otro ciclo. Cerrar con X sólo oculta la aplicación;
Salir instala una actualización descargada tras drenar el motor.
La instalación viva de Roberto no se reinicia ni actualiza por esta publicación.

La validación pública se documenta en [el recibo del updater](release-0.3.4-updater.json).
Comprueba manifiesto, descarga completa, firma, versión y SHA-256 con el plugin
Tauri real; no simula instalación ni una actualización en el equipo del cliente.

[Publicado como latest](https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.4)
con nueve assets y tamaños exactos. Ambos aliases HTTP 206; manifiesto público
idéntico y DMG público completo coincide por SHA-256. Updater Tauri real para
Windows y Mac: descarga, firma, versión 0.3.4 y SHA-256 PASS.

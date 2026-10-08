# Biank Desktop 0.3.1 — evidencia de publicación

[Publicado como latest](https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.1)
con nueve assets. Ambos aliases de instalación responden HTTP 206; sus tamaños
y el manifiesto público coinciden exactamente con los archivos preparados.
La descarga completa del DMG público también coincide por SHA-256.

## Instalación nativa

[Run 37811974094](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37811974094)
terminó SUCCESS para Windows x64 y macOS ARM64. Ambos probaron Chromium real,
la ventana nativa, UI, motor y Codex desde el instalador del sistema.
Windows instaló el NSIS; macOS montó el DMG y arrancó una copia de la aplicación.
La firma macOS ad hoc pasó `codesign --verify --deep --strict` antes y después
de la instalación. No se dispone de Developer ID/notarización Apple.

Core inmutable `1971a75eb582dcab4193fe75623846ba0f2fd281`, tag
`desktop-core-v0.3.1`. Shell `f7abf2bffa1b4c97907902aeac12c07f8a50615d`, tag
`v0.3.1`. El harness `fde9e38` sólo corrige dos expectativas de tests y no
modifica código runtime respecto al core empaquetado.

## Validaciones

- Motor: 1.056 PASS, uno omitido, cero fallos/cancelaciones. La suite usa la UI
  recién construida en una ruta aislada, XDG_RUNTIME_DIR medido, dos procesos
  concurrentes y timeout de 120 segundos. En la primera corrida, el directorio
  systemd ausente y el bundle previo produjeron resultados no acreditables;
  las expectativas de borrado y etiqueta de tarea también se actualizaron.
- Interfaz: 393 tests, uno omitido, cero fallos tras actualizar la etiqueta.
- Rust: seis PASS. Normalización de recursos macOS: uno PASS.
- TypeScript del coordinador y build Vite: PASS.
- Windows final tras Authenticode: 4.871 recursos coinciden con el manifest;
  certificado de desarrollo y timestamp RFC3161 DigiCert verificados. No es
  una identidad comercialmente confiable. El updater se vuelve a firmar después
  de Authenticode y acredita exactamente la versión 0.3.1.
- macOS updater: firma/version y 5.175 recursos del tar.gz PASS.

El NSIS mide 538.859.184 bytes; el DMG, 843.656.435; el tar.gz updater,
867.677.493. La aplicación macOS descomprimida mide aproximadamente 1,9 GB:
Chromium representa 1.268 MiB, Codex 316 MiB y Node 108 MiB. Esta versión no
reduce el tamaño del paquete.

## Alcance

Memoria periódica recuperable CP-A555, mensaje vivo de Biank CP-A554,
distinción de conversación/tarea CP-A349, conservación de identidad ante
fallos transitorios CP-A544 y corrección de firma ad hoc Mac. El fallo concreto
de un cliente Mac requiere comprobar el nuevo artefacto en ese equipo antes de
marcarse resuelto. No se certifica aquí la migración completa desde Electron ni
un ciclo de actualización instalado 0.3.0 → 0.3.1. La primera instalación Tauri
desde Electron sigue siendo manual; el canal Tauri exige firma ligada a versión.
La instalación local usada por Roberto no se reinició durante esta publicación.

## Canal público

El plugin Tauri real descargó desde `latest` los paquetes Windows y macOS
y acreditó manifiesto, firma, versión 0.3.1 y SHA-256: **PASS** para ambos.
[Evidencia](release-0.3.1-updater.json). No instala ni simula un salto de versión.
El ticket CUR-261 permanece abierto: el gate de arranque no prueba Gatekeeper
con cuarentena de Chrome; ad hoc no equivale a confianza Developer ID/notarización.

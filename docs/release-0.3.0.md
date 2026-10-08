# Biank Desktop 0.3.0 — validación de release

Estado: draft completo en carga; aún no promovido a latest.

Las PRs [core #1](https://github.com/jobeto86/biank/pull/1) y
[shell #1](https://github.com/jobeto86/biank-desktop-tauri/pull/1) están fusionadas.
El spec canónico define el primer paso manual desde Electron y el canal firmado
Tauri para actualizaciones posteriores. La lectura nativa de llaves anteriores
se conserva; no se empaqueta el ejecutable, SDK ni framework Electron.

## Instaladores reales

- Windows: [run 37786372449](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37786372449),
  job 113342068997 PASS, shell 349e6fd. El job macOS de ese mismo run se canceló
  en cola; el resultado Windows es independiente. Chromium real, seis tests
  Rust/firma, bóveda sintética cifrada por Electron, NSIS instalado, ventana
  nativa, motor y Codex PASS. Los 4.871 recursos también pasaron sobre el
  instalador final después de Authenticode.
- macOS ARM64: [run 37791458236](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37791458236),
  shell d4b9a94 en main, todos los jobs PASS. Chromium real, seis tests Rust/firma,
  arranque nativo, DMG montado, app instalada, ventana nativa, motor y Codex PASS.
  Los 5.175 recursos del tar.gz updater pasaron la auditoría de integridad.
- Ambos artefactos: firma updater y versión 0.3.0 PASS. La identidad pública
  del updater configurada en CI coincide con la clave pública de verificación.
- NSIS final: 513.89 MiB; DMG:
  804.30 MiB; tar.gz updater:
  827.17 MiB. No se acredita reducción de RAM
  ni del tamaño completo.

Core inmutable: `d82d9f6c67bf027e9e74da157703a339357d8e5c`, tag
`desktop-core-v0.3.0`. Shell oficial: tag `v0.3.0` en `d4b9a94`; el tag del
bootstrap inicial se conserva como `v0.3.0-bootstrap-af84a32`.

La suite completa del core pasó 1.034 tests, omitió uno y tuvo un fallo de entorno
Linux. Ese chequeo de sintaxis systemd pasó al repetirlo con el directorio
runtime del usuario medido. Las 25 regresiones de navegador y los dos tests
concurrentes de transición de salud también pasaron.

## Canal de actualización

Tauri comprueba al arrancar y cada seis horas. Descarga versiones nuevas,
exige firma ligada a la versión y aplica después de drenar el motor al salir o
por acción explícita. El primer paso desde Electron es manual: no se certifica
un salto automático desde electron-updater y no se publican feeds Electron.
`validate_update_feed` usa el plugin Tauri real para comprobar manifiesto,
descarga, firma, versión y SHA-256 de la publicación; nunca instala ni fuerza
una actualización de la aplicación cliente.

Estado del chequeo del canal público: pendiente de promoción.

## Límites medidos

Windows conserva el certificado de desarrollo del canal anterior, fingerprint
SHA-1 `A890D223E6CA8D0D96EEEB3DC24E8B12785E538B`: firma y timestamp RFC3161
DigiCert verificados. No es una identidad comercialmente confiable. macOS no
incluye Developer ID/notarización Apple. La firma updater es independiente.

La migración sintética macOS quedó esperando consentimiento Keychain; no se
reporta PASS. La lectura de cuentas `Biank Key` y la histórica `Biank` se conserva
sin ejecutar Electron. La migración completa de un cliente existente y un ciclo
instalado de actualización 0.3.0 a una versión futura no se certifican aquí.
Los tickets de fallos cliente permanecen abiertos sin evidencia específica.

# CI Tauri habilitado — 2026-10-08

Roberto autorizó configurar el grant y la firma («Sí»).

- Deploy key SSH read-only en el core `jobeto86/biank`; el satélite recibe su
  clave privada mediante `BIANK_CORE_READ_SSH_KEY`. No se copió la sesión OAuth
  personal de GitHub. Ambos runners acreditaron el checkout privado.
- Firma updater propia: secreto `TAURI_SIGNING_PRIVATE_KEY` y variable pública
  `BIANK_TAURI_PUBLIC_KEY`. Copia local de recuperación fuera de los repositorios,
  directorio 0700 y archivos privados 0600; los valores no se imprimieron.
- SHA core: `d82d9f6`; rama candidata
  `candidate/tauri-browser-20261008` en ambos repositorios. Se preservaron los
  cambios ajenos sin incluirlos en el commit del core.
- Prueba local de firma PASS: original válido, contenido modificado rechazado,
  versión 0.3.0 ligada al comentario firmado. El updater exige versión firmada.
- Regresión adicional: cerrar la ventana real Chromium y reabrirla fallaba;
  corregido en core d82d9f6 y probado con 25 regresiones PASS. La sesión se conserva.
- Instalador macOS inicial medido: 571.82 MiB; tar.gz updater 589.06 MiB.
  El ZIP de Actions incluía también el app sin comprimir (2.6 GB); el workflow
  conserva ahora sólo los formatos instalables y updater con sus firmas.
- Build inicial: https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37769690408
  macOS compiló y su firma/verificación de versión pasó. La auditoría del
  tar.gz detectó enlaces Chromium materializados por el bundler y un manifiesto
  incompatible: ese candidato falla integridad y no se distribuye.
- Corrección: normalizar enlaces macOS antes de sellar el manifiesto. Un test
  reproduce framework Versions/Current y verifica archivos materializados,
  preservación en Linux y rechazo de enlaces externos. Se evita el fast path
  de copia de Node 22.22.1 que retenía enlaces pese a dereference:true.

Electron también permite firma Authenticode/Developer ID. Adoptar Tauri no
resuelve esas identidades: la firma updater no es un certificado Authenticode ni
notarización Apple.
No se aprovisionaron esas identidades, promovieron releases, fusionaron ramas
ni sustituyeron instalaciones de clientes.

Windows rechazó el fixture firmado después de que Git lo convirtiera a CRLF.
`.gitattributes` conserva sus bytes mediante `-text`; no se relaja la firma.
El workflow permite repetir Windows sin reconstruir macOS.

El primer NSIS consumió 13 minutos de compresión LZMA (367.91 MiB). Los
candidatos del updater usan ZLIB para medir la reducción del tiempo de build
y el incremento de tamaño, conservando los mismos recursos y controles de firma.

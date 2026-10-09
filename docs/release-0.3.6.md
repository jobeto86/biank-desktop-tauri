# Biank Desktop 0.3.6 — el arranque no corta el respaldo de actualización

Origen: un cliente Windows quedó atorado al actualizar. El shell Tauri mataba
al motor a los 120 s fijos aunque el respaldo previo a migrar siguiera
avanzando; el marcador de versión no se escribía y cada arranque repetía el
respaldo y volvía a morir.

- Shell: el arranque sólo se corta tras 60 s sin progreso del hijo propio o
  10 min en total (igual que Electron).
- Core `6d6e0e8` (= 0.3.5 `26c2bb8` + latido del respaldo cada 5 s, limpieza
  de staging abortado y limpieza tolerante a EBUSY en una prueba nativa).
- Gate nuevo en CI: candidato NSIS instalado arranca sobre un perfil sintético
  de 3 GB marcado 0.3.5; exige respaldo publicado, marcador avanzado y ningún
  staging huérfano.

Publicado como latest: https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.6.
Fuente shell adcdb4c, core 6d6e0e8, run 37953380821 SUCCESS en ambos targets.
Gate de perfil grande Windows PASS: 3072 MB, arranque en 88 s. Authenticode de
desarrollo con timestamp DigiCert PASS; firmas updater ligadas a 0.3.6 PASS.
Nueve assets; latest.json público idéntico al preparado; alias HTTP 206 y
SHA-256 del instalador Windows descargado igual al local.

Límites: 88 s en el runner no demuestra un caso >120 s en hardware de cliente;
la recuperación del cliente que reportó el atasco no está verificada.

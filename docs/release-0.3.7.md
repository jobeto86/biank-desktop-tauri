# Biank Desktop 0.3.7 — complemento CBSuite obligatorio

Origen: en la Mac de Rafael (0.3.6) Biank respondía «no hay conexiones ni accesos» y
no encontraba el ERP aunque la cuenta era CBSuite.

- Core `aadb584`: CP-A560 (servidor MCP `cbsuite`, `listar_conexiones.empresa`,
  skill de producto `operar-cbsuite`, revocación 409, contexto en encargos),
  CP-A557 navegador integrado, CUR-278 latido del buzón (inactivo sin
  `mailbox-heartbeat.json`), disquete Guardado/Guardar, encabezado sin «En turno».
- Shell `c38c80d`: guarda del instalador Windows (motor quieto antes de reemplazar).

Publicado como latest: https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.7.
Run 37983399844 SUCCESS en ambos targets. El primer intento Windows cayó por EBUSY
en la limpieza de la prueba de autorización ChatGPT (mismo flake que el run
37952338527); el reintento del mismo run pasó sin cambiar SHA. Suite core 1068/1069
sin fallas. Authenticode de desarrollo con timestamp DigiCert PASS; firmas updater
ligadas a 0.3.7 PASS. Nueve assets; latest.json público idéntico al preparado; SHA-256
del instalador Windows y del DMG descargados iguales a los locales.

Límites: la conexión CBSuite en la instalación de Rafael no está verificada; 0.3.7
expone su estado para diagnosticarla. macOS con firma ad hoc; Windows con certificado
de desarrollo.

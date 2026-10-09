# Biank Desktop 0.3.8 — Complementos, Aplicaciones, MCP y Habilidades

- Core `8bc18b1`: CP-A561 (cuatro pestañas con contador, «Explorar directorio» y «Añadir»,
  CBSuite fijo encendido y bloqueado, pausa sin revocar en Aplicaciones y MCP,
  habilidades personales y del producto agrupadas, Contraseñas aparte) y CP-A560-7
  (tarjeta CBSuite con el mismo estado que `listar_conexiones.empresa`).
- Shell `d9736ab`: sólo versión.

Publicado como latest: https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.8.
Run 37996603164 SUCCESS en ambos targets al primer intento. Suite core 1072/1073 sin
fallas. Authenticode de desarrollo con timestamp DigiCert PASS; firmas updater ligadas a
0.3.8 PASS. La ceremonia creó el borrador pero la subida de los tres archivos grandes
falló; se subieron al mismo borrador, se verificaron los nueve tamaños contra los locales
y se promovió a latest. latest.json público idéntico al preparado; SHA-256 del
instalador Windows, el DMG y el tar.gz descargados iguales a los locales.

Límites: la conexión CBSuite en la instalación de Rafael sigue sin verificar. macOS con
firma ad hoc; Windows con certificado de desarrollo.

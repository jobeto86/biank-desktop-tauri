# Biank Desktop 0.3.2 — empaquetado compacto

Release: https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.2.

## Causa y corrección

La copia de recursos macOS expandía los enlaces internos del framework de
Chromium en archivos independientes. El paquete contenía tres copias físicas
del framework. Ahora se preservan los enlaces relativos, se rechazan enlaces
absolutos o fuera del runtime y el bundle macOS copia el directorio completo
sin desreferenciar. Sólo queda una copia física del framework.

Ambos sistemas incluyen un único Chromium completo, usado para navegación
visible y headless. Se elimina el headless shell adicional. No cambia el
comportamiento runtime del core respecto a 0.3.1.

## Tamaños medidos

| Artefacto | 0.3.1 | 0.3.2 | Reducción |
|---|---:|---:|---:|
| DMG macOS ARM64 | 843.656.435 bytes | 398.007.570 bytes | 52,82% |
| NSIS Windows x64 firmado | 538.859.184 bytes | 438.997.488 bytes | 18,53% |

El updater macOS mide 409.284.547 bytes; sus recursos instalados,
949.286.997 bytes (antes aproximadamente 1,9 GB). El runtime Windows instalado
mide 1.110.167.142 bytes. No hay archivos duplicados mayores de 10 MiB en
Windows ni headless shell adicional en ningún paquete. Chromium y Codex siguen
siendo los componentes principales: no se promete un instalador mínimo.

## Evidencia nativa

[Run 37817580921](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37817580921)
SUCCESS para ambos sistemas. Chromium real visible/headless y arranque desde
NSIS instalado / DMG montado y copiado PASS. Los manifiestos acreditan 4.579
recursos Windows y 4.606 Mac. Windows final se verifica nuevamente tras firma
Authenticode de desarrollo y timestamp RFC3161; la firma updater se genera
sobre esos bytes finales y exige versión 0.3.2.

Seis tests Rust, un fixture de enlaces de framework y diez tests agent-browser
PASS. La suite runtime de 0.3.1 es la baseline, no se volvió a ejecutar completa
porque este cambio modifica exclusivamente distribución. El fixture headed
local en Linux ARM bajo Xvfb tuvo timeout; las pruebas nativas en ambos
sistemas de distribución sí pasaron.

Core f9ee365f4142aefddf7f333f5c3a86752edabe84, tag desktop-core-v0.3.2.
Shell 06e479454159d509781d20538515e654d98d40a1, tag v0.3.2.

## Confianza Apple: bloqueo pendiente

La firma ad hoc pasa codesign deep/strict, pero la nueva prueba con cuarentena
de Chrome y Gatekeeper devuelve REJECTED, exit 3. El peso corregido no resuelve
esa política. Developer ID y notarización no están configurados. CUR-261
permanece abierto; no se acredita recuperación en el equipo del cliente ni se
recomienda retirar su cuarentena. Windows usa un certificado de desarrollo,
no una identidad comercial públicamente confiable.

## Canal público

Publicado como latest con nueve assets y tamaños exactos. Ambos aliases de
instalación responden HTTP 206 y el manifiesto público coincide con el
preparado. La descarga completa del DMG coincide por SHA-256. El plugin Tauri
real descargó y verificó firma, versión 0.3.2 y SHA-256 para ambos targets:
PASS. [Evidencia](release-0.3.2-updater.json). No se simula un salto de versión
ni se reinicia o actualiza la instalación local de Roberto.

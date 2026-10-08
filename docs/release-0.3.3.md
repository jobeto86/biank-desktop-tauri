# Biank Desktop 0.3.3 — apertura nativa del login

Rafa alcanzaba la pantalla de acceso en macOS 0.3.2, pero recibía un error de
ventanas emergentes. El login reservaba window.open about:blank, mientras el
bridge Tauri sólo enviaba al sistema los enlaces HTML externos. Faltaba conectar
la apertura programática de autorización al comando nativo existente.

Google y CBSuite reciben ahora auth.open del bridge, abren mediante open_external
y sondean después de aceptar la apertura. Ante un fallo recuperan los botones y
muestran un error del navegador del sistema. ChatGPT utiliza la misma apertura
cuando llega la URL, una vez por inicio; pintar una autorización pendiente no
abre ventanas. Conserva el enlace manual y el aviso ante fallo. El cliente web
mantiene su reserva síncrona de popup. No se cambia el proveedor ni se importan
cookies, sesiones o credenciales personales.

## Fuente y validación

Core 46b675223ebe8c7cb23062d5e0ad513f41d486f3, tag desktop-core-v0.3.3.
Shell d1b86f5e915b35a547482c43557406d9dff9194d, tag v0.3.3.
[Run 37825006414](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/37825006414)
SUCCESS para Windows x64 y macOS ARM64. El gate debug nativo requiere ahora que
la ventana Tauri invoque correctamente la apertura del navegador del sistema,
además de UI, bridge y motor. Chromium visible/headless y arranque desde NSIS
instalado / DMG montado y copiado PASS para ambos sistemas.

Pruebas locales: seis de regresión del login nativo, una de bridge, cinco de
broker del navegador, tres de cuenta web, una de ChatGPT web y seis Rust PASS.
TypeScript/Vite y cargo fmt check PASS. No se repite la suite completa runtime:
el cambio sólo conecta la UI de autorización con el comando nativo existente.

Integridad de recursos finales: 4.579 Windows y 4.606 Mac PASS. Windows final
con Authenticode de desarrollo y timestamp RFC3161 válido, luego se firma el
updater con versión 0.3.3. El DMG mide 397.994.914 bytes, el NSIS final
438.997.104 y el tar.gz updater Mac 409.284.426. Se conserva una única copia
física del framework Chromium y no se incluye headless shell adicional.

## Límites de la evidencia

La apertura nativa acredita que el OS acepta lanzar el navegador; no completa
OAuth ni certifica autenticación con cuentas reales de clientes. Ese resultado
requiere prueba de Rafa en su equipo. CUR-261 permanece abierto: Gatekeeper con
cuarentena de Chrome continúa REJECTED (exit 3), aun con firma ad hoc deep/strict
válida. No hay Developer ID/notarización. Windows conserva certificado de
desarrollo, no una identidad comercial de confianza pública.

## Canal público

[Publicado como latest](https://github.com/jobeto86/biank-desktop/releases/tag/v0.3.3)
con nueve assets y tamaños exactos. Ambos aliases responden HTTP 206; el
manifiesto público coincide. El DMG público descargado completo coincide por
SHA-256. El updater Tauri real descargó y verificó firma, versión 0.3.3 y SHA-256
para ambos targets: PASS. [Evidencia](release-0.3.3-updater.json).

Se notificó a Rafa por WhatsApp con vínculo directo de Mac, pendiente Apple y
solicitud de confirmar login; envío y sticker de agradecimiento verificados.
No se afirma lectura ni recuperación en su equipo.
La instalación viva de Roberto no se reinicia ni actualiza por esta publicación.

# Recuperación del perfil Electron en Windows — 2026-10-08

La prueba manual de Roberto muestra «No se pudo leer Local State de Electron»
en el arranque de Tauri. La rama de recuperación v10 encontró la llave sellada,
pero no pudo leer el perfil seleccionado. No se ha inspeccionado su filesystem.

El migrador anterior sólo resolvía APPDATA/Biank. La aplicación Electron histórica
usa productName Biank y package name biank-desktop, sin setPath(userData).
El cambio contempla ambas carpetas conocidas; una ruta explícita mediante
BIANK_ELECTRON_USER_DATA sigue siendo exclusiva y debe ser absoluta.

Cada candidato debe autenticar por AES-GCM la llave sellada existente después
de recuperar su llave Chromium mediante DPAPI. Una carpeta presente con llave
ajena no se adopta. Sin candidato válido, el arranque falla conservando los
archivos: no genera una llave nueva ni reemplaza Local State o material legado.
El llavero nativo sólo se escribe tras recuperar y validar la llave original.

Validación local Linux ARM64: 7 pruebas unitarias y 1 de firma PASS.
Regresiones: carpeta Biank ausente, carpeta Biank con llave ajena, recuperación
por biank-desktop, override exclusivo, rechazo DPAPI, material truncado y
conservación del archivo original. La prueba DPAPI real se incluye sólo en
Windows para el gate cargo test existente; no se ejecutó en este host Linux.

Pendiente: build y prueba nativa Windows, comprobar el perfil concreto de Roberto
y verificar reapertura con datos existentes. Este cambio no publica instalador
ni acredita recuperación del cliente; Pablo sigue pendiente de la prueba.

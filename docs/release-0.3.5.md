# Biank Desktop 0.3.5 — progreso visible durante el arranque

Mandato de Roberto: publicar la corrección y mostrar un splash animado que
informe de la actividad durante los respaldos y la preparación tras actualizar.

El shell anterior sólo mostraba HTML sin estilos y «Iniciando Biank». El core
ya emitía runtime-phase.json, pero el shell no lo consumía. Ahora se muestran
las fases del proceso hijo propio: respaldo, conteo de archivos comprobados,
verificación de la copia, preparación del perfil y apertura. Se muestra tiempo
transcurrido, sin prometer duración ni inventar porcentajes. El splash respeta
el tema del sistema y movimiento reducido. Un error real detiene la animación
y conserva el mensaje, sin confundirlo con actividad normal.

Se incluye la recuperación conservadora de Local State en ambas carpetas
históricas de Electron. La captura anterior indicaba un fallo de lectura; el
reporte posterior de arranque largo no demuestra por sí solo que ese fallo
fuera el mismo proceso. No se declara causa única ni recuperación de Pablo.

Validación local: 9 pruebas Rust y regresión JS PASS; revisión visual por
Chromium de respaldo, error, tema claro/oscuro y movimiento reducido PASS.
Las fases sólo se aceptan si el PID corresponde al hijo propio; texto mostrado
por fase acotado, sin rutas ni datos de bóveda. Se mantienen respaldo,
verificación criptográfica, aislamiento, timeout de arranque y canal updater.

Builds nativos y verificación pública se registran al terminar la publicación.

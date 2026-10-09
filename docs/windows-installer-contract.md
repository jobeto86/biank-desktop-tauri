# Windows installer: cierre seguro antes de reemplazar archivos

Mandato de Roberto, 2026-10-09: el instalador debe asegurarse de cerrar Biank y
su runtime antes de instalar y ser resiliente ante procesos que siguen activos.

Antes de copiar, el instalador pide cierre ordenado al shell de la instalación
destino. El shell usa el drain existente: trabajo activo impide el cierre y la
instalación. No aplica otra actualización descargada durante ese cierre.
La ventana oculta también cuenta como proceso vivo. Se espera un plazo acotado
y se comprueba todo proceso cuyo ejecutable pertenece a esa instalación,
incluidos Node, Codex, Chromium y cloudflared, y los archivos ejecutables/DLL.
No se terminan procesos por nombre ni procesos de otras instalaciones.

Si no puede inspeccionar, cerrar o acceder exclusivamente a los archivos,
aborta antes de copiar. No ofrece omitir archivos ni declara instalación sana.
Versiones antiguas que no admiten la petición requieren Salir de Biank; si
quedan procesos huérfanos, reiniciar Windows. Nunca se fuerza trabajo activo.
La guarda es previa; no promete inmunidad a fallos de disco o a un programa
externo que abra un archivo después de la comprobación.

Validación: contrato ejecutable en Windows con proceso Node sintético que
bloquea instalación, archivo bloqueado, y proceso fuera de la instalación.
No hay publicación de una nueva versión en este cambio.

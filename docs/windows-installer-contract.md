# Windows installer: cierre seguro antes de reemplazar archivos

Mandato de Roberto, 2026-10-09: el instalador debe asegurarse de cerrar Biank y
su runtime antes de instalar y ser resiliente ante procesos que siguen activos.
Revisión 2026-10-16: el instalador debe resolver solo los restos de versiones
anteriores (procesos huérfanos, instalaciones viejas) en vez de mandar a
reiniciar Windows.

Antes de copiar, el instalador pide cierre ordenado al shell de la instalación
destino. El shell usa el drain existente: trabajo activo impide el cierre. En
ese caso el instalador ofrece Reintentar/Cancelar; nunca termina un shell vivo.

Sin shell vivo, todo proceso cuyo ejecutable pertenece a esa instalación (Node,
Codex, Chromium, cloudflared) es huérfano: nadie lo drena y sólo retiene
archivos. Tras una gracia corta se termina por ruta, nunca por nombre; procesos
de otras instalaciones no se tocan. Después se comprueban `.exe`/`.dll` con
acceso exclusivo, reintentando unos segundos (antivirus, indexador). Si otro
programa los sigue reteniendo, se ofrece Reintentar/Cancelar y no se copia nada.

El desinstalador ejecuta la misma preparación antes de borrar, para no dejar
huérfanos que bloqueen la siguiente instalación.

Instalaciones heredadas: una entrada de desinstalación por usuario cuyo nombre
empieza por «Biank», con carpeta distinta a la destino y desinstalador dentro
de esa carpeta (el build Electron retirado), se cierra y se desinstala en
silencio. Conserva los datos de usuario, que el vault importa al arrancar. Si
falla, se informa y la instalación nueva continúa.

En modo silencioso (`/S`) cualquier negativa termina con código distinto de
cero. La guarda es previa; no promete inmunidad a fallos de disco o a un
programa externo que abra un archivo después de la comprobación.

Validación: contrato ejecutable en Windows con runtime huérfano que se termina,
shell ocupado que se preserva, archivo bloqueado, proceso fuera de la
instalación, e instalación Electron sintética retirada por su entrada de registro.

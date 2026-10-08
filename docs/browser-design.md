# Navegador del agente con shell Tauri

Estado: aprobado por Roberto el 2026-10-07 («Ok así lo haremos»). Sustituye la incrustación Electron de CP-A485 para Tauri.

## Recomendación

Tauri aloja la interfaz de Biank. El coordinador mantiene Chromium empaquetado,
iniciado bajo demanda en ventana propia, con perfil persistente del agente y
control mediante Playwright/MCP. Persona y agente interactúan con las mismas
páginas; no se duplican cookies en el webview del shell ni en el navegador personal.

El botón Navegador de Biank abre o enfoca la ventana del agente. Ocultar el
panel no cierra páginas. Reiniciar conserva la sesión. Los permisos y la
atribución de pestañas continúan ligados a la conversación autorizada.
Autorizaciones OAuth usan el navegador predeterminado, como establece CP-A517.

El spec canónico ya incorpora la adaptación de CP-A485 y CP-A466: ventana
Chromium del agente, con perfiles y permisos conservados. El modo explícito
headless sigue disponible; un cambio de modo incompatible conserva el trabajo
y requiere terminar las páginas anteriores.

## Motivo técnico

Tauri usa WebView2 en Windows, WKWebView en macOS y WebKitGTK en Linux.
La conexión CDP de Playwright usada en la integración vigente sólo admite
navegadores Chromium. Por tanto, usar el webview de la aplicación como navegador
del agente no conserva el mismo mecanismo de control en los tres sistemas.

Mantener Chromium separado conserva una versión conocida por Playwright y
evita conceder los permisos nativos del shell a páginas externas. Empaquetarlo
no equivale a mantener Electron: el shell, bandeja y ventanas de producto pasan
a Tauri; Chromium se inicia únicamente cuando la navegación lo necesita.

## Impacto que debe medirse

El instalador completo seguirá incluyendo Node, Codex, Chromium y demás
recursos. No se promete un instalador de 18 MB ni menos de 50 MB para la
workstation completa. Tauri puede reducir la duplicación del shell; el consumo
de una sesión activa del navegador debe medirse por separado.

## Aceptación antes de release

1. Instalación limpia sin Node, Chrome ni herramientas de desarrollo externas.
2. MCP escribe en un formulario y la persona observa y modifica esa misma página.
3. Reinicio conserva sesión y almacenamiento; agentes distintos no los comparten.
4. Pestañas y permisos respetan conversación, revocación y cierre canónico.
5. Enlaces externos no reciben bridge nativo, credenciales locales ni acceso al shell.
6. Windows y macOS verifican apertura, foco, navegación, descargas y cierre.

## Fuentes

- https://v2.tauri.app/reference/webview-versions/
- https://playwright.dev/docs/api/class-browsertype#browser-type-connect-over-cdp

# Biank Desktop (Tauri Edition)

> **Workstation Local-First de Agentes Autónomos**  
> Shell nativo ligero en Rust (Tauri v2) para sustituir el envoltorio de Electron, reduciendo drásticamente el consumo de memoria RAM y almacenamiento en estaciones de trabajo cliente.

---

## 🎯 Objetivo de la Edición Tauri

La versión original de Biank Desktop basada en Electron consumía entre 250 MB y 500 MB de RAM base al mantener Chromium embebido y un proceso Node.js dedicado por cada instancia. 

Esta edición reimplementa el shell de escritorio con **Tauri v2**:
1. **Huella de memoria <50 MB**: Utiliza el motor web nativo del sistema operativo (WebView2 en Windows 10/11, WKWebView en macOS, WebKitGTK en Linux).
2. **Mascota Marina transparente nativa**: Ventana flotante secundaria (`companion`) sin bordes ni marco, acelerada por hardware nativo.
3. **Bóveda segura (`keyring`)**: Cifrado y resguardo de la llave maestra mediante APIs criptográficas nativas del SO (DPAPI en Windows, Keychain en macOS, Secret Service en Linux) reemplazando `safeStorage` de Electron.
4. **Supervisor del Coordinador en Rust**: Supervisión de procesos hijos (`coordinator` backend) con ciclo de vida atado al shell principal.

---

## 🏗️ Arquitectura

```text
┌────────────────────────────────────────────────────────┐
│                   BIANK DESKTOP (TAURI)                │
│                                                        │
│  ┌───────────────────────┐   ┌──────────────────────┐  │
│  │     Main Window       │   │   Marina Companion   │  │
│  │   (Vite / Web UI)     │   │ (Transparent Window) │  │
│  └───────────▲───────────┘   └──────────▲───────────┘  │
│              │                          │              │
│              └────────────┬─────────────┘              │
│                           │ (IPC / Events)             │
│              ┌────────────▼─────────────┐              │
│              │    Tauri Core (Rust)     │              │
│              │ - Window & Tray Mgmt     │              │
│              │ - Vault (Keyring / DPAPI)│              │
│              │ - Process Supervisor     │              │
│              └────────────┬─────────────┘              │
│                           │ (Child Process / Loopback) │
│              ┌────────────▼─────────────┐              │
│              │  Coordinador TypeScript  │              │
│              │  (Sidecar Node Runtime)  │              │
│              └──────────────────────────┘              │
└────────────────────────────────────────────────────────┘
```

---

## 🚀 Desarrollo

### Requisitos
- **Node.js**: >= 20.x
- **Rust & Cargo**: >= 1.78.x
- **Dependencias de sistema**:
  - Windows: WebView2 Runtime (incluido en Windows 10/11).
  - macOS: Xcode Command Line Tools.
  - Linux: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`.

### Comandos
```bash
# Instalar dependencias
npm install

# Modo desarrollo
npm run tauri dev

# Compilar para producción (instaladores nativos)
npm run tauri build
```

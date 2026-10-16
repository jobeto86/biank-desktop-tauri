; Native guard (biank-desktop.exe --prepare-install): closes Biank, ends orphaned
; runtime processes, waits for its files and retires the legacy Electron build.
; No PowerShell: policies, Smart App Control or antivirus may block it.
; Exit 0 ready, 2 busy shell, 1 files locked. If the guard cannot run at all the
; installer continues with Tauri's own running-app check instead of blocking.
!macro BIANK_PREPARE MODE
  biank_prepare_${MODE}:
  DetailPrint "Cerrando Biank de forma segura..."
  nsExec::ExecToStack '"$PLUGINSDIR\biank-guard.exe" --prepare-install "$INSTDIR" ${MODE}'
  Pop $0
  Pop $1
  DetailPrint "$1"
  StrCmp $0 "0" biank_ready_${MODE}
  StrCmp $0 "2" biank_busy_${MODE}
  StrCmp $0 "1" biank_locked_${MODE}
  DetailPrint "Guarda de Biank no disponible ($0); se continua con la comprobacion estandar."
  Goto biank_ready_${MODE}
  biank_busy_${MODE}:
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Biank sigue abierto o está terminando un trabajo.$\n$\nEspera a que termine o usa Salir de Biank en la bandeja, y pulsa Reintentar." /SD IDCANCEL IDRETRY biank_prepare_${MODE}
  Goto biank_abort_${MODE}
  biank_locked_${MODE}:
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Otro programa está usando archivos de Biank.$\n$\n$1$\n$\nCierra ventanas del Explorador o programas abiertos en la carpeta de Biank y pulsa Reintentar. No se reemplazó ningún archivo." /SD IDCANCEL IDRETRY biank_prepare_${MODE}
  biank_abort_${MODE}:
  SetErrorLevel 1
  Quit
  biank_ready_${MODE}:
!macroend
!macro NSIS_HOOK_PREINSTALL
  InitPluginsDir
  ; The new binary is already in the package; NSIS stores identical data once.
  File /oname=$PLUGINSDIR\biank-guard.exe "${MAINBINARYSRCPATH}"
  !insertmacro BIANK_PREPARE install
!macroend
!macro NSIS_HOOK_PREUNINSTALL
  InitPluginsDir
  ; The installed binary belongs to this uninstaller's own version.
  CopyFiles /SILENT "$INSTDIR\${MAINBINARYNAME}.exe" "$PLUGINSDIR\biank-guard.exe"
  !insertmacro BIANK_PREPARE uninstall
!macroend

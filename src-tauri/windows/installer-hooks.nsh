!define BIANK_INSTALL_GUARD_PATH "${__FILEDIR__}\prepare-install.ps1"
; Closes Biank, ends orphaned runtime processes and waits for its files. A busy
; shell or a file held by another program offers Retry instead of a dead end.
!macro BIANK_PREPARE MODE
  InitPluginsDir
  File /oname=$PLUGINSDIR\biank-prepare-install.ps1 "${BIANK_INSTALL_GUARD_PATH}"
  biank_prepare_${MODE}:
  DetailPrint "Cerrando Biank de forma segura..."
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\biank-prepare-install.ps1" -InstallDir "$INSTDIR" -Mode ${MODE}'
  Pop $0
  Pop $1
  DetailPrint "$1"
  StrCmp $0 "0" biank_ready_${MODE}
  StrCmp $0 "2" 0 biank_locked_${MODE}
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Biank sigue abierto o está terminando un trabajo.$\n$\nEspera a que termine o usa Salir de Biank en la bandeja, y pulsa Reintentar." /SD IDCANCEL IDRETRY biank_prepare_${MODE}
  Goto biank_abort_${MODE}
  biank_locked_${MODE}:
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Otro programa está usando archivos de Biank.$\n$\nCierra ventanas del Explorador o programas abiertos en la carpeta de Biank y pulsa Reintentar. Si persiste, reinicia Windows. No se reemplazó ningún archivo." /SD IDCANCEL IDRETRY biank_prepare_${MODE}
  biank_abort_${MODE}:
  SetErrorLevel 1
  Quit
  biank_ready_${MODE}:
!macroend
!macro NSIS_HOOK_PREINSTALL
  !insertmacro BIANK_PREPARE install
!macroend
!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro BIANK_PREPARE uninstall
!macroend

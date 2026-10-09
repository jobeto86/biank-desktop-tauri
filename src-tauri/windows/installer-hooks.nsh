!define BIANK_INSTALL_GUARD_PATH "${__FILEDIR__}\prepare-install.ps1"
!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Preparando el cierre seguro de Biank..."
  InitPluginsDir
  File /oname=$PLUGINSDIR\biank-prepare-install.ps1 "${BIANK_INSTALL_GUARD_PATH}"
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\biank-prepare-install.ps1" -InstallDir "$INSTDIR"'
  Pop $0
  Pop $1
  StrCmp $0 "0" biank_preinstall_ready
  DetailPrint "$1"
  IfSilent +2
  MessageBox MB_OK|MB_ICONSTOP "Biank todavía tiene procesos activos o archivos bloqueados. Termina el trabajo y usa Salir de Biank; si persiste, reinicia Windows. La instalación se detuvo antes de reemplazar archivos."
  SetErrorLevel 1
  Quit
  biank_preinstall_ready:
!macroend

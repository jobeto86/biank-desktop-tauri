$ErrorActionPreference = 'Stop'
$nsis = 'C:/Program Files (x86)/NSIS/makensis.exe'
if (!(Test-Path $nsis)) {
  choco install nsis -y --no-progress
  if ($LASTEXITCODE -ne 0) { throw 'NSIS install failed' }
}
$hooks = (Resolve-Path 'src-tauri/windows/installer-hooks.nsh').Path
$output = Join-Path $env:RUNNER_TEMP 'biank-guard-harness.exe'
$destination = Join-Path $env:RUNNER_TEMP 'biank-guard-harness-target'
$script = Join-Path $env:RUNNER_TEMP 'biank-guard-harness.nsi'
@"
Unicode true
Name "Biank guard fixture"
OutFile "$output"
InstallDir "$destination"
RequestExecutionLevel user
!include "$hooks"
Section
  !insertmacro NSIS_HOOK_PREINSTALL
  FileOpen `$2 "`$INSTDIR\installed-proof.txt" w
  FileWrite `$2 "accepted"
  FileClose `$2
SectionEnd
"@ | Set-Content $script
New-Item -ItemType Directory -Path $destination -Force | Out-Null
& $nsis $script
if ($LASTEXITCODE -ne 0) { throw 'NSIS hook compilation failed' }
$result = Start-Process $output -ArgumentList '/S' -Wait -PassThru
if ($result.ExitCode -ne 0 -or !(Test-Path "$destination/installed-proof.txt")) { throw 'Clean installation guard failed' }
Remove-Item "$destination/installed-proof.txt"
$locked = [IO.File]::Open("$destination/blocked.dll",[IO.FileMode]::Create,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
  $result = Start-Process $output -ArgumentList '/S' -Wait -PassThru
  if ($result.ExitCode -eq 0 -or (Test-Path "$destination/installed-proof.txt")) { throw 'NSIS copied files despite blocked DLL' }
} finally { $locked.Dispose() }

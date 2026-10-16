param([string]$GuardExecutable = 'src-tauri/target/debug/biank-desktop.exe')
$ErrorActionPreference = 'Stop'
$nsis = 'C:/Program Files (x86)/NSIS/makensis.exe'
if (!(Test-Path $nsis)) {
  choco install nsis -y --no-progress
  if ($LASTEXITCODE -ne 0) { throw 'NSIS install failed' }
}
$guard = (Resolve-Path $GuardExecutable).Path
$hooks = (Resolve-Path 'src-tauri/windows/installer-hooks.nsh').Path
function New-Harness([string]$Name, [string]$Binary) {
  $output = Join-Path $env:RUNNER_TEMP "$Name.exe"
  $destination = Join-Path $env:RUNNER_TEMP "$Name-target"
  $script = Join-Path $env:RUNNER_TEMP "$Name.nsi"
@"
Unicode true
Name "Biank guard fixture"
OutFile "$output"
InstallDir "$destination"
RequestExecutionLevel user
!define MAINBINARYSRCPATH "$Binary"
!define MAINBINARYNAME "biank-desktop"
!include "$hooks"
Section
  !insertmacro NSIS_HOOK_PREINSTALL
  FileOpen `$2 "`$INSTDIR\installed-proof.txt" w
  FileWrite `$2 "accepted"
  FileClose `$2
  WriteUninstaller "`$INSTDIR\uninstall.exe"
SectionEnd
Section Uninstall
  !insertmacro NSIS_HOOK_PREUNINSTALL
  Delete "`$INSTDIR\uninstall.exe"
SectionEnd
"@ | Set-Content $script
  New-Item -ItemType Directory -Path "$destination/runtime/node" -Force | Out-Null
  & $nsis $script
  if ($LASTEXITCODE -ne 0) { throw 'NSIS hook compilation failed' }
  [pscustomobject]@{ Installer = $output; Target = $destination }
}
function Invoke-Harness($Harness) {
  Remove-Item "$($Harness.Target)/installed-proof.txt" -ErrorAction SilentlyContinue
  $result = Start-Process $Harness.Installer -ArgumentList '/S' -Wait -PassThru
  [pscustomobject]@{ Code = $result.ExitCode; Installed = (Test-Path "$($Harness.Target)/installed-proof.txt") }
}

$harness = New-Harness 'biank-guard-harness' $guard
$run = Invoke-Harness $harness
if ($run.Code -ne 0 -or !$run.Installed) { throw 'Clean installation guard failed' }

# A runtime process left by a removed or crashed version no longer dead-ends the install.
Copy-Item "$env:SystemRoot/System32/PING.EXE" "$($harness.Target)/runtime/node/node.exe"
$orphan = Start-Process "$($harness.Target)/runtime/node/node.exe" -ArgumentList '-n','600','127.0.0.1' -WindowStyle Hidden -PassThru
try {
  $run = Invoke-Harness $harness
  if ($run.Code -ne 0 -or !$run.Installed) { throw 'Orphan runtime blocked the installation' }
  if (!$orphan.HasExited) { throw 'Orphan runtime survived the installer' }
} finally { if (!$orphan.HasExited) { $orphan.Kill() } }

$locked = [IO.File]::Open("$($harness.Target)/runtime/node/blocked.dll",[IO.FileMode]::Create,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
  $run = Invoke-Harness $harness
  if ($run.Code -eq 0 -or $run.Installed) { throw 'NSIS copied files despite blocked DLL' }
} finally { $locked.Dispose() }
Remove-Item "$($harness.Target)/runtime/node/blocked.dll"

# A guard that cannot start (blocked, corrupt) must not block the installation.
$broken = Join-Path $env:RUNNER_TEMP 'biank-guard-broken.exe'
Set-Content -LiteralPath $broken -Value 'not an executable'
$run = Invoke-Harness (New-Harness 'biank-guard-unavailable' $broken)
if ($run.Code -ne 0 -or !$run.Installed) { throw 'Unavailable guard blocked the installation' }

Copy-Item $guard "$($harness.Target)/biank-desktop.exe"
$uninstall = Start-Process "$($harness.Target)/uninstall.exe" -ArgumentList '/S',"_?=$($harness.Target)" -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw "Uninstall guard failed: $($uninstall.ExitCode)" }
Write-Output 'PASS: clean install, orphan runtime ended, locked DLL blocks, unavailable guard does not block, uninstall guard runs'

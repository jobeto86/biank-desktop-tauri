param([string]$InstallDir, [switch]$DefineOnly)
$ErrorActionPreference = 'Stop'
function Get-BiankInstallProcesses([string]$Directory) {
    $base = [IO.Path]::GetFullPath($Directory).TrimEnd('\') + '\'
    @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
        $_.ExecutablePath -and $_.ExecutablePath.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)
    })
}
function Assert-BiankInstallFilesAvailable([string]$Directory) {
    if (!(Test-Path -LiteralPath $Directory)) { return }
    Get-ChildItem -LiteralPath $Directory -File -Recurse -ErrorAction Stop | Where-Object {
        $_.Extension -in '.exe','.dll'
    } | ForEach-Object {
        $stream = [IO.File]::Open($_.FullName, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
        $stream.Dispose()
    }
}
function Prepare-BiankInstall([string]$Directory, [int]$WaitSeconds = 30) {
    if (!$Directory -or ![IO.Path]::IsPathRooted($Directory)) { throw 'Installation path must be absolute' }
    $processes = @(Get-BiankInstallProcesses $Directory)
    $shell = Join-Path $Directory 'biank-desktop.exe'
    if ($processes | Where-Object { $_.ExecutablePath -ieq $shell }) {
        # Single-instance IPC asks the existing shell to drain; never taskkill /F.
        $request = Start-Process -FilePath $shell -ArgumentList '--installer-close' -PassThru
        if (!$request.WaitForExit(10000)) { throw 'The close request did not finish' }
    }
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    do {
        $processes = @(Get-BiankInstallProcesses $Directory)
        if ($processes.Count -eq 0) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($processes.Count) { throw 'The installation still has active processes' }
    Assert-BiankInstallFilesAvailable $Directory
    # Recheck after the file probe; fail closed if another instance reopened.
    if (@(Get-BiankInstallProcesses $Directory).Count) { throw 'Biank reopened during preparation' }
}
if (!$DefineOnly) {
    try { Prepare-BiankInstall $InstallDir; exit 0 }
    catch { Write-Output 'Biank no pudo cerrar todos sus procesos o liberar sus archivos. Termina el trabajo y usa Salir de Biank; si persiste, reinicia Windows antes de instalar. No se reemplazaron archivos.'; exit 1 }
}

param([string]$InstallDir, [ValidateSet('install','uninstall')][string]$Mode = 'install', [switch]$DefineOnly)
$ErrorActionPreference = 'Stop'
# Exit codes for the NSIS hook: 0 ready, 2 the shell is busy (TimeoutException, retryable), 1 anything else.
function Get-BiankInstallProcesses([string]$Directory) {
    $base = [IO.Path]::GetFullPath($Directory).TrimEnd('\') + '\'
    @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
        $_.ProcessId -ne $PID -and $_.ExecutablePath -and $_.ExecutablePath.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)
    })
}
function Wait-BiankProcessesGone([string]$Directory, [int]$Seconds, [scriptblock]$Filter = { $true }) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $left = @(Get-BiankInstallProcesses $Directory | Where-Object $Filter)
        if ($left.Count -eq 0) { return @() }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    $left
}
function Stop-BiankOrphans([string]$Directory, [int]$GraceSeconds) {
    # Without a live shell nobody drains this runtime: node, codex, cloudflared and
    # Chromium left behind by a crash or an uninstall only hold files. Selected by
    # executable path, never by name, so other installations are untouched.
    $left = @(Wait-BiankProcessesGone $Directory $GraceSeconds)
    foreach ($process in $left) { Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue }
    if (@(Wait-BiankProcessesGone $Directory 10).Count) { throw 'Orphan Biank processes survived termination' }
}
function Test-BiankInstallFilesAvailable([string]$Directory) {
    if (!(Test-Path -LiteralPath $Directory)) { return $true }
    try {
        Get-ChildItem -LiteralPath $Directory -File -Recurse -ErrorAction Stop | Where-Object {
            $_.Extension -in '.exe','.dll'
        } | ForEach-Object {
            $stream = [IO.File]::Open($_.FullName, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
            $stream.Dispose()
        }
        $true
    } catch { $false }
}
function Wait-BiankInstallFilesAvailable([string]$Directory, [int]$Seconds) {
    # Antivirus and indexers hold freshly touched binaries for a moment.
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    while (!(Test-BiankInstallFilesAvailable $Directory)) {
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Installation files remain locked' }
        Start-Sleep -Milliseconds 500
    }
}
function Prepare-BiankInstall([string]$Directory, [int]$WaitSeconds = 30, [int]$GraceSeconds = 5, [int]$FileSeconds = 15) {
    if (!$Directory -or ![IO.Path]::IsPathRooted($Directory)) { throw 'Installation path must be absolute' }
    $shell = Join-Path $Directory 'biank-desktop.exe'
    $isShell = { $_.ExecutablePath -ieq $shell }
    if (@(Get-BiankInstallProcesses $Directory | Where-Object $isShell).Count) {
        # Single-instance IPC asks the existing shell to drain; active work keeps it open.
        $request = Start-Process -FilePath $shell -ArgumentList '--installer-close' -PassThru
        [void]$request.WaitForExit(10000)
        if (@(Wait-BiankProcessesGone $Directory $WaitSeconds $isShell).Count) {
            throw [TimeoutException]::new('Biank is still running')
        }
    }
    Stop-BiankOrphans $Directory $GraceSeconds
    Wait-BiankInstallFilesAvailable $Directory $FileSeconds
    # Recheck after the file probe; fail closed if another instance reopened.
    if (@(Get-BiankInstallProcesses $Directory).Count) { throw [TimeoutException]::new('Biank reopened during preparation') }
}
function Get-BiankLegacyInstalls([string]$Directory) {
    # The retired Electron build registered its own uninstall entry and folder, so
    # it survives next to this installation. Same family, different location.
    $target = [IO.Path]::GetFullPath($Directory).TrimEnd('\')
    # Per-user only, like both installers: a machine-wide entry would need UAC mid-update.
    foreach ($root in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall')) {
        if (!(Test-Path $root)) { continue }
        Get-ChildItem $root -ErrorAction SilentlyContinue | ForEach-Object {
            $entry = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
            if (!$entry -or $_.PSChildName -eq 'Biank' -or "$($entry.DisplayName)" -notmatch '^biank' -or !$entry.InstallLocation) { return }
            $location = [IO.Path]::GetFullPath("$($entry.InstallLocation)").TrimEnd('\')
            if ($location -ieq $target -or !(Test-Path -LiteralPath $location)) { return }
            $command = if ($entry.QuietUninstallString) { "$($entry.QuietUninstallString)" } else { "$($entry.UninstallString) /S" }
            if ($command -notmatch '^\s*"([^"]+)"\s*(.*)$') { return }
            $uninstaller = [IO.Path]::GetFullPath($Matches[1])
            # Only an uninstaller shipped inside that installation is ever executed.
            if (!$uninstaller.StartsWith($location + '\', [StringComparison]::OrdinalIgnoreCase) -or !(Test-Path -LiteralPath $uninstaller)) { return }
            [pscustomobject]@{ Key = $_.PSPath; Name = "$($entry.DisplayName)"; Location = $location; Uninstaller = $uninstaller; Arguments = $Matches[2] }
        }
    }
}
function Remove-BiankLegacyInstall($Legacy) {
    foreach ($process in @(Get-BiankInstallProcesses $Legacy.Location)) {
        $live = Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue
        if ($live) { [void]$live.CloseMainWindow() }
    }
    Stop-BiankOrphans $Legacy.Location 10
    # The silent uninstaller keeps user data, which the vault imports on first start.
    $start = @{ FilePath = $Legacy.Uninstaller; PassThru = $true; Wait = $true }
    if ($Legacy.Arguments.Trim()) { $start.ArgumentList = $Legacy.Arguments.Trim() }
    $run = Start-Process @start
    $deadline = [DateTime]::UtcNow.AddSeconds(60)
    while ((Test-Path -LiteralPath $Legacy.Key) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 500 }
    if (Test-Path -LiteralPath $Legacy.Key) { throw "Legacy uninstall did not finish ($($run.ExitCode))" }
}
if (!$DefineOnly) {
    try {
        Prepare-BiankInstall $InstallDir
        if ($Mode -eq 'install') {
            foreach ($legacy in @(Get-BiankLegacyInstalls $InstallDir)) {
                # A leftover old version never blocks the new one; it is reported instead.
                try { Remove-BiankLegacyInstall $legacy; Write-Output "Version anterior retirada: $($legacy.Name)" }
                catch { Write-Output "No se pudo retirar $($legacy.Name): $($_.Exception.Message)" }
            }
        }
        exit 0
    } catch [TimeoutException] {
        Write-Output 'Biank sigue abierto o terminando un trabajo.'
        exit 2
    } catch {
        Write-Output "Biank no pudo liberar sus archivos: $($_.Exception.Message)"
        exit 1
    }
}

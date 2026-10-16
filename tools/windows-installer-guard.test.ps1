param([string]$NodeExecutable = (Get-Command node).Source)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../src-tauri/windows/prepare-install.ps1" -DefineOnly
$root = Join-Path ([IO.Path]::GetTempPath()) ('biank-install-guard-' + [guid]::NewGuid())
$inside = Join-Path $root 'installed'
$outside = Join-Path $root 'unrelated'
$legacyDir = Join-Path $root 'legacy-electron'
$legacyKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\biank-guard-test-' + [guid]::NewGuid()
New-Item -ItemType Directory -Path $inside,$outside,$legacyDir | Out-Null
$orphan = $null; $other = $null; $shell = $null; $locked = $null; $legacyApp = $null
function Assert-Refused([scriptblock]$Action, [type]$Expected = [Exception]) {
    $refused = $false
    try { & $Action } catch { if (!($_.Exception -is $Expected)) { throw "Wrong refusal: $($_.Exception.GetType().Name)" }; $refused = $true }
    if (!$refused) { throw 'Unsafe installation was accepted' }
}
try {
    Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $inside 'node.exe')
    Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $outside 'node.exe')
    $fixture = Join-Path $root 'idle.js'
    Set-Content -LiteralPath $fixture -Value 'setInterval(()=>{},1000);'
    $other = Start-Process -FilePath (Join-Path $outside 'node.exe') -ArgumentList $fixture -PassThru
    Prepare-BiankInstall $inside -WaitSeconds 1 -GraceSeconds 1 -FileSeconds 1
    if ($other.HasExited) { throw 'An unrelated process was interrupted' }

    # Runtime left behind without its shell: the installer ends it and proceeds.
    $orphan = Start-Process -FilePath (Join-Path $inside 'node.exe') -ArgumentList $fixture -PassThru
    Prepare-BiankInstall $inside -WaitSeconds 1 -GraceSeconds 1 -FileSeconds 5
    if (!$orphan.WaitForExit(5000)) { throw 'Orphan runtime blocked the installation' }
    if ($other.HasExited) { throw 'An unrelated process was interrupted' }

    # A live shell that does not drain (active work) is never killed: retryable refusal.
    $shellPath = Join-Path $inside 'biank-desktop.exe'
    Copy-Item -LiteralPath $NodeExecutable -Destination $shellPath
    $shell = Start-Process -FilePath $shellPath -ArgumentList $fixture -PassThru
    Assert-Refused { Prepare-BiankInstall $inside -WaitSeconds 1 -GraceSeconds 1 -FileSeconds 1 } ([TimeoutException])
    if ($shell.HasExited) { throw 'A busy shell was force-killed' }
    # Only the test owns these synthetic processes and may terminate them.
    $shell.Kill(); $shell.WaitForExit(); $shell = $null

    $file = Join-Path $inside 'locked.dll'
    [IO.File]::WriteAllText($file,'synthetic fixture, unchanged')
    $locked = [IO.File]::Open($file,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    Assert-Refused { Prepare-BiankInstall $inside -WaitSeconds 1 -GraceSeconds 1 -FileSeconds 1 }
    $locked.Dispose(); $locked = $null
    if ([IO.File]::ReadAllText($file) -ne 'synthetic fixture, unchanged') { throw 'File was modified' }
    Prepare-BiankInstall $inside -WaitSeconds 1 -GraceSeconds 1 -FileSeconds 1
    Assert-Refused { Prepare-BiankInstall 'relative-path' -WaitSeconds 1 }

    # Legacy Electron install: detected by its uninstall entry, closed, uninstalled silently.
    Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $legacyDir 'Biank Desktop.exe')
    $uninstaller = Join-Path $legacyDir 'Uninstall biank-desktop.exe'
    Copy-Item -LiteralPath $NodeExecutable -Destination $uninstaller
    $uninstallScript = Join-Path $legacyDir 'uninstall.js'
    $regPath = $legacyKey -replace '^HKCU:\\','HKCU\'
    Set-Content -LiteralPath $uninstallScript -Value "require('child_process').execFileSync('reg',['delete',process.argv[2],'/f']);"
    New-Item -Path $legacyKey -Force | Out-Null
    New-ItemProperty -Path $legacyKey -Name DisplayName -Value 'biank-desktop 0.2.31' | Out-Null
    New-ItemProperty -Path $legacyKey -Name InstallLocation -Value $legacyDir | Out-Null
    New-ItemProperty -Path $legacyKey -Name QuietUninstallString -Value "`"$uninstaller`" `"$uninstallScript`" `"$regPath`"" | Out-Null
    $legacyApp = Start-Process -FilePath (Join-Path $legacyDir 'Biank Desktop.exe') -ArgumentList $fixture -PassThru
    $found = @(Get-BiankLegacyInstalls $inside | Where-Object { $_.Key -like "*$(Split-Path $legacyKey -Leaf)" })
    if ($found.Count -ne 1) { throw 'Legacy installation was not detected' }
    if (@(Get-BiankLegacyInstalls $legacyDir | Where-Object { $_.Key -like "*$(Split-Path $legacyKey -Leaf)" }).Count) { throw 'Target installation treated as legacy' }
    Remove-BiankLegacyInstall $found[0]
    if (!$legacyApp.WaitForExit(5000)) { throw 'Legacy app kept running' }
    if (Test-Path $legacyKey) { throw 'Legacy uninstall entry survived' }
    if ($other.HasExited) { throw 'An unrelated process was interrupted' }
    Write-Output 'PASS: orphan runtime ended, busy shell preserved, unrelated runtime survives, locked file blocks, originals preserved, legacy install retired'
} finally {
    if ($locked) { $locked.Dispose() }
    foreach ($process in @($orphan,$other,$shell,$legacyApp)) {
        if ($process -and !$process.HasExited) { $process.Kill(); $process.WaitForExit() }
    }
    if (Test-Path $legacyKey) { Remove-Item $legacyKey -Force }
    Remove-Item -LiteralPath $root -Recurse -Force
}

param([string]$NodeExecutable = (Get-Command node).Source)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../src-tauri/windows/prepare-install.ps1" -DefineOnly
$root = Join-Path ([IO.Path]::GetTempPath()) ('biank-install-guard-' + [guid]::NewGuid())
$inside = Join-Path $root 'installed'
$outside = Join-Path $root 'unrelated'
New-Item -ItemType Directory -Path $inside,$outside | Out-Null
$child = $null; $other = $null; $locked = $null
function Assert-Refused([scriptblock]$Action) {
    $refused = $false
    try { & $Action } catch { $refused = $true }
    if (!$refused) { throw 'Unsafe installation was accepted' }
}
try {
    Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $inside 'node.exe')
    Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $outside 'node.exe')
    $fixture = Join-Path $root 'idle.js'
    Set-Content -LiteralPath $fixture -Value 'setInterval(()=>{},1000);'
    $other = Start-Process -FilePath (Join-Path $outside 'node.exe') -ArgumentList $fixture -PassThru
    Prepare-BiankInstall $inside -WaitSeconds 1
    if ($other.HasExited) { throw 'An unrelated process was interrupted' }
    $child = Start-Process -FilePath (Join-Path $inside 'node.exe') -ArgumentList $fixture -PassThru
    Assert-Refused { Prepare-BiankInstall $inside -WaitSeconds 1 }
    if ($child.HasExited) { throw 'An active process was force-killed' }
    # Only the test owns these synthetic processes and may terminate them.
    $child.Kill(); $child.WaitForExit(); $child = $null
    $file = Join-Path $inside 'locked.dll'
    [IO.File]::WriteAllText($file,'synthetic fixture, unchanged')
    $locked = [IO.File]::Open($file,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    Assert-Refused { Prepare-BiankInstall $inside -WaitSeconds 1 }
    $locked.Dispose(); $locked = $null
    if ([IO.File]::ReadAllText($file) -ne 'synthetic fixture, unchanged') { throw 'File was modified' }
    Prepare-BiankInstall $inside -WaitSeconds 1
    Assert-Refused { Prepare-BiankInstall 'relative-path' -WaitSeconds 1 }
    Write-Output 'PASS: live runtime blocks, unrelated runtime survives, locked file blocks, originals preserved, released installation accepted'
} finally {
    if ($locked) { $locked.Dispose() }
    foreach ($process in @($child,$other)) {
        if ($process -and !$process.HasExited) { $process.Kill(); $process.WaitForExit() }
    }
    Remove-Item -LiteralPath $root -Recurse -Force
}

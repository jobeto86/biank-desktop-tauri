$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-launch-diagnostic.ps1')
$marker = 'PRIVATE_FIXTURE_PATH_AND_ERROR_TEXT'
$native = [System.ComponentModel.Win32Exception]::new(5, $marker)
$wrapper = [InvalidOperationException]::new($marker, $native)
foreach ($exception in @($native, $wrapper)) {
    $record = [System.Management.Automation.ErrorRecord]::new(
        $exception, $marker, [System.Management.Automation.ErrorCategory]::PermissionDenied, $marker)
    $record.Exception.Data[$marker] = $marker
    $result = Get-Cur260LaunchFailure -Record $record
    $json = $result | ConvertTo-Json -Compress
    if ($result.win32_code -ne 5 -or $result.category -ne 'PermissionDenied') {
        throw 'Native launch category/code lost through wrapper'
    }
    if ($json.Contains($marker) -or $result.Count -ne 5) {
        throw 'Private error data escaped projection'
    }
}
$record = [System.Management.Automation.ErrorRecord]::new(
    [InvalidOperationException]::new($marker), $marker,
    [System.Management.Automation.ErrorCategory]::InvalidOperation, $marker)
$result = Get-Cur260LaunchFailure -Record $record
if ($null -ne $result.win32_code -or $result.category -ne 'InvalidOperation') {
    throw 'Non-native error incorrectly attributed to Win32'
}
foreach ($wrappers in @(7, 8)) {
    $exception = [System.ComponentModel.Win32Exception]::new(5, $marker)
    for ($i = 0; $i -lt $wrappers; $i++) {
        $exception = [InvalidOperationException]::new($marker, $exception)
    }
    $record = [System.Management.Automation.ErrorRecord]::new(
        $exception, $marker, [System.Management.Automation.ErrorCategory]::InvalidOperation, $marker)
    $result = Get-Cur260LaunchFailure -Record $record
    if (($wrappers -eq 7 -and $result.win32_code -ne 5) -or
        ($wrappers -eq 8 -and $null -ne $result.win32_code)) {
        throw 'Bounded traversal boundary incorrect'
    }
    if (($result | ConvertTo-Json -Compress).Contains($marker)) {
        throw 'Private wrapped error data escaped projection'
    }
}
Add-Type @'
using System;
using System.Management.Automation;
public sealed class Cur260RecordException : Exception, IContainsErrorRecord {
    public ErrorRecord ErrorRecord { get; set; }
    public Cur260RecordException(string message) : base(message) {}
}
'@
$linked = [Cur260RecordException]::new($marker)
$linked.ErrorRecord = [System.Management.Automation.ErrorRecord]::new(
    $native, $marker, [System.Management.Automation.ErrorCategory]::PermissionDenied, $marker)
$record = [System.Management.Automation.ErrorRecord]::new(
    $linked, 'NativeCommandFailed,' + $marker,
    [System.Management.Automation.ErrorCategory]::ResourceUnavailable, $marker)
$result = Get-Cur260LaunchFailure -Record $record
if ($result.win32_code -ne 5 -or !$result.native_command_failed -or
    $result.exception_kind -ne 'Other' -or ($result | ConvertTo-Json -Compress).Contains($marker)) {
    throw 'Nested ErrorRecord lost native cause or leaked private data'
}
$linked.ErrorRecord = $record # Deliberate self cycle.
$result = Get-Cur260LaunchFailure -Record $record
if ($null -ne $result.win32_code) { throw 'Cyclic record fabricated native cause' }
foreach ($wrappers in @(7, 8)) {
    $exception = $native
    for ($i = 0; $i -lt $wrappers; $i++) {
        $next = [Cur260RecordException]::new($marker)
        $next.ErrorRecord = [System.Management.Automation.ErrorRecord]::new(
            $exception, $marker, [System.Management.Automation.ErrorCategory]::ResourceUnavailable, $marker)
        $exception = $next
    }
    $record = [System.Management.Automation.ErrorRecord]::new(
        $exception, 'NativeCommandFailedSuffix,' + $marker,
        [System.Management.Automation.ErrorCategory]::ResourceUnavailable, $marker)
    $result = Get-Cur260LaunchFailure -Record $record
    if (($wrappers -eq 7 -and $result.win32_code -ne 5) -or
        ($wrappers -eq 8 -and $null -ne $result.win32_code) -or $result.native_command_failed -or
        ($result | ConvertTo-Json -Compress).Contains($marker)) {
        throw 'Nested record depth, exact error identifier or sanitization failed'
    }
}
$record = [System.Management.Automation.ErrorRecord]::new(
    [System.Management.Automation.RuntimeException]::new($marker), $marker,
    [System.Management.Automation.ErrorCategory]::ResourceUnavailable, $marker)
$result = Get-Cur260LaunchFailure -Record $record
if ($result.exception_kind -ne 'RuntimeException' -or $result.native_command_failed -or
    $null -ne $result.win32_code -or ($result | ConvertTo-Json -Compress).Contains($marker)) {
    throw 'Runtime ErrorRecord classification or sanitization failed'
}
Write-Output 'PASS: bounded nested records and cycles; private text excluded. Windows execution NOT_RUN.'

$missing = Join-Path ([System.IO.Path]::GetTempPath()) ($marker + [guid]::NewGuid().ToString('N'))
$probe = Get-Cur260DirectLaunchProbe -FilePath $missing
if ($probe.outcome -ne 'start_failed' -or !$probe.cleanup_confirmed -or
    $null -eq $probe.win32_code -or $probe.Count -ne 5 -or
    ($probe | ConvertTo-Json -Compress).Contains($marker)) {
    throw 'Direct launch failure lost native cause, cleanup or sanitization'
}
$self = (Get-Process -Id $PID).Path
$probe = Get-Cur260DirectLaunchProbe -FilePath $self
if ($probe.outcome -ne 'exited' -or $probe.exit_code -ne 0 -or
    $probe.node22_version -or !$probe.cleanup_confirmed -or
    $null -ne $probe.win32_code -or ($probe | ConvertTo-Json -Compress).Contains($self)) {
    throw 'Direct launch success, cleanup or minimal projection failed'
}
Write-Output 'PASS: direct Process.Start success/failure fixtures; Windows restricted-token probe NOT_RUN.'
if ($IsLinux) {
    $fixture = Join-Path ([System.IO.Path]::GetTempPath()) ('cur260-probe-' + [guid]::NewGuid().ToString('N'))
    try {
        $null = New-Item -ItemType Directory -Path $fixture
        $slow = Join-Path $fixture 'slow-version'
        [System.IO.File]::WriteAllText($slow, "#!/bin/sh`nexec sleep 30`n")
        [System.IO.File]::SetUnixFileMode($slow,
            [System.IO.UnixFileMode]::UserRead -bor [System.IO.UnixFileMode]::UserWrite -bor [System.IO.UnixFileMode]::UserExecute)
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        $probe = Get-Cur260DirectLaunchProbe -FilePath $slow
        $clock.Stop()
        if ($probe.outcome -ne 'timeout' -or !$probe.cleanup_confirmed -or
            $null -ne $probe.exit_code -or $probe.node22_version -or
            $clock.Elapsed.TotalSeconds -gt 9 -or ($probe | ConvertTo-Json -Compress).Contains($fixture)) {
            throw 'Direct launch timeout did not bound and clean its owned child'
        }
    } finally { Remove-Item -LiteralPath $fixture -Recurse -Force }
    Write-Output 'PASS: Linux owned slow-process timeout and cleanup; Windows timeout NOT_RUN.'
}

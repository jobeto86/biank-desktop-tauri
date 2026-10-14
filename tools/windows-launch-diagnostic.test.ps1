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

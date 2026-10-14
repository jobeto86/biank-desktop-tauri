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
    if ($json.Contains($marker) -or $result.Count -ne 3) {
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
Write-Output 'PASS: native/wrapped codes preserved; private text excluded; non-native cause unknown. Windows execution NOT_RUN.'

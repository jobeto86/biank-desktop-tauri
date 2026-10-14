# Pure projection: never retain exception messages, paths, command text or data.
function Get-Cur260LaunchFailure {
    param([Parameter(Mandatory)][System.Management.Automation.ErrorRecord]$Record)
    $category = $Record.CategoryInfo.Category
    $result = [ordered]@{
        category = if ([Enum]::IsDefined([System.Management.Automation.ErrorCategory], $category)) {
            $category.ToString()
        } else { 'NotSpecified' }
        hresult = [int]$Record.Exception.HResult
        win32_code = $null
        exception_kind = switch ($Record.Exception.GetType().FullName) {
            'System.Management.Automation.ApplicationFailedException' { 'ApplicationFailedException' }
            'System.ComponentModel.Win32Exception' { 'Win32Exception' }
            'System.Management.Automation.RuntimeException' { 'RuntimeException' }
            default { 'Other' }
        }
        native_command_failed = ($Record.FullyQualifiedErrorId -split ',')[0] -ceq 'NativeCommandFailed'
    }
    # ErrorRecord can point back to its owner. Bound both depth and total work.
    $seen = [System.Collections.Generic.HashSet[System.Exception]]::new()
    $queue = [System.Collections.Generic.Queue[object]]::new()
    $queue.Enqueue(@{ exception = $Record.Exception; depth = 0 })
    while ($queue.Count -gt 0 -and $seen.Count -lt 16) {
        $entry = $queue.Dequeue()
        $exception = $entry.exception
        if ($null -eq $exception -or $entry.depth -ge 8 -or !$seen.Add($exception)) { continue }
        if ($exception -is [System.ComponentModel.Win32Exception]) {
            $result.win32_code = [int]$exception.NativeErrorCode
            break
        }
        $queue.Enqueue(@{ exception = $exception.InnerException; depth = $entry.depth + 1 })
        if ($exception -is [System.Management.Automation.IContainsErrorRecord]) {
            $nested = $exception.ErrorRecord
            if ($null -ne $nested) {
                $queue.Enqueue(@{ exception = $nested.Exception; depth = $entry.depth + 1 })
            }
        }
    }
    return $result
}

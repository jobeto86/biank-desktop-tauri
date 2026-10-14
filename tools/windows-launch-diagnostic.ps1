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
    }
    $exception = $Record.Exception
    for ($depth = 0; $depth -lt 8 -and $null -ne $exception; $depth++) {
        if ($exception -is [System.ComponentModel.Win32Exception]) {
            $result.win32_code = [int]$exception.NativeErrorCode
            break
        }
        $exception = $exception.InnerException
    }
    return $result
}

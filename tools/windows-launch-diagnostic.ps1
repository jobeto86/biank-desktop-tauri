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

# Diagnostic only after the original PowerShell invocation failed. Never upgrades
# that failure to PASS; starts the same resolved binary with --version only.
function Get-Cur260DirectLaunchProbe {
    param([Parameter(Mandatory)][string]$FilePath)
    $result = [ordered]@{ outcome = 'start_failed'; exit_code = $null;
        node22_version = $false; win32_code = $null; cleanup_confirmed = $false }
    $process = [System.Diagnostics.Process]::new()
    $started = $false
    try {
        $process.StartInfo.FileName = $FilePath
        $process.StartInfo.ArgumentList.Add('--version')
        $process.StartInfo.UseShellExecute = $false
        $process.StartInfo.CreateNoWindow = $true
        $process.StartInfo.RedirectStandardOutput = $true
        $process.StartInfo.RedirectStandardError = $true
        $started = $process.Start()
        if ($started) {
            # Capture at most 64 characters; excess output can only time out.
            # Never accumulate an unbounded output/error string in memory.
            $buffer = [char[]]::new(64)
            $stdout = $process.StandardOutput.ReadAsync($buffer, 0, $buffer.Length)
            if ($process.WaitForExit(5000)) {
                $result.outcome = 'exited'
                $result.exit_code = $process.ExitCode
                # Do not wait indefinitely for pipes inherited by grandchildren.
                if ($stdout.Wait(1000)) {
                    $result.node22_version = $process.ExitCode -eq 0 -and
                        [string]::new($buffer, 0, $stdout.Result) -match '^v22\.'
                }
            } else { $result.outcome = 'timeout' }
        }
    } catch {
        $result.outcome = if ($started) { 'probe_failed' } else { 'start_failed' }
        $result.win32_code = (Get-Cur260LaunchFailure -Record $_).win32_code
    } finally {
        try {
            if ($started -and !$process.HasExited) {
                # This handle identifies only the diagnostic child we just started.
                $process.Kill()
                $null = $process.WaitForExit(2000)
            }
            $result.cleanup_confirmed = !$started -or $process.HasExited
        } catch { $result.cleanup_confirmed = $false }
        try { $process.Dispose() } catch { $result.cleanup_confirmed = $false }
    }
    return $result
}

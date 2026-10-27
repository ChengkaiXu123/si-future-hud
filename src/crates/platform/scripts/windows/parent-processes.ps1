# Win32_Process reports command lines and the owner's SID without reading any
# process environment. A reused parent PID must not be newer than its child.
# https://learn.microsoft.com/windows/win32/cimwin32prov/win32-process
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$diagnostics = $env:AGENT_HUD_ANCESTRY_DIAGNOSTICS -eq '1'
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$count = 0
function Report-Stage([string]$stage) {
    if ($diagnostics) {
        [Console]::Error.WriteLine("agent-hud-ancestry: stage=$stage ms=$($clock.ElapsedMilliseconds) count=$count")
        [Console]::Error.Flush()
    }
}
# Avoid ConvertTo-Json's first-use module loading inside this bounded probe.
# These two string fields need only JSON's quote, backslash and control escapes.
function Json-String([string]$value) {
    '"' + [regex]::Replace($value, '["\\\x00-\x1f]', {
        param($match)
        '\u{0:x4}' -f [int][char]$match.Value
    }) + '"'
}
trap { Report-Stage 'script-error'; exit 1 }
Report-Stage 'started'
$owner = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
Report-Stage 'child-query'
$child = Get-CimInstance Win32_Process -Filter "ProcessId = $([uint32]$env:AGENT_HUD_CALLER_PID)"
Report-Stage 'child-ready'
$seen = [System.Collections.Generic.HashSet[uint32]]::new()
for ($count = 0; $count -lt 16 -and $null -ne $child; $count++) {
    $parentId = [uint32]$child.ParentProcessId
    if ($parentId -eq 0 -or -not $seen.Add($parentId)) { Report-Stage 'chain-end'; break }
    Report-Stage 'parent-query'
    try { $parent = Get-CimInstance Win32_Process -Filter "ProcessId = $parentId" }
    catch { Report-Stage 'parent-error'; break }
    Report-Stage 'parent-ready'
    if ($null -eq $parent -or $null -eq $parent.CreationDate -or
        $null -eq $child.CreationDate -or $parent.CreationDate -gt $child.CreationDate) { Report-Stage 'parent-invalid'; break }
    Report-Stage 'owner-query'
    try { $identity = Invoke-CimMethod -InputObject $parent -MethodName GetOwnerSid }
    catch { Report-Stage 'owner-error'; break }
    Report-Stage 'owner-ready'
    if ($identity.ReturnValue -ne 0 -or $identity.Sid -ne $owner -or
        [string]::IsNullOrEmpty($parent.ExecutablePath) -or
        [string]::IsNullOrEmpty($parent.CommandLine)) { Report-Stage 'owner-or-fields-invalid'; break }
    $record = '{"executable":' + (Json-String $parent.ExecutablePath) +
        ',"command":' + (Json-String $parent.CommandLine) + '}'
    [Console]::Out.WriteLine($record)
    [Console]::Out.Flush()
    Report-Stage 'record-written'
    $child = $parent
}
Report-Stage 'finished'

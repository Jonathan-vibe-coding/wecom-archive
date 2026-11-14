param(
    [Parameter(Mandatory=$true)][string]$RunRoot,
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$Profile,
    [Parameter(Mandatory=$true)][int]$DebugPort
)
$ErrorActionPreference = 'Stop'
$plans = @()
$registryRoot = $null
$failed = $false
try {
    # Never use machine overrides on a developer machine or self-hosted runner.
    if ($env:GITHUB_ACTIONS -cne 'true' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted' -or
        $env:RUNNER_OS -cne 'Windows' -or $env:OS -cne 'Windows_NT' -or
        -not [Environment]::Is64BitProcess) { throw 'Hosted Windows runner required' }
    $repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')).TrimEnd('\','/')
    if (-not $env:GITHUB_WORKSPACE -or -not $repoRoot.Equals(
        [IO.Path]::GetFullPath($env:GITHUB_WORKSPACE).TrimEnd('\','/'), [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unexpected runner workspace'
    }
    . (Join-Path $PSScriptRoot '../workflow.ps1')
    $root = (Get-Item -LiteralPath $RunRoot).FullName
    Assert-WorkflowDirectory $repoRoot $root
    Assert-WorkflowChildPath (Join-Path $repoRoot 'target/acceptance') $root
    if ([IO.File]::ReadAllText((Join-Path $root '.synthetic-run')) -cne 'desktop-acceptance.v1') { throw 'Synthetic run required' }
    $exe = Get-Item -LiteralPath $Executable
    $valueName = $exe.Name
    if ($valueName -cnotmatch '^WeComArchive\.Acceptance\.[a-f0-9]{32}\.[a-z]+\.exe$' -or
        ($exe.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Dedicated test executable required' }
    Assert-WorkflowDirectory $root $exe.DirectoryName
    Assert-WorkflowChildPath (Join-Path $root 'app') $exe.FullName
    $profilePath = (Get-Item -LiteralPath $Profile).FullName
    Assert-WorkflowDirectory $root $profilePath
    if ($DebugPort -lt 1 -or $DebugPort -gt 65535) { throw 'Invalid test port' }

    # Microsoft documents HKLM before HKCU, then AppUserModelID / EXE name / '*'.
    # Use only this unique EXE name, never the production name, AppID or wildcard.
    # https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/webview2-idl#createcorewebview2environmentwithoptions
    $registryRoot = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
    $values = @{
        AdditionalBrowserArguments = "--remote-debugging-port=$DebugPort --remote-debugging-address=127.0.0.1 --disable-background-networking"
        UserDataFolder = $profilePath
    }
    foreach ($setting in @('AdditionalBrowserArguments', 'UserDataFolder')) {
        $key = $registryRoot.CreateSubKey("SOFTWARE\Policies\Microsoft\Edge\WebView2\$setting", $true)
        $entry = [pscustomobject]@{ key=$key; exists=($key.GetValueNames() -contains $valueName); value=$null; kind=$null; expected=$values[$setting]; written=$false }
        $plans += $entry
        # An AppID override takes priority over an executable-name override.
        if ($key.GetValueNames() -contains 'com.torbenxiong.wecomarchive') { throw 'Conflicting AppID policy' }
        if ($entry.exists) {
            $entry.kind = $key.GetValueKind($valueName)
            $entry.value = $key.GetValue($valueName, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        }
    }
    # Both old values and their types are captured before the first write.
    foreach ($entry in $plans) {
        $entry.key.SetValue($valueName, $entry.expected, [Microsoft.Win32.RegistryValueKind]::String)
        $entry.written = $true
    }
    [Console]::Out.WriteLine('ready')
    [Console]::Out.Flush()
    # EOF also releases the lease when the owning Node process exits.
    $release = [Console]::In.ReadLineAsync()
    if (-not $release.Wait([TimeSpan]::FromMinutes(3))) { throw 'Policy lease expired' }
} catch {
    $failed = $true
} finally {
    foreach ($entry in $plans) {
        try {
            if ($entry.written) {
                if ($entry.key.GetValueKind($valueName) -ne [Microsoft.Win32.RegistryValueKind]::String -or
                    $entry.key.GetValue($valueName) -cne $entry.expected) { throw 'Policy changed by another writer' }
                if ($entry.exists) { $entry.key.SetValue($valueName, $entry.value, $entry.kind) }
                else { $entry.key.DeleteValue($valueName, $false) }
            }
        } catch { $failed = $true }
        finally { $entry.key.Dispose() }
    }
    if ($registryRoot) { $registryRoot.Dispose() }
}
if ($failed) {
    [Console]::Error.WriteLine('SYNTHETIC_WEBVIEW_POLICY_FAILED')
    exit 1
}

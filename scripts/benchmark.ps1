# Run from Windows PowerShell. Results stay in a fresh temporary directory.
param(
    [Parameter(Mandatory = $true)][string]$Rom,
    [Parameter(Mandatory = $true)][string]$State,
    [ValidateRange(1, 10)][int]$Scale = 3,
    [switch]$NoHooks,
    [switch]$CpuShaders,
    [switch]$HostProfile,
    [switch]$GpuProfile,
    [switch]$NoAliasCopies
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'dist\kiutrace-perf.exe'
$library = Join-Path $env:APPDATA '3dsrecomp\0004000000030100.dll'
foreach ($file in @($exe, $library, $Rom, $State)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Missing file: $file" }
}
$Rom = (Resolve-Path -LiteralPath $Rom).ProviderPath
$State = (Resolve-Path -LiteralPath $State).ProviderPath
$output = Join-Path ([IO.Path]::GetTempPath()) ('kiu-perf-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $output | Out-Null
Copy-Item -LiteralPath $State -Destination (Join-Path $output 'state.state')
$commands = @('load state.state', 'overclock 2')
if (-not $NoHooks) {
    $commands += @('wide 1.7777778', 'hud 0.6', 'narrtop', 'portraits 0.5', 'mirror', 'pausetop')
}
$measure = if ($HostProfile) { 'timed 300' } else { 'bench 300' }
$commands += @('checkcode', 'run 90', $measure)
$commands + $commands | Set-Content -LiteralPath (Join-Path $output 'commands.txt')
$settings = @{
    KIUTRACE_LIB = $library
    KIUTRACE_DIRECT = '1'
    KIUTRACE_SCALE = "$Scale"
    KIUTRACE_NO_HOOKS = $(if ($NoHooks) { '1' } else { $null })
    ZAKURO_CPU_SHADERS = $(if ($CpuShaders) { '1' } else { $null })
    ZAKURO_GPU_SHADERS = $null
    ZAKURO_GPU_TIMES = $(if ($HostProfile -or $GpuProfile) { '1' } else { $null })
    ZAKURO_GPU_ALIAS_COPY = $(if ($NoAliasCopies) { '0' } else { $null })
    RUST_LOG = 'error,zakuro_core::loader=info,zakuro_gpu::times=info'
}
$previous = @{}
foreach ($key in $settings.Keys) {
    $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
    [Environment]::SetEnvironmentVariable($key, $settings[$key], 'Process')
}
Push-Location $output
try {
    & $exe $Rom (Join-Path $output 'commands.txt') $output *> (Join-Path $output 'results.log')
    if ($LASTEXITCODE -ne 0) { throw "Benchmark failed; see $output\results.log" }
    Get-Content -LiteralPath (Join-Path $output 'results.log') | Select-String 'drawing on|frames in|  host '
    Write-Output "Full log and commands: $output"
} finally {
    Pop-Location
    foreach ($key in $previous.Keys) {
        [Environment]::SetEnvironmentVariable($key, $previous[$key], 'Process')
    }
}

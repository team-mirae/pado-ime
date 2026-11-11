# Installs the build in Windows Sandbox and types through it there (windows/type-test.ps1), without
# touching this PC: the repository is mapped read-only, results come back in a folder of their own.
# Build first (windows/build.ps1). The Sandbox window shows the test typing; it closes by itself.
#   windows/sandbox.ps1             install + type tests, results in windows\host\build\sandbox\
#   windows/sandbox.ps1 -Keep       leave the Sandbox open afterwards, to try pado by hand
param([switch] $Keep)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..").Path
$results = "$root\windows\host\build\sandbox"
New-Item $results -ItemType Directory -Force | Out-Null
Remove-Item "$results\*" -Force -ErrorAction SilentlyContinue
$wsb = Join-Path $env:TEMP 'pado-ime.wsb'
$keepArg = if ($Keep) { '-Keep' } else { '' }
# vGPU off: with it on the Sandbox stayed black on the owner's PC (bada-windows' tools/sandbox).
@"
<Configuration>
  <vGPU>Disable</vGPU>
  <MappedFolders>
    <MappedFolder><HostFolder>$root</HostFolder><SandboxFolder>C:\pado-ime</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder>
    <MappedFolder><HostFolder>$results</HostFolder><SandboxFolder>C:\results</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder>
  </MappedFolders>
  <LogonCommand><Command>powershell -NoProfile -ExecutionPolicy Bypass -File C:\pado-ime\windows\sandbox-run.ps1 $keepArg</Command></LogonCommand>
</Configuration>
"@ | Set-Content $wsb -Encoding utf8
Start-Process $wsb
Write-Host "Windows Sandbox started. Results: $results\results.txt (done.txt when finished)"
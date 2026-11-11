# Opens Windows Sandbox with this repository mapped read-only at C:\bada-windows, to install and test
# a build without touching this PC (docs/testing.md). Build first (tools/build.ps1); in the Sandbox,
# run C:\bada-windows\tools\sandbox\in-sandbox.ps1 from an elevated PowerShell.
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\..\.."
$wsb = Join-Path $env:TEMP 'bada-windows.wsb'
# vGPU off: with it on the Sandbox stayed black on the owner's PC (FKeyIME installer/sandbox).
@"
<Configuration>
  <vGPU>Disable</vGPU>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$root</HostFolder>
      <SandboxFolder>C:\bada-windows</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand><Command>explorer.exe C:\bada-windows</Command></LogonCommand>
</Configuration>
"@ | Set-Content $wsb -Encoding utf8
Start-Process $wsb

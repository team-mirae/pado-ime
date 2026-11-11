# Runs a command inside a Visual Studio developer environment for one architecture.
#   tools/dev.ps1 -Arch x64 -Command 'cmake --preset x64'
# Adapted from team-mirae/jis-moonshot windows/tools/dev.ps1.
param(
    [ValidateSet('x64', 'x86')] [string] $Arch = 'x64',
    [Parameter(Mandatory)] [string] $Command
)
$ErrorActionPreference = 'Stop'
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'No Visual Studio with the C++ x64/x86 tools (Build Tools 2022 is enough).' }
$devShell = Join-Path $vs 'Common7\Tools\Launch-VsDevShell.ps1'
$vsArch = @{ x64 = 'amd64'; x86 = 'x86' }[$Arch]  # Launch-VsDevShell's names
# VsDevCmd looks for vswhere on PATH, and native stderr must not abort this script (PS 5.1).
$env:Path = "$(Split-Path $vswhere);$env:Path"
$ErrorActionPreference = 'Continue'
powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$devShell' -Arch $vsArch -HostArch amd64 -SkipAutomaticLocation | Out-Null; Set-Location '$PSScriptRoot\..'; $Command; exit `$LASTEXITCODE"
exit $LASTEXITCODE

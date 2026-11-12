# Builds pado's installer from the build in windows/host/build/ime/pado (windows/build.ps1 first) with
# Inno Setup 6: windows/host/build/installer/pado-ime-<version>-setup.exe. The version is the engine's
# (engine/Cargo.toml), unless given.
#   windows/installer.ps1 [-Version 0.3.1]
param([string] $Version)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..").Path
if (-not $Version) {
    $Version = (Select-String -Path "$root\engine\Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
}
if (-not (Test-Path "$root\windows\host\build\ime\pado\x64\bada_windows.dll")) { throw 'Nothing built: run windows/build.ps1 first.' }
$iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
          "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source }
if (-not $iscc) { throw 'Inno Setup 6 not found: winget install JRSoftware.InnoSetup' }
& $iscc /Q "/DAppVersion=$Version" "$root\windows\installer\pado.iss"
if ($LASTEXITCODE) { throw "ISCC failed ($LASTEXITCODE)" }
Write-Host "Built $root\windows\host\build\installer\pado-ime-$Version-setup.exe"

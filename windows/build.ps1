# Builds pado ime for Windows: bada's engine as pado_engine.dll and bada-windows' TIP (windows/host),
# x64 and x86, with the host's core tests; then the settings window (pado-settings.exe), the engine's
# data and the licenses, laid out as installed in windows/host/build/ime/pado/.
#   windows/build.ps1                   everything
#   windows/build.ps1 -Arch x64         one architecture of the TIP (the settings window is x64)
#   windows/build.ps1 -EngineTests      also cargo test in engine/ (bada's own tests)
# Then windows/install.ps1. Needs Rust (with i686-pc-windows-msvc) and VS Build Tools: docs/windows.md.
param(
    [ValidateSet('x64', 'x86')] [string[]] $Arch = @('x64', 'x86'),
    [switch] $EngineTests
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$hostDir = "$root\windows\host"
$out = "$hostDir\build\ime\pado"
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe" }

if ($EngineTests) {
    & $cargo test --manifest-path "$root\engine\Cargo.toml"
    if ($LASTEXITCODE) { throw 'engine tests failed' }
}

# The engine and the TIP, through bada-windows' own build.
& "$hostDir\tools\build.ps1" -Ime "$root\windows\pado" -Arch $Arch
if ($LASTEXITCODE) { throw 'windows/host/tools/build.ps1 failed' }

# The settings window: one x64 program (32-bit apps start it too), static CRT.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
$ErrorActionPreference = 'Continue'
& $cargo build --manifest-path "$root\windows\settings\Cargo.toml" --release --target x86_64-pc-windows-msvc --target-dir "$hostDir\build\cargo"
$ErrorActionPreference = 'Stop'
Remove-Item Env:RUSTFLAGS
if ($LASTEXITCODE) { throw 'cargo build pado-settings failed' }
Copy-Item "$hostDir\build\cargo\x86_64-pc-windows-msvc\release\pado-settings.exe" $out -Force

# The data ime.json lists (data/…), as the Mac app shipped it: en-ko's sensitive words left out.
$data = "$out\data"
Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
New-Item "$data\en-ko\brands" -ItemType Directory -Force | Out-Null
$manifest = Get-Content "$root\windows\pado\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
foreach ($item in $manifest.data) {
    $files = if ($item -is [string]) { @($item) } else { @($item.concat | Where-Object { $_ -is [string] }) }
    foreach ($f in $files) { Copy-Item "$root\engine\$f" (Join-Path $out $f) -Force }
}
Copy-Item "$root\engine\data\en-ko\brands\*.tsv" "$data\en-ko\brands\" -Force

# Licenses: pado's (MIT), bada-windows' (CC0) and the data's.
$licenses = "$out\licenses"
New-Item $licenses -ItemType Directory -Force | Out-Null
Copy-Item "$root\LICENSE" "$licenses\LICENSE-pado.txt" -Force
Copy-Item "$hostDir\LICENSE" "$licenses\LICENSE-bada-windows.txt" -Force
Copy-Item "$root\engine\data\LICENSE-*.txt" $licenses -Force
Write-Host "Built $out"
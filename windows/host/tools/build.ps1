# Builds bada_windows.dll for x64 and x86 (32-bit apps load the x86 one), builds an IME's engine for
# both, lays them out as the IME is installed (build/ime/<name>/), and runs the core tests against it.
#   tools/build.ps1                          the shout example (examples/shout)
#   tools/build.ps1 -Ime path\to\ime-folder   another IME: a folder with ime.json and engine\Cargo.toml
#   tools/build.ps1 -Arch x64                one architecture
# Outputs: build/<arch>/ (the DLL, the tests, bw_testpad, bw_state, bw_unloadtest) and
# build/ime/<name>/ (ime.json, x64\, x86\: what tools/install.ps1 installs).
param(
    [ValidateSet('x64', 'x86')] [string[]] $Arch = @('x64', 'x86'),
    [string] $Ime = "$PSScriptRoot\..\examples\shout"
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$Ime = Resolve-Path $Ime
$manifest = Get-Content "$Ime\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe" }
if (-not (Test-Path $cargo)) { throw 'cargo not found: install Rust (rustup) first.' }
$triples = @{ x64 = 'x86_64-pc-windows-msvc'; x86 = 'i686-pc-windows-msvc' }
$out = "$root\build\ime\$($manifest.name)"
New-Item $out -ItemType Directory -Force | Out-Null
Copy-Item "$Ime\ime.json" $out -Force
if ($manifest.windows.icon) { Copy-Item (Join-Path $Ime $manifest.windows.icon) $out -Force }

foreach ($a in $Arch) {
    $triple = $triples[$a]
    # The engine. The static CRT: it loads into every app and can't count on a VC++ runtime.
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    $ErrorActionPreference = 'Continue'
    & $cargo build --manifest-path "$Ime\engine\Cargo.toml" --release --lib --target $triple --target-dir "$root\build\cargo"
    $ErrorActionPreference = 'Stop'
    Remove-Item Env:RUSTFLAGS
    if ($LASTEXITCODE) { throw "cargo build $triple failed" }
    $engine = Get-ChildItem "$root\build\cargo\$triple\release" -Filter '*.dll' | Select-Object -First 1
    if (-not $engine) { throw "no engine DLL for $triple" }

    $build = "build/$a"
    # The core tests type through shout; another engine gets only the engine-free tests.
    $shout = if ($manifest.prefix -eq 'shout') { $engine.FullName } else { '' }
    & "$PSScriptRoot\dev.ps1" -Arch $a -Command "cmake --preset $a -B $build -DBW_SHOUT_LIB='$shout' | Out-Null; if (`$LASTEXITCODE) { exit 1 }; cmake --build $build; if (`$LASTEXITCODE) { exit 1 }; & '$build\bw_core_tests.exe' '$shout'"
    if ($LASTEXITCODE) { throw "build or tests failed for $a" }

    New-Item "$out\$a" -ItemType Directory -Force | Out-Null
    Copy-Item "$root\$build\bada_windows.dll", $engine.FullName "$out\$a\" -Force
}
Write-Host "Built $out"

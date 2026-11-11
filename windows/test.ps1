# pado's tests on Windows, after windows/build.ps1:
#   1. windows/tests/pado_tests.cpp: the built IME loaded as the TIP loads it, bada's layouts typed
#      through bada-windows' host logic (x64 and x86);
#   2. the settings window's server (pado-settings.exe --url-file): its page, its API, its guards.
#      It writes nothing: the registry and the dictionary are only read.
#   windows/test.ps1 [-Arch x64]
param([ValidateSet('x64', 'x86')] [string[]] $Arch = @('x64', 'x86'))
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$hostDir = "$root\windows\host"
$ime = "$hostDir\build\ime\pado"
$root = "$root"; $hostDir = "$hostDir"
if (-not (Test-Path "$ime\ime.json")) { throw 'Nothing built: run windows/build.ps1 first.' }
$failed = 0

foreach ($a in $Arch) {
    $out = "$hostDir\build\$a"
    $src = "$root\windows\tests\pado_tests.cpp"
    $exe = "$out\pado_tests.exe"
    Remove-Item $exe -ErrorAction SilentlyContinue
    & "$hostDir\tools\dev.ps1" -Arch $a -Command "cl /nologo /std:c++20 /EHsc /utf-8 /O2 /MT /W3 /DUNICODE /D_UNICODE /DNOMINMAX /I '$hostDir\src\core' '$src' /Fo'$out\pado_tests.obj' /Fe'$exe' /link '$out\bw_core.lib'"
    if (-not (Test-Path $exe)) { Write-Host "pado_tests didn't build for $a" -ForegroundColor Red; $failed++; continue }
    & $exe "$ime\ime.json" "$root\docs\dictionary-example.json"
    if ($LASTEXITCODE) { Write-Host "pado_tests failed for $a" -ForegroundColor Red; $failed++ } else { Write-Host "pado_tests passed for $a" }
}

# The settings window's server.
$urlFile = Join-Path ([IO.Path]::GetTempPath()) "pado-settings-test-$PID.txt"
Remove-Item $urlFile -ErrorAction SilentlyContinue
$p = Start-Process "$ime\pado-settings.exe" -ArgumentList '--url-file', "`"$urlFile`"" -PassThru
for ($i = 0; $i -lt 50 -and -not (Test-Path $urlFile); $i++) { Start-Sleep -Milliseconds 100 }
$url = (Get-Content $urlFile -Raw).Trim().TrimEnd('/')
Remove-Item $urlFile
$port = ([uri]$url).Port
function Check($name, $ok) { if ($ok) { Write-Host "  ok  $name" } else { Write-Host "  FAIL $name" -ForegroundColor Red; $script:failed++ } }
try {
    $page = Invoke-WebRequest "$url/" -UseBasicParsing
    Check 'the page' ($page.Content -match 'pado ime')
    $state = Invoke-RestMethod "$url/api/state"
    Check 'the catalog' ($state.catalog.layouts.Count -ge 13 -and ($state.catalog.layouts | Where-Object id -eq 15).name -match 'bada roman')
    Check 'the F-key actions' ($state.catalog.actions.Count -gt 0)
    Check "bada's settings model, from upstream" ((Invoke-WebRequest "$url/settings.js" -UseBasicParsing).Content -match 'bada-settings/1')
    $noToken = try { Invoke-WebRequest "http://127.0.0.1:$port/api/state" -UseBasicParsing | Out-Null; $false } catch { $true }
    Check 'refused without the token' $noToken
    $badHost = try { Invoke-WebRequest "$url/api/state" -Headers @{ Host = "localhost:$port" } -UseBasicParsing | Out-Null; $false } catch { $true }
    Check 'refused under another host name' $badHost
} finally {
    try { Invoke-RestMethod -Method Post "$url/api/bye" | Out-Null } catch {}
    Start-Sleep -Milliseconds 300
    Check 'it exits when told' ($p.HasExited)
    if (-not $p.HasExited) { $p.Kill() }
}
if ($failed) { throw "$failed failed" }
Write-Host 'All passed'
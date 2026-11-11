# Inside Windows Sandbox (tools/sandbox/start.ps1): installs the mapped build, adds the IME to the
# language list, records the registration, and runs the type tests. Everything is gone when the
# Sandbox closes.
param([string] $Name = 'shout')
$ErrorActionPreference = 'Stop'
Set-ExecutionPolicy -Scope Process Bypass -Force
& C:\bada-windows\tools\install.ps1 -Name $Name
$manifest = Get-Content "$env:ProgramFiles\bada-windows\$Name\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
$clsid = $manifest.windows.clsid
$results = "$env:USERPROFILE\Desktop\bada-windows-results.txt"
& {
    "== Windows: $((Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion) $((Get-CimInstance Win32_OperatingSystem).BuildNumber)"
    "== Language list"
    foreach ($l in (Get-WinUserLanguageList).ToArray()) { "  $($l.LanguageTag): $($l.InputMethodTips -join ', ')" }
    "== TSF registration"
    reg query "HKLM\SOFTWARE\Microsoft\CTF\TIP\$clsid" /s 2>&1 | Select-String -NotMatch 'Category|^$'
    "== COM (64 / 32)"
    reg query "HKLM\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" 2>&1
    reg query "HKLM\SOFTWARE\WOW6432Node\Classes\CLSID\$clsid\InprocServer32" 2>&1
    "== Type tests"
    & C:\bada-windows\tools\type-test.ps1 -Name $Name
} *>&1 | Tee-Object $results
Write-Host "Results: $results"

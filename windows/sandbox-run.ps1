# Inside Windows Sandbox (windows/sandbox.ps1): installs the mapped build, adds pado to the language
# list, records what was registered, runs windows/type-test.ps1, writes C:\results, and shuts the
# Sandbox down (unless -Keep). Everything in the Sandbox is gone when it closes.
param([switch] $Keep)
$ErrorActionPreference = 'Continue'
$log = 'C:\results\results.txt'
& {
    "== Windows: $((Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion) $((Get-CimInstance Win32_OperatingSystem).BuildNumber)"
    "== Install"
    # The mapped repository is read-only: the build is installed from a copy.
    $from = 'C:\pado-ime\windows'
    foreach ($d in 'host\tools', 'host\build\ime\pado', 'host\build\x64') { New-Item "C:\pado\$d" -ItemType Directory -Force | Out-Null }
    Copy-Item "$from\install.ps1", "$from\type-test.ps1" C:\pado\
    Copy-Item "$from\host\LICENSE" C:\pado\host\
    Copy-Item "$from\host\tools\*" C:\pado\host\tools\ -Recurse
    Copy-Item "$from\host\build\ime\pado\*" C:\pado\host\build\ime\pado\ -Recurse
    Copy-Item "$from\host\build\x64\bw_testpad.exe" C:\pado\host\build\x64\
    & C:\pado\install.ps1
    $manifest = Get-Content "$env:ProgramFiles\bada-windows\pado\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
    $clsid = $manifest.windows.clsid
    # Already elevated here, so the installer skipped its unelevated half: the language list.
    $tip = '0411:{0}{1}' -f $clsid, $manifest.windows.profile
    $list = Get-WinUserLanguageList
    $ja = (New-WinUserLanguageList ja-JP)[0]
    $ja.InputMethodTips.Clear()
    $ja.InputMethodTips.Add($tip)
    $list.Add($ja)
    Set-WinUserLanguageList $list -Force
    # The diagnostic log: activations, focus, the engine's loading and errors (never typed text).
    New-Item HKCU:\Software\bada-windows\pado -Force -ErrorAction SilentlyContinue | Out-Null
    Set-ItemProperty HKCU:\Software\bada-windows\pado DiagnosticLog 1 -Type DWord
    "== Installed files"
    Get-ChildItem "$env:ProgramFiles\bada-windows\pado" -Recurse -File | Measure-Object Length -Sum | ForEach-Object { "  $($_.Count) files, $([math]::Round($_.Sum / 1MB, 1)) MB" }
    "== Language list"
    foreach ($l in (Get-WinUserLanguageList).ToArray()) { "  $($l.LanguageTag): $($l.InputMethodTips -join ', ')" }
    "== TSF registration"
    reg query "HKLM\SOFTWARE\Microsoft\CTF\TIP\$clsid" /s 2>&1 | Select-String -NotMatch 'Category|^$'
    "== COM (64 / 32)"
    reg query "HKLM\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" 2>&1
    reg query "HKLM\SOFTWARE\WOW6432Node\Classes\CLSID\$clsid\InprocServer32" 2>&1
    # Each test window must come to the front to get the injected keys. In the Sandbox the logon
    # script isn't the foreground program, so Windows' foreground lock kept every window after the
    # first in the back (measured: one case passed, the rest got no keys). Off for this session.
    Add-Type -Namespace Pado -Name Spi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint a, uint b, System.IntPtr c, uint d); [DllImport("user32.dll")] public static extern bool LockSetForegroundWindow(uint code);'
    [void][Pado.Spi]::SystemParametersInfo(0x2001, 0, [IntPtr]::Zero, 3)  # SPI_SETFOREGROUNDLOCKTIMEOUT 0
    [void][Pado.Spi]::LockSetForegroundWindow(2)                          # LSFW_UNLOCK
    Set-ItemProperty 'HKCU:\Control Panel\Desktop' ForegroundLockTimeout 0 -Type DWord
    "== Type tests"
    & C:\pado\type-test.ps1
    "== Log"
    Get-Content "$env:ProgramData\bada-windows\pado.log" -ErrorAction SilentlyContinue | Select-Object -Last 80
} *>&1 | Out-File $log -Encoding utf8
'done' | Out-File C:\results\done.txt
if (-not $Keep) { Stop-Computer -Force }
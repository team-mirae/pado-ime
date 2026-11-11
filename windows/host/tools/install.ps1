# Installs or removes a bada-windows IME on this PC (development install; needs admin, so it
# re-launches itself elevated). Other input methods and the rest of the language list are left as
# they are. Test in a VM or Windows Sandbox first (docs/testing.md).
#   tools/install.ps1                  build/ime/shout → Program Files\bada-windows\shout, register
#                                      x64 + x86, add it to its language's keyboards
#   tools/install.ps1 -Name myime      another IME built by tools/build.ps1 -Ime
#   tools/install.ps1 -Update          copy a new build over the installed one, no elevation
#   tools/install.ps1 -Uninstall       undo all of it
#   tools/install.ps1 -Log             turn the diagnostic log on and follow it
# Apps that were running keep the old DLL (or none) until they restart (FKeyIME
# docs/issues/apps-dont-load-fkeyime.md). Adapted from team-mirae/jis-moonshot windows/tools/install.ps1.
param(
    [string] $Name = 'shout',
    [switch] $Uninstall,
    [switch] $Update,
    [switch] $Log,
    [string] $User  # set by the unelevated call: the SID that may update the install folder
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
$built = "$root\build\ime\$Name"
$dest = "$env:ProgramFiles\bada-windows\$Name"
$data = "$env:ProgramData\bada-windows"
$manifestPath = if (Test-Path "$built\ime.json") { "$built\ime.json" } else { "$dest\ime.json" }
$manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
$clsid = $manifest.windows.clsid
$language = if ($manifest.windows.language) { $manifest.windows.language } else { 'ja-JP' }
$langId = [Globalization.CultureInfo]::GetCultureInfo($language).LCID
$tip = '{0:X4}:{1}{2}' -f $langId, $clsid, $manifest.windows.profile
$key = "HKCU:\Software\bada-windows\$($manifest.name)"

function Copy-Build {
    Copy-Item "$built\ime.json" $dest -Force
    Get-ChildItem $built -Filter '*.ico' | Copy-Item -Destination $dest -Force
    foreach ($arch in 'x64', 'x86') {
        New-Item "$dest\$arch" -ItemType Directory -Force | Out-Null
        foreach ($file in Get-ChildItem "$built\$arch" -Filter '*.dll') {
            $target = "$dest\$arch\$($file.Name)"
            Get-ChildItem "$dest\$arch" -Filter "$($file.Name).old-*" | Remove-Item -Force -ErrorAction SilentlyContinue
            if (Test-Path $target) {
                # A process with the DLL loaded keeps the file open, but it can be renamed.
                try { Remove-Item $target -Force } catch { Rename-Item $target "$($file.Name).old-$(Get-Date -f yyyyMMddHHmmssfff)" }
            }
            Copy-Item $file.FullName "$dest\$arch\"
        }
    }
}

if ($Update) {
    Copy-Build
    Write-Host "Updated $dest (apps load it when they next start)"
    return
}
if ($Log) {
    New-Item $key -Force -ErrorAction SilentlyContinue | Out-Null
    Set-ItemProperty $key DiagnosticLog 1 -Type DWord
    Get-Content "$data\$($manifest.name).log" -Tail 40 -Wait -Encoding utf8
    return
}

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).
    IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    $argv = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-Name', $Name,
              '-User', [Security.Principal.WindowsIdentity]::GetCurrent().User.Value)
    if ($Uninstall) { $argv += '-Uninstall' }
    $p = Start-Process powershell -Verb RunAs -ArgumentList $argv -Wait -PassThru
    if ($p.ExitCode) { exit $p.ExitCode }
    # The language list is per user: changed here, unelevated (the elevated process may be another
    # account). Iterate .ToArray(): the list enumerates as one object otherwise (FKeyIME's notes).
    $list = Get-WinUserLanguageList
    $entry = $null
    foreach ($l in $list.ToArray()) {
        if ($l.LanguageTag -eq $language -or $l.LanguageTag -eq $language.Split('-')[0]) { $entry = $l }
    }
    if ($Uninstall) {
        if ($entry -and $entry.InputMethodTips -contains $tip) {
            [void]$entry.InputMethodTips.Remove($tip)
            Set-WinUserLanguageList $list -Force
        }
    } else {
        if (-not $entry) {
            $entry = (New-WinUserLanguageList $language)[0]
            $entry.InputMethodTips.Clear()
            $list.Add($entry)
        }
        if ($entry.InputMethodTips -notcontains $tip) {
            $entry.InputMethodTips.Add($tip)
            Set-WinUserLanguageList $list -Force
        }
    }
    Write-Host "$($entry.LanguageTag) keyboards now:"
    foreach ($l in (Get-WinUserLanguageList).ToArray()) {
        if ($l.LanguageTag -eq $entry.LanguageTag) { $l.InputMethodTips | ForEach-Object { "  $_" } }
    }
    exit 0
}

New-Item $data -ItemType Directory -Force | Out-Null
Start-Transcript "$data\install-$Name.log" -Append | Out-Null
trap { Write-Host $_ -ForegroundColor Red; Stop-Transcript | Out-Null; exit 1 }

if ($Uninstall) {
    foreach ($arch in 'x64', 'x86') {
        $regsvr = if ($arch -eq 'x64') { 'System32' } else { 'SysWOW64' }
        if (Test-Path "$dest\$arch\bada_windows.dll") { & "$env:SystemRoot\$regsvr\regsvr32.exe" /s /u "$dest\$arch\bada_windows.dll" }
    }
    foreach ($k in "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$clsid", "HKLM:\SOFTWARE\Classes\CLSID\$clsid",
                   "HKLM:\SOFTWARE\WOW6432Node\Classes\CLSID\$clsid") {
        Remove-Item $k -Recurse -ErrorAction SilentlyContinue
    }
    Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path $dest) { Write-Warning "$dest is still in use; delete it after signing out." }
    Stop-Transcript | Out-Null
    exit 0
}

if (-not (Test-Path "$built\x64\bada_windows.dll")) { throw "Nothing built in ${built}: run tools/build.ps1 first." }
# The log folder, writable by every process, AppContainer (Store) apps included.
icacls $data /grant '*S-1-15-2-1:(OI)(CI)M' '*S-1-5-32-545:(OI)(CI)M' | Out-Null
# Program Files is readable by ALL APPLICATION PACKAGES; the user's Desktop isn't.
New-Item $dest -ItemType Directory -Force | Out-Null
if ($User) { icacls $dest /grant "*${User}:(OI)(CI)M" | Out-Null }
Copy-Build
Copy-Item "$root\LICENSE" $dest -Force
& "$env:SystemRoot\System32\regsvr32.exe" /s "$dest\x64\bada_windows.dll"
if ($LASTEXITCODE) { throw "regsvr32 x64 failed ($LASTEXITCODE)" }
& "$env:SystemRoot\SysWOW64\regsvr32.exe" /s "$dest\x86\bada_windows.dll"
if ($LASTEXITCODE) { throw "regsvr32 x86 failed ($LASTEXITCODE)" }
Write-Host "Registered $dest"
Stop-Transcript | Out-Null
exit 0

# pado's installer, the per-user half (windows/installer/pado.iss runs it as the user who started
# setup): adds pado ime to that user's 日本語 keyboards and lets Store (AppContainer) apps read its
# settings key, as windows/install.ps1 does. -Uninstall takes it out of the keyboards again.
param([switch] $Uninstall)
$ErrorActionPreference = 'Stop'
$manifest = Get-Content "$PSScriptRoot\..\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
$language = $manifest.windows.language
$tip = '{0:X4}:{1}{2}' -f [Globalization.CultureInfo]::GetCultureInfo($language).LCID, $manifest.windows.clsid, $manifest.windows.profile
# Iterate .ToArray(): the list enumerates as one object otherwise (FKeyIME's notes).
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
    exit 0
}
if (-not $entry) {
    $entry = (New-WinUserLanguageList $language)[0]
    $entry.InputMethodTips.Clear()
    $list.Add($entry)
}
if ($entry.InputMethodTips -notcontains $tip) {
    $entry.InputMethodTips.Add($tip)
    Set-WinUserLanguageList $list -Force
}
$key = "HKCU:\Software\bada-windows\$($manifest.name)"
if (-not (Test-Path $key)) { New-Item $key -Force | Out-Null }
$acl = Get-Acl $key
$sid = New-Object Security.Principal.SecurityIdentifier 'S-1-15-2-1'
$acl.AddAccessRule((New-Object Security.AccessControl.RegistryAccessRule($sid, 'ReadKey', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
Set-Acl $key $acl
exit 0

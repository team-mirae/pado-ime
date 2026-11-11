# Installs, updates or removes pado ime on this PC: bada-windows' installer (windows/host/tools/
# install.ps1) for the build in windows/host/build/ime/pado. Needs admin (UAC). It registers the TIP
# for x64 and x86 apps and adds "pado ime" to the 日本語 keyboards; nothing else in the language list
# changes. Restart apps that were running.
#   windows/install.ps1               install (Program Files\bada-windows\pado)
#   windows/install.ps1 -Update       copy a new build over it, no UAC (apps load it when they restart)
#   windows/install.ps1 -Uninstall    undo it all (your settings and dictionary stay)
#   windows/install.ps1 -Log          turn the diagnostic log on and follow it
param([switch] $Uninstall, [switch] $Update, [switch] $Log)
$ErrorActionPreference = 'Stop'
$argv = @{ Name = 'pado' }
if ($Uninstall) { $argv.Uninstall = $true }
if ($Update) { $argv.Update = $true }
if ($Log) { $argv.Log = $true }
& "$PSScriptRoot\host\tools\install.ps1" @argv
if ($LASTEXITCODE) { exit $LASTEXITCODE }
if (-not $Uninstall -and -not $Log -and -not $Update) {
    # The settings key, readable by Store (AppContainer) apps: ALL APPLICATION PACKAGES may read it.
    # (bada-windows grants it at its first own write; the settings window writes it first.)
    $key = 'HKCU:\Software\bada-windows\pado'
    if (-not (Test-Path $key)) { New-Item $key -Force | Out-Null }
    $acl = Get-Acl $key
    $sid = New-Object Security.Principal.SecurityIdentifier 'S-1-15-2-1'
    $acl.AddAccessRule((New-Object Security.AccessControl.RegistryAccessRule($sid, 'ReadKey', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    Set-Acl $key $acl
    Write-Host 'pado ime is installed: Win+Space, under 日本語. Restart apps that were running.'
}
exit 0
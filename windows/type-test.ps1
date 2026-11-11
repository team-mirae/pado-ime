# Types bada's layouts through the INSTALLED pado ime into bw_testpad (bada-windows' test window: a
# classic Edit, a RichEdit through the IMM layer, a RichEdit on TSF) and checks the text. The testpad
# is in front while it runs: don't touch the keyboard. Run it in Windows Sandbox (windows/sandbox.ps1)
# or after windows/install.ps1. Each case's settings are set for it and the old ones put back after.
#   windows/type-test.ps1                  every case, every control
#   windows/type-test.ps1 -Target tsf     one control
#   windows/type-test.ps1 -Filter roman   cases whose name has "roman"
# Adapted from windows/host/tools/type-test.ps1 (shout's cases).
param(
    [ValidateSet('edit', 'rich', 'tsf')] [string[]] $Target = @('edit', 'rich', 'tsf'),
    [string] $Filter = '',
    [int] $LoadWait = 8000
)
$ErrorActionPreference = 'Stop'
$pad = Resolve-Path "$PSScriptRoot\host\build\x64\bw_testpad.exe"
$manifest = Get-Content "$env:ProgramFiles\bada-windows\pado\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
$lang = '0x{0:X4}' -f [Globalization.CultureInfo]::GetCultureInfo($manifest.windows.language).LCID
$ime = @('--clsid', $manifest.windows.clsid, '--profile', $manifest.windows.profile, '--lang', $lang)
$key = 'HKCU:\Software\bada-windows\pado'
if (-not (Test-Path $key)) { New-Item $key -Force | Out-Null }

# name, keys, expected text, settings (registry values; EngineSettings is bada's JSON). Every case
# starts in muhenkan (NonConvert) and ends committed: classic controls show only the first character
# of an open composition. 変換 is Convert, カタカナ KanaMode, 半角/全角 Backquote.
$cases = @(
    @('muhenkan: QWERTY with JIS symbols', 'NonConvert KeyA S-Digit2 BracketLeft IntlRo S-IntlRo', 'a"@\_', @{}),
    @('2-set: 한글, Space commits and types', 'NonConvert Convert KeyG KeyK KeyS KeyR KeyM KeyF Space', '한글 ', @{ EngineSettings = '{"enabled":[10]}' }),
    @('2-set: Esc commits', 'NonConvert Convert KeyG KeyK Escape', '하', @{ EngineSettings = '{"enabled":[10]}' }),
    @('2-set: 無変換 commits and leaves henkan', 'NonConvert Convert KeyG KeyK NonConvert KeyA', '하a', @{ EngineSettings = '{"enabled":[10]}' }),
    @('bada roman: gwiha is 귀하', 'NonConvert Convert KeyG KeyW KeyI KeyH KeyA S-Enter', '귀하', @{ EngineSettings = '{"enabled":[0,15]}' }),
    @('kana: 3uq, ⇧Tab to hiragana', 'NonConvert Convert Digit3 KeyU KeyQ S-Tab S-Enter', 'あなた', @{ EngineSettings = '{"enabled":[20],"script_forms":true}' }),
    @('半角/全角 in henkan: full width', 'NonConvert Convert KeyA KeyB Backquote S-Enter', 'ａｂ', @{ EngineSettings = '{"enabled":[0]}' }),
    @('profiles: カタカナ and 変換 each their own', 'NonConvert KanaMode KeyH KeyI S-Enter Convert KeyG KeyK S-Enter', 'hi하',
      @{ EngineSettings = '{"enabled":[10],"use_profiles":true,"profiles":{"katakana":{"enabled":[0]}}}' }),
    @('caps: Caps Lock switches mode (toggle)', 'NonConvert CapsLock KeyG KeyK S-Enter', '하', @{ EngineSettings = '{"enabled":[10]}'; CapsLock = 'toggle' })
)
$apiCases = @(
    @('API: what''s staged', 'tsf', 'NonConvert Convert KeyG KeyK @state Escape', '(?s)^1 하\n\d+ \{(?=.*"staged":"하")(?=.*"ime":"pado")'),
    @('API: a password field is a security error', 'pwscope', 'NonConvert Convert KeyG KeyK @state', '(?s)^-3 \n\d+ \{(?=.*"secure":true)')
)

$names = 'CapsLock', 'CapsRule', 'PassOn', 'PublishState', 'EngineSettings'
$saved = @{}
foreach ($n in $names) { $saved[$n] = (Get-ItemProperty $key -Name $n -ErrorAction SilentlyContinue).$n }
$failed = 0
$out = Join-Path $env:TEMP 'pado-type-test.txt'
$stateFile = Join-Path $env:TEMP 'pado-type-test-state.txt'
function Bump { Set-ItemProperty $key SettingsVersion ([int](Get-Random -Maximum 2000000000)) -Type DWord }
function Run($targetName, $keys, $settings) {
    foreach ($n in 'CapsLock', 'CapsRule', 'PassOn', 'EngineSettings') { Remove-ItemProperty $key -Name $n -ErrorAction SilentlyContinue }
    Set-ItemProperty $key PublishState 1 -Type DWord
    foreach ($k in $settings.Keys) { Set-ItemProperty $key $k $settings[$k] }
    Bump
    Remove-Item $out, $stateFile -ErrorAction SilentlyContinue
    $keys = $keys -replace '@state', "@state=$stateFile"
    # Each testpad is a new program: pado loads the engine and its data (about 50 MB) on the thread
    # pool when it activates, and keys wait for it. Typed before it's loaded, they'd land after the
    # testpad has read its text. ($LoadWait ms: a cold Windows Sandbox needs several seconds.)
    $keys = "@wait=$LoadWait $keys"
    $p = Start-Process $pad -ArgumentList (@('--target', $targetName, '--keys', "`"$keys`"", '--out', "`"$out`"") + $ime) -PassThru
    # The testpad must be in front for the keys it injects (it waits $LoadWait ms first).
    Start-Sleep -Milliseconds 700
    [void](New-Object -ComObject WScript.Shell).AppActivate($p.Id)
    # The first program to load pado loads the engine's data (about a second).
    if (-not $p.WaitForExit(60000)) { $p.Kill(); return '<timeout>' }
    if (Test-Path $out) { [IO.File]::ReadAllText($out, [Text.Encoding]::UTF8) } else { '<no output>' }
}
try {
    foreach ($c in $cases) {
        if ($Filter -and $c[0] -notlike "*$Filter*") { continue }
        foreach ($t in $Target) {
            $got = Run $t $c[1] $c[3]
            # Nothing at all: the window didn't get the keys (it wasn't in front). Once more.
            if ($got -eq '' -and $c[2] -ne '') { $got = Run $t $c[1] $c[3] }
            if ($got -ceq $c[2]) { Write-Host "ok   $($c[0]) [$t]" }
            else { Write-Host "FAIL $($c[0]) [$t]: got [$got], expected [$($c[2])]" -ForegroundColor Red; $failed++ }
        }
    }
    foreach ($c in $apiCases) {
        if ($Filter -and $c[0] -notlike "*$Filter*") { continue }
        Run $c[1] $c[2] @{ EngineSettings = '{"enabled":[10]}' } | Out-Null
        $got = if (Test-Path $stateFile) { [IO.File]::ReadAllText($stateFile, [Text.Encoding]::UTF8) } else { '<no state>' }
        if ($got -cmatch $c[3]) { Write-Host "ok   $($c[0]) [$($c[1])]" }
        else { Write-Host "FAIL $($c[0]) [$($c[1])]: got [$got]" -ForegroundColor Red; $failed++ }
    }
} finally {
    foreach ($n in $names) {
        if ($null -eq $saved[$n]) { Remove-ItemProperty $key -Name $n -ErrorAction SilentlyContinue }
        else { Set-ItemProperty $key $n $saved[$n] }
    }
    Bump
}
if ($failed) { Write-Host "$failed failed" -ForegroundColor Red; exit 1 }
Write-Host 'all passed'

# Types key sequences through the installed IME into bw_testpad and checks the text: a classic Edit
# control and a RichEdit through the IMM compatibility layer, and a RichEdit on TSF itself. The
# testpad is the foreground window while it runs: don't touch the keyboard. Settings a case needs are
# set for it and restored after. Cases are shout's (examples/shout); another IME brings its own.
#   tools/type-test.ps1                  all cases, every control
#   tools/type-test.ps1 -Target tsf     one control
#   tools/type-test.ps1 -Filter caps    cases whose name has "caps"
# Adapted from team-mirae/jis-moonshot windows/tools/type-test.ps1.
param(
    [ValidateSet('edit', 'rich', 'tsf')] [string[]] $Target = @('edit', 'rich', 'tsf'),
    [string] $Filter = '',
    [string] $Name = 'shout'
)
$ErrorActionPreference = 'Stop'
$pad = Resolve-Path "$PSScriptRoot\..\build\x64\bw_testpad.exe"
$manifest = Get-Content "$env:ProgramFiles\bada-windows\$Name\ime.json" -Raw -Encoding utf8 | ConvertFrom-Json
$language = if ($manifest.windows.language) { $manifest.windows.language } else { 'ja-JP' }
$lang = '0x{0:X4}' -f [Globalization.CultureInfo]::GetCultureInfo($language).LCID
$ime = @('--clsid', $manifest.windows.clsid, '--profile', $manifest.windows.profile, '--lang', $lang)
$key = "HKCU:\Software\bada-windows\$($manifest.name)"
if (-not (Test-Path $key)) { New-Item $key -Force | Out-Null }

# name, keys, expected text, settings for the case (@{ value = data }; $null: removed)
# Under the Japanese layout the unshifted Caps Lock key is 英数 (VK_DBE_ALPHANUMERIC): Caps Lock
# itself is Shift + it, so the respect cases press S-CapsLock.
# Cases end committed: classic controls (edit, rich) show only the first character of an open
# composition (docs/tsf-notes.md). Edit controls store a new line as CR LF.
$cases = @(
    @('HELLO world (superkeyboard''s shout test)', 'KeyH KeyE KeyL KeyL KeyO Space KeyW KeyO KeyR KeyL KeyD Tab Enter', "HELLO world`r`n", @{}),
    @('Shift+Enter commits, nothing else', 'KeyA KeyB S-Enter', 'AB', @{}),
    @('Esc commits, then does nothing', 'KeyX Escape', 'X', @{}),
    @('Period commits, then types .', 'KeyA KeyB Period', 'AB.', @{}),
    @('Backspace edits, then deletes', 'KeyO KeyK Space KeyN KeyO Backspace Backspace Backspace', 'OK', @{}),
    @('a shortcut commits, then reaches the app', 'KeyX C-KeyA', 'X', @{}),
    @('muhenkan (無変換) types JIS', 'KeyH KeyI NonConvert KeyA S-Digit2 BracketLeft IntlRo S-IntlRo IntlYen', "HIa`"@\_$([char]0xA5)", @{}),
    @('henkan (変換) is the home key: the next entry with a menu', 'NonConvert Convert KeyH KeyI Convert S-Enter', 'hi', @{}),
    @('Shift+変換 goes back', 'KeyH KeyI Convert S-Convert S-Enter', 'HI', @{}),
    @('カタカナ: henkan (shout has no profiles: the next entry too)', 'NonConvert KanaMode KeyH KeyI KanaMode S-Enter', 'hi', @{}),
    @('半角/全角 in muhenkan shouts the selection', 'NonConvert KeyQ KeyU KeyI KeyE KeyT S-Home Backquote', 'QUIET', @{}),
    @('keys passed on through Windows'' layout', 'NonConvert KeyA KeyB', 'ab', @{ PassOn = 'windows' }),
    @('caps: Caps Lock switches mode (toggle)', 'CapsLock KeyA CapsLock KeyB S-Enter', 'aB', @{ CapsLock = 'toggle' }),
    @('caps: Caps Lock types capitals in muhenkan, Shift or not (respect)', 'NonConvert S-CapsLock KeyA S-KeyB S-CapsLock KeyC', 'ABc', @{ CapsLock = 'respect' }),
    @('caps: UPPER_SNAKE', 'NonConvert S-CapsLock KeyA Space KeyB S-CapsLock', 'A_B', @{ CapsLock = 'respect'; CapsRule = 'uppersnake' }),
    @('#9 a virtual key without a scan code types its letter', 'VK:48 VK:49 S-Enter', 'HI', @{}),
    @('#4 Unicode-injected characters pass, committing first', 'KeyH KeyI U+0078', 'HIx', @{})
)
# The public API (docs/api.md), read by bw_testpad from the IME's DLL at @state: GetStaged's result
# and text, then GetState's. name, target, keys, a pattern the result must match.
$apiCases = @(
    @('API: what''s staged', 'tsf', 'KeyH KeyI @state Escape', '(?s)^2 HI\n\d+ \{(?=.*"staged":"HI")(?=.*"secure":false)'),
    @('API: a password field is a security error', 'pwscope', 'KeyH KeyI @state', '(?s)^-3 \n\d+ \{(?=.*"secure":true)(?=.*"staged":"")'),
    @('API: muhenkan, nothing staged', 'tsf', 'NonConvert @state', '(?s)^0 \n\d+ \{(?=.*"mode":"muhenkan")')
)

$names = 'CapsLock', 'CapsRule', 'PassOn', 'PublishState'
$saved = @{}
foreach ($n in $names) { $saved[$n] = (Get-ItemProperty $key -Name $n -ErrorAction SilentlyContinue).$n }
$capsWas = [Console]::CapsLock
$failed = 0
$out = Join-Path $env:TEMP 'bw-type-test.txt'
$stateFile = Join-Path $env:TEMP 'bw-type-test-state.txt'
function Run($targetName, $keys, $settings) {
    foreach ($n in 'CapsLock', 'CapsRule', 'PassOn') { Remove-ItemProperty $key -Name $n -ErrorAction SilentlyContinue }
    Set-ItemProperty $key PublishState 1 -Type DWord
    foreach ($k in $settings.Keys) { Set-ItemProperty $key $k $settings[$k] }
    Remove-Item $out, $stateFile -ErrorAction SilentlyContinue
    $keys = $keys -replace '@state', "@state=$stateFile"
    $p = Start-Process $pad -ArgumentList (@('--target', $targetName, '--keys', "`"$keys`"", '--out', "`"$out`"") + $ime) -PassThru
    if (-not $p.WaitForExit(30000)) { $p.Kill(); return '<timeout>' }
    if (Test-Path $out) { [IO.File]::ReadAllText($out, [Text.Encoding]::UTF8) } else { '<no output>' }
}
try {
    foreach ($c in $cases) {
        if ($Filter -and $c[0] -notlike "*$Filter*") { continue }
        foreach ($t in $Target) {
            $got = Run $t $c[1] $c[3]
            if ($got -ceq $c[2]) { Write-Host "ok   $($c[0]) [$t]" }
            else { Write-Host "FAIL $($c[0]) [$t]: got [$got], expected [$($c[2])]" -ForegroundColor Red; $failed++ }
        }
    }
    foreach ($c in $apiCases) {
        if ($Filter -and $c[0] -notlike "*$Filter*") { continue }
        Run $c[1] $c[2] @{} | Out-Null
        $got = if (Test-Path $stateFile) { [IO.File]::ReadAllText($stateFile, [Text.Encoding]::UTF8) } else { '<no state>' }
        if ($got -cmatch $c[3]) { Write-Host "ok   $($c[0]) [$($c[1])]" }
        else { Write-Host "FAIL $($c[0]) [$($c[1])]: got [$got]" -ForegroundColor Red; $failed++ }
    }
} finally {
    foreach ($n in $names) {
        if ($null -eq $saved[$n]) { Remove-ItemProperty $key -Name $n -ErrorAction SilentlyContinue }
        else { Set-ItemProperty $key $n $saved[$n] }
    }
    if ([Console]::CapsLock -ne $capsWas) { Write-Warning 'Caps Lock was left changed: press it once.' }
}
if ($failed) { Write-Host "$failed failed" -ForegroundColor Red; exit 1 }
Write-Host 'all passed'

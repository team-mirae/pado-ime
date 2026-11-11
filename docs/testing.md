# Building and testing

## What runs where

| | macOS / Linux | Windows |
|---|---|---|
| Core tests (JSON, keys, manifests, the host logic typed into shout, the state table) | `tools/test-core.sh` | `tools/build.ps1` runs them for x64 and x86 |
| The DLL, testpad, API reader, unload test | | `tools/build.ps1` |
| Typing through the installed IME | | `tools/type-test.ps1` (a desktop; don't touch the keyboard) |
| CI | the core tests | builds x64 + x86 and runs the core tests (no desktop: no typing) |

Status at the first commit (2026-11-11, built on a Mac): the core tests pass (242 checks, ASan and
UBSan) against shout from bada-compatible `f00ca67`; the core and the API/state/settings/
registration sources compile with mingw-w64. The text service, the indicator, the testpad and the
unload test **haven't been compiled** (mingw-w64's TSF headers lack `ITfTextInputProcessorEx`,
`ITfMenu` and the language bar interfaces) and nothing has run on Windows yet: start at "First run
on Windows" below.

First Windows build (2026-11-11, Windows 11 Pro 26200, VS 2022 Build Tools, CMake 4.4, Rust 1.99.1):
everything compiles with MSVC for x64 and x86 with no warnings at `/W4`, the text service and the
testpad included. The core tests pass, 241 checks on both (the 242nd is macOS-only: the `.dylib`
name). `bw_unloadtest` passes on both without installing: 8 rounds, private memory flat at 2 MB.
Then installed on the owner's PC (`tools/install.ps1`: registered x64 + x86, added under 日本語) and
`tools/type-test.ps1` run: 16 of 54 failed at first; after the fixes in
[tsf-notes.md](tsf-notes.md#first-type-test-2026-11-11), **53 of 54 pass** (the one left: a Unicode-injected
character reaches a classic Edit before the commit; it can't be sent again). Not run yet: `bw_state --watch` by hand and
the **[measure]** items.

A development build that logs every key (virtual key, scan code, mode before and after, eaten) to the
diagnostic log: configure with `-DCMAKE_CXX_FLAGS="/DWIN32 /D_WINDOWS /GR /EHsc /DBW_KEY_LOG"` in a
build folder of its own. It logs keystrokes: never ship it, and turn the log off after.

## Toolchain (Windows 10 or 11)

| Tool | For | Install |
|---|---|---|
| Visual Studio 2022 Build Tools: MSVC x64/x86, Windows SDK | the DLL and the tests | `winget install Microsoft.VisualStudio.2022.BuildTools` then add "Desktop development with C++" |
| CMake 3.25+, Ninja | the build | both come with the Build Tools' C++ workload, or `winget install Kitware.CMake Ninja-build.Ninja` |
| Rust (rustup) with `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` | the engine | `winget install Rustlang.Rustup`, then `rustup target add i686-pc-windows-msvc` |
| Git | | `winget install Git.Git` |
| AutoHotkey v2 (optional) | the API example | `winget install AutoHotkey.AutoHotkey` |

`tools/dev.ps1` runs a command in a Visual Studio developer shell for one architecture (the build
scripts use it): `Launch-VsDevShell.ps1` wants `amd64`, not `x64` (inherited, FKeyIME).

## Test environment

Registering a TIP and changing the language list are system changes. Test them first in a disposable
Windows (inherited advice, FKeyIME `AGENTS.md`):

- **Windows Sandbox** (Windows 11 Pro; enable the feature, restart). `tools/sandbox/bada-windows.wsb`
  maps the repository's `build\` folder into the Sandbox read-only and opens it; inside, run
  `tools\sandbox\in-sandbox.ps1` from the mapped folder: it installs the build (no Visual Studio
  needed in the Sandbox), adds the profile and runs the type tests. Keep `vGPU` off: with it on the
  Sandbox stayed black on the owner's PC (FKeyIME).
- **A Hyper-V VM** for anything that needs a sign-out or reboot (the Sandbox loses everything when
  closed): sign-in behaviour, "apps keep the old DLL".
- On a real PC: `tools/install.ps1` and `tools/install.ps1 -Uninstall`. Never inject Win keys there
  (FKeyIME: a stuck Start menu held the mouse; `Stop-Process -Name StartMenuExperienceHost` frees it).

## First run on Windows

```powershell
git clone https://github.com/team-mirae/bada-windows; cd bada-windows
rustup target add i686-pc-windows-msvc
tools/build.ps1            # engine + DLL for x64 and x86, core tests; build/ime/shout/
tools/install.ps1          # UAC: Program Files\bada-windows\shout, regsvr32 x64 + x86, Japanese keyboards
```

Then pick **shout (bada-windows)** with Win+Space (it's under 日本語) and type in Notepad: `hello`
shows HELLO, Tab gives hello, Space commits. 無変換 / 変換 switch mode; the indicator shows SH / sh.
**Restart apps that were running** before the install: they keep no DLL, or the old one (inherited).

Then, in this order:

1. `tools/type-test.ps1`: every case in the three controls, plus the API cases.
2. `build\x64\bw_unloadtest.exe {9C0F2BF6-0B78-447E-A99D-2FBAE9E8995E} build\ime\shout\x64\bada_windows.dll`:
   memory mustn't grow across rounds (the engine is freed in `DllCanUnloadNow`).
3. `build\x64\bw_state.exe {9C0F2BF6-0B78-447E-A99D-2FBAE9E8995E} --watch` while typing in Notepad,
   in a browser's password field, and in a Store app (docs/api.md's **[measure]**).
4. `tools/install.ps1 -Log`: the log while you use it.
5. The **[measure]** items of docs/spec.md §10, by hand on a JIS keyboard (injected keys can't stand
   in for physical language keys, inherited).

What a failure means, and what was tried before, is in [tsf-notes.md](tsf-notes.md); add what you
measure there.

## Making a test case

`tools/type-test.ps1`'s `$cases`: a name, keys, the expected text, and registry settings for the case.
Keys are `KeyboardEvent.code` names sent by scan code, with `S-` `C-` `A-` prefixes; `NonConvert`,
`Convert`, `KanaMode`, `Lang1`, `Lang2`, `CapsLock`; `VK:XX`; `U+XXXX`; `@state` (the API's answers).
Cases end committed: classic controls show only the first character of an open composition.

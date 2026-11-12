# pado ime on Windows

pado ime is bada ime on Windows. Its engine, layouts, data, ranking, menu, F-key actions, profiles,
dictionary and settings file are bada's, unchanged. Only the shell differs: bada's macOS
InputMethodKit app is replaced by a TSF text input processor.

| Part | Where | What |
|---|---|---|
| The engine | `engine/` | bada's, from upstream unchanged (`bada-engine`, C ABI `engine/include/bada.h`) |
| The engine as a DLL | `windows/pado/engine/` | A cdylib that re-exports `bada::ffi`: `pado_engine.dll` |
| The input method | `windows/host/` | [bada-windows](https://github.com/team-mirae/bada-windows) (a git subtree): one TSF text input processor, `bada_windows.dll`, that hosts any bada-compatible engine named in a manifest. Its own spec: [windows/host/docs/spec.md](../windows/host/docs/spec.md) |
| The IME | `windows/pado/ime.json` | pado's manifest: the engine, its 11 data buffers, its permanent GUIDs, its labels (파 / A) and its settings program |
| Settings | `windows/settings/` | `pado-settings.exe`: bada's settings window as a local page (below) |
| Build, install, tests | `windows/build.ps1`, `windows/install.ps1`, `windows/test.ps1`, `windows/tests/` | |
| The installer | `windows/installer.ps1`, `windows/installer/` | Inno Setup 6: `pado-ime-<version>-setup.exe`, released on GitHub by a tag (below) |

Names: **pado ime**, 파도 입력기, パドIME (pado: 파도, a wave). The layouts keep their names: bada plain
(16, the default), bada roman (15), bada (13).

## Install

Windows 10 or 11, x64. **The installer:** `pado-ime-<version>-setup.exe` from
[Releases](https://github.com/team-mirae/pado-ime/releases) (UAC). It does what `windows/install.ps1`
does: `Program Files\bada-windows\pado`, registered for 64- and 32-bit apps, the log folder, and, as
the user who started it, pado ime added to 日本語's keyboards (`windows/installer/user.ps1`). A DLL an
app has loaded is renamed out of the way, so an update needs no restart; apps load the new one when
they restart. Uninstall (Settings → Apps) undoes it all; your settings and dictionary stay.

**Releasing:** a tag `v<version>`, the engine's version in `engine/Cargo.toml`, pushed to GitHub:
`.github/workflows/windows.yml` builds, tests, builds the installer and publishes the release, with
`windows/installer/notes/v<version>.md` as its notes. `windows/installer.ps1` builds the installer
here (Inno Setup 6: `winget install JRSoftware.InnoSetup`); `windows/sandbox.ps1 -Setup` tries it in
Windows Sandbox.

**From source:**

| Tool | Install |
|---|---|
| Visual Studio 2022 Build Tools, "Desktop development with C++" | `winget install Microsoft.VisualStudio.2022.BuildTools` |
| Rust, with the 32-bit target | `winget install Rustlang.Rustup`, then `rustup target add i686-pc-windows-msvc` |
| CMake and Ninja | with the Build Tools' C++ workload, or `winget install Kitware.CMake Ninja-build.Ninja` |

```powershell
git clone https://github.com/team-mirae/pado-ime; cd pado-ime
windows/build.ps1      # engine + TIP for x64 and x86, settings, data: windows/host/build/ime/pado
windows/test.ps1       # pado's tests (no install needed)
windows/install.ps1    # UAC: Program Files\bada-windows\pado, registered for 64- and 32-bit apps
```

Then pick **pado ime** with Win+Space (it's listed under 日本語). Restart apps that were running.
`windows/install.ps1 -Update` copies a new build over it; `-Uninstall` removes it (your settings and
dictionary stay).

## Keys

bada's keys (spec §6), as bada-windows maps them to a PC JIS keyboard
([host spec §4](../windows/host/docs/spec.md#4-language-keys)):

| bada on a Mac | pado on Windows |
|---|---|
| 英数: muhenkan | **無変換** (and an Apple keyboard's 英数; under Windows' Japanese layout the Caps Lock key alone is 英数) |
| かな: henkan, profile `mac-kana`; in henkan, the next menu entry | **変換**: henkan, profile `henkan`; in henkan, the next menu entry. 変換 is the home key: with profiles off, every key gives the same henkan |
| 変換, カタカナ (recorded, through Input Monitoring) | **変換**, **カタカナ/ひらがな**: fixed, no recording. An Apple keyboard's かな brings up `mac-kana` |
| MEH + かな: the other mode | MEH (Ctrl+Alt+Shift) + 変換 / カタカナ: the other mode |
| ⌃かな: fixes what shows (spec §6.3) | **Ctrl+変換** (or Ctrl + another profile key), in henkan |
| Tab, ↓ / ⇧Tab, ↑, ⇧Enter, Esc, Space, ⌥Space, 半角/全角, F6–F10, ⌥ + letter | the same, with Alt for ⌥: all of it is the engine's |
| Caps Lock: bada's (toggle by default) | Caps Lock: Windows' by default; Settings → Layouts → Caps Lock has bada's toggle and transient ([host spec §5](../windows/host/docs/spec.md#5-caps-lock)) |

The indicator in the taskbar shows **파** in henkan and **A** in muhenkan. Click it to switch; right-click
for the menu: mode, profile, Caps Lock, off in this program, **Settings…** and **Open Dictionary Folder**.

## Settings

**Settings…** in the indicator's menu starts `pado-settings.exe`. It serves bada's settings window
(the Mac's pages: Layouts, Profiles, Menu, F Keys, Dictionary, Backup, plus Windows and About) on
127.0.0.1 and opens it in an Edge app window. The page's model is bada's own `web/src/settings.js`,
compiled in from upstream, so its defaults, normalisation and `bada-settings/1` file are the Mac's:
a file exported on the Mac or at bada.sjun.me imports here and back.

Settings are kept in `HKCU\Software\bada-windows\pado`: `Settings` (the whole JSON),
`EngineSettings` (what the engine gets), `CapsLock` / `CapsRule`, and the host's own values
([host spec §9](../windows/host/docs/spec.md#9-settings)). Apps pick a change up at their next focus
change. The server answers only the random token in its URL and only to `127.0.0.1` by number, and
exits when the window closes.

## The dictionary

bada's user dictionary ([dictionary-contract.md](dictionary-contract.md)) lives in
`%APPDATA%\bada-windows\pado\dictionary.json` (the menu's Open Dictionary Folder). Edit it there or on
the settings window's Dictionary page, which checks it with the engine's own parser and saves only a
file that loads. As on the Mac, a changed file is loaded again at the next focus change.

## State for other programs

bada's `bada_state` (spec §9) is published by bada-windows per thread, with its own fields under
`host`, for AutoHotkey and the like: [windows/host/docs/api.md](../windows/host/docs/api.md). Off in
password fields.

## What isn't the Mac's

- **Memory.** Each program that types with pado loads the engine and its data: about 140 MB
  (measured, x64, `windows/tests/pado_tests.cpp`) and 0.6–1.2 s on the thread pool the first time.
  On the Mac one process holds it once. Sharing the data across processes needs the engine to read
  it from a mapped file: a change upstream.
- **Store (AppContainer) apps** can't read `%APPDATA%`: they type without your dictionary. Settings
  reach them (the registry key grants them read).
- **The Keyboard Viewer's bada legends** (`bada.keylayout`) have no Windows counterpart.
- Everything bada-windows marks **[measure]** or **[decide]** (its spec §10) applies: Apple JIS
  keyboards' 英数 / かな scan codes, the Caps Lock injection, a key for `mac-kana` on PC keyboards.
- Not built: ARM64. The installer isn't signed: SmartScreen warns about it until it's known.

## Testing

| | |
|---|---|
| `windows/build.ps1` | builds everything and runs bada-windows' core tests (188 checks per architecture) |
| `windows/test.ps1` | `windows/tests/pado_tests.cpp` (the built IME loaded as the TIP loads it; QWERTY, 2-set, bada roman, bada plain, kana, profiles, Ctrl+変換, 半角/全角, the dictionary, typed through the host into a text field; x64 and x86) and the settings server (page, API, token and host guards) |
| `cargo test` in `engine/` | bada's own tests (`-EngineTests` on `windows/build.ps1`); the typing robot: [robot.md](robot.md) |
| `windows/sandbox.ps1` | installs the build in Windows Sandbox and runs `windows/type-test.ps1` there, without touching this PC; results in `windows/host/build/sandbox/results.txt`. `-Keep` leaves the Sandbox open to try pado by hand; `-Setup` installs with the installer |
| `windows/type-test.ps1` | types bada's layouts through the **installed** TIP into bada-windows' test window (a classic Edit, a RichEdit through the IMM layer, a RichEdit on TSF): QWERTY with JIS symbols, 2-set, Esc and 無変換 committing, bada roman, kana with ⇧Tab to hiragana, 半角/全角, profiles, Caps Lock toggle, and the API (what's staged; a password field is a security error) |

Measured in Windows Sandbox (Windows 11 24H2, 2026-11-11): the install registers x64 and x86, the
engine loads in 0.7–1.2 s per program, and 28 of the 29 cases pass. The one left (無変換 in the classic
Edit) received no keys at all, twice, and passed in the run before: the test window wasn't in front
for the injected keys (the Sandbox's foreground lock, which `windows/sandbox-run.ps1` lifts as far as
it can). A case that gets no keys is tried once more. Not yet measured: by hand on a real JIS keyboard.

Registering a TIP changes the system: try it in Windows Sandbox first ([windows/host/docs/testing.md](../windows/host/docs/testing.md)).

## Staying a fork

pado is a GitHub fork of [team-mirae/bada-ime](https://github.com/team-mirae/bada-ime), and
`windows/host` is a subtree of [team-mirae/bada-windows](https://github.com/team-mirae/bada-windows).
`.github/workflows/sync-upstream.yml` runs daily (and by hand). For each upstream it merges new work into
a branch (`sync/bada-ime`, `sync/bada-windows`) and opens a pull request:

- bada-ime: upstream's changes to what pado removed (`mac/`, the Mac and web-deploy workflows) are
  dropped, and pado keeps its own `README.md` and `CLAUDE.md`. Any other conflict stops the sync and
  opens an issue: merge `upstream/main` by hand.
- bada-windows: `git subtree pull --prefix=windows/host … --squash`.

By hand:

```sh
git remote add upstream https://github.com/team-mirae/bada-ime.git
git fetch upstream && git merge upstream/main
git subtree pull --prefix=windows/host https://github.com/team-mirae/bada-windows.git main --squash
```

A change upstream that pado needs on Windows: the Mac shell's behaviour lives in `web/src` too
(CLAUDE.md); a change to `web/src/settings-ui.js` needs the same change in
`windows/settings/page/ui.js`, and a new setting in `web/src/settings.js` reaches the settings window by
itself.

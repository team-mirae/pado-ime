# bada-windows: spec

The Windows host for **bada-compatible** engines: one TSF text input processor
(`bada_windows.dll`) that loads any engine speaking the C ABI of
[bada-compatible v1.1](https://github.com/team-mirae/bada-compatible/blob/main/docs/bada-compatible-v1.md)
(bada ime 0.2.0's, plus the optional `P_state`) and does on Windows what bada ime's macOS shell does
on a Mac: language keys, modes, profiles, Caps Lock, the menu, and the text in the app.

Facts are marked **measured** (on Windows, by someone), **inherited** (measured in jis-moonshot or
FKeyIME, same TSF code), or **[measure]** (written without a Windows machine; check first).
**[decide]** marks a choice the owner hasn't made.

- [1. Shape](#1-shape)
- [2. The manifest](#2-the-manifest)
- [3. One input method, two modes](#3-one-input-method-two-modes)
- [4. Language keys](#4-language-keys)
- [5. Caps Lock](#5-caps-lock)
- [6. Keys the engine passes on](#6-keys-the-engine-passes-on)
- [7. Text out](#7-text-out)
- [8. Privacy](#8-privacy)
- [9. Settings](#9-settings)
- [10. Open questions](#10-open-questions)

## 1. Shape

| Part | Where | What |
|---|---|---|
| The core | `src/core/` | Portable C++20, no Windows headers: JSON, the Output object, scan codes → `KeyboardEvent.code`, the manifest, loading an engine (`LoadLibraryW` / `dlopen`), the **host logic** (`host.*`: bada-compatible §7 adapted as below) and the state table. Unit-tested on any OS against shout |
| The shell | `src/tip/` | COM and TSF: key events in, the core's Outputs out as a TSF composition, the menu window, the indicator, settings in the registry, the published state and its API |
| An IME | its own folder | `ime.json` and the engine's DLL. `examples/shout/` is the template's: bada-compatible's shout, unchanged |

All typing behaviour is the engine's; all key policy is the core's; the shell only moves keys in and
text out. The engine is **not linked**: the DLL reads its manifest and loads the engine at run time,
so one build of bada-windows hosts any engine, and several IMEs install side by side, each a copy of
`bada_windows.dll` with its own manifest and GUIDs.

### Installed layout

```
%ProgramFiles%\bada-windows\<name>\
  ime.json                      the manifest
  x64\bada_windows.dll          the TIP, for 64-bit apps
  x64\<engine>.dll              the engine (static CRT), for 64-bit apps
  x86\bada_windows.dll          the same for 32-bit apps
  x86\<engine>.dll
  <data files the manifest lists>
```

`bada_windows.dll` looks for `ime.json` in its own folder, then one up.

## 2. The manifest

superkeyboard's manifest (`team-mirae/superkeyboard`, `docs/ime-contract.md` §8; bada-compatible
§9), unchanged, plus a `windows` object:

| Field | | |
|---|---|---|
| `name`, `description`, `library`, `prefix`, `data`, `settings`, `mode`, `keyboard` | superkeyboard's | `library` takes `{arch}` (`x64`, `x86`, `arm64`) besides `{ext}`; relative paths start at `root` (default: the manifest's folder). `data` is loaded once per process on the thread pool. `settings` are every engine's starting settings. `mode` is the mode at the start of a session. `keyboard: "us"` makes Windows' layout type passed-on keys by default (§6) |
| `windows.clsid` | required | The TIP's COM class. **Permanent**: in users' registries and language lists |
| `windows.profile` | required | The language profile's GUID. **Permanent** |
| `windows.language` | `"ja-JP"` | The language the profile is listed under (§2.2) |
| `windows.display_name` | `name` | The name in Windows' language list and the indicator |
| `windows.labels` | `{"henkan": "あ", "muhenkan": "A"}` | The indicator's text per mode |
| `windows.icon` | the DLL's | An `.ico` for the language list |
| `windows.settings_app` | none | A program (relative to `root`) the menu's **Settings…** starts: an IME's own settings window |

Make new GUIDs for a new IME: `[guid]::NewGuid()` in PowerShell, twice.

### 2.2 Why Japanese

A TIP is listed under one language, and Windows types with that language's keyboard layout under it.
Under **ja-JP** that's the Japanese layout (kbd106 on a JIS keyboard), which gives the JIS language
keys their own virtual keys (`VK_NONCONVERT`, `VK_CONVERT`, `VK_DBE_*`, `VK_OEM_AUTO`) and types JIS
symbols when a key reaches the app (FKeyIME: only its Japanese profile typed JIS symbols, measured;
under Korean and English, Windows' US layouts typed). bada-windows maps keys by scan code anyway
(§4), so another language works too (`"ko-KR"`, `"en-US"`), with Windows' layout for that language
under it.

## 3. One input method, two modes

bada ime on macOS registers two input sources (Bada Muhenkan, Bada Henkan) and lets macOS switch
between them. **bada-windows registers one TSF profile, which handles both modes itself.**

- The mode is the text service's state, **one per session**: every app shares it (a TSF global
  compartment keyed by the IME's CLSID, never stored). A session starts in the manifest's `mode`.
  A switch in one app commits what's staged there; the others follow when they next see it, each
  committing its own (`Host::Follow`).
- With each mode the text service also keeps the henkan **profile** (§4.2), in the same compartment.
- The open/close compartment mirrors the mode (open: henkan) for the input indicator and apps that
  read it. A change of it from elsewhere is put back (inherited: Windows closes it when a profile
  activates).
- **The indicator** (taskbar) shows the manifest's label for the mode. A click switches mode; a right
  click opens the menu (§9).
- Every switch calls `P_set_mode` (which commits what's staged), then `P_set_profile` with the current
  profile if the engine has it (bada-compatible §7.4).

## 4. Language keys

The host handles these itself (bada-compatible §7.4); they reach the engine only as `Lang1`, "next
menu entry" (⇧: previous). They're recognised by **scan code**, so a key works whatever layout Windows
loaded; by virtual key only for injected keys without a scan code.

| Key | Scan code | What it does |
|---|---|---|
| 無変換 | `0x7B` | Muhenkan mode (bada's 英数) |
| 変換 | `0x79` | Henkan mode, profile `henkan`. In henkan: §4.2 |
| カタカナ/ひらがな | `0x70` | Henkan mode, profile `katakana`. In henkan: §4.2 |
| 英数 (an Apple JIS keyboard's) | `0xF1` / `0x71` **[measure]** | Muhenkan mode |
| かな (an Apple JIS keyboard's) | `0xF2` / `0x72` **[measure]** | Henkan mode, profile `mac-kana` (setting `KanaKey`; `none` passes it to the app) |
| The Caps Lock key | `0x3A` | §5 |
| ⌃⌥⇧ (MEH) + a profile key | | The other mode (bada's ⌃⌥⇧かな) |

- Held down, a language key acts once (auto-repeats are eaten and ignored).
- A profile key with ⌃, ⌥ or ⊞ (but not all of ⌃⌥⇧) is eaten and does nothing.
- 半角/全角 (`0x29`) is not a language key: it's the engine's `Backquote` (bada's full-width /
  half-width; shout's "shout the selection"). Under the Japanese layout it arrives as `VK_OEM_AUTO` /
  `VK_OEM_ENLW`, and ⌥ + it as `VK_KANJI` (inherited, FKeyIME); all are `Backquote`.

### 4.2 Henkan profiles (the owner's adaptation)

bada's profiles (bada `docs/spec.md` §6.6) are `mac-kana` (かな), `henkan` (変換) and `katakana`
(カタカナ), with かな the home key. **On Windows, 変換 is the home key and `henkan` the default
profile**: PC JIS keyboards have 変換 and no かな.

- In muhenkan, a profile key enters henkan in its profile.
- In henkan, the key of the profile in use moves to the next menu entry (⇧: previous); so does 変換
  in another profile while a menu of more than one row shows (bada's `next-or-mac-kana` default, with
  変換 in かな's place). Otherwise a profile key switches to its profile, committing what's staged.
- **`mac-kana` has no key of its own on a PC keyboard.** The owner suggested Right Alt: a TIP never
  sees a Right Alt, Right Win or Right Ctrl tap (inherited, FKeyIME
  `docs/issues/modifier-taps-unreachable.md`: the key sink isn't called, and preserved keys don't
  fire), so it can't be a profile key without a system-wide remap. An Apple JIS keyboard's かな key
  gives it (setting `KanaKey`, default on), and the menu can pick it. **[decide]** whether another
  key should (the Menu key fires as a preserved key, FKeyIME measured).
- An engine without `P_set_profile` (shout) has one henkan: every profile key enters it, and in
  henkan moves in the menu.

## 5. Caps Lock

Caps Lock is the host's (bada-compatible v1.1 §7.7): the engine never sees it, and events carry no
Caps Lock state. **Handling it is optional, and off by default** (setting `CapsLock`):

| Setting | Caps Lock | ⇧ ⌃ ⌥ + Caps Lock |
|---|---|---|
| `respect` (**default**) | Windows' own | Windows' own |
| `toggle` | Switches mode (bada's default on macOS). Leaves Caps Lock off | Caps Lock, in muhenkan only; in henkan nothing |
| `transient` | Caps Lock in muhenkan; in henkan nothing, and kept off | as Caps Lock |

- With `toggle` or `transient`, leaving muhenkan turns Caps Lock off, and in henkan it's turned off
  if another program turns it on.
- **Under the Japanese layout the unshifted Caps Lock key is 英数** (`VK_DBE_ALPHANUMERIC`): no lock
  changes, and Caps Lock itself is ⇧ + it. So with `respect` that key is 英数 (muhenkan), and with
  `toggle` it switches mode without Caps Lock ever turning on. Under other layouts it's `VK_CAPITAL`.
- Windows changes the lock state when the key goes down, before the IME sees it; turning it back
  takes an injected press (`SendInput`, as FKeyIME did), which the text service lets through when it
  comes back. **[measure]** whether `GetKeyState` already holds the new state during that key-down,
  and how this behaves with an elevated app in front (UIPI drops injected input there).
- **What Caps Lock types** (§6): with it on, passed-on letters are capitals, ⇧ or not
  (bada-compatible v1.1 §7.3, as macOS: not Windows' ⇧-inverts). `CapsRule = uppersnake` (bada's
  UPPER_SNAKE, muhenkan only): Space flushes and types `_`, ⌥Space a space, without `P_key`.

## 6. Keys the engine passes on

A key the engine doesn't consume reaches the app after the commit (bada-compatible §7.2: commit, then
act). A plain or ⇧ character key (the 48 JIS positions) is then **typed by the host from its JIS
table** (bada-compatible §7.3: `!` `"` on ⇧1 ⇧2, `@` `[` `:` `]`, ¥, ろ as `\` / `_`), whatever layout
Windows has: the key is eaten and its character inserted. With `PassOn = windows` (or the manifest's
`keyboard: "us"`) the app gets the key and Windows' layout types it. Shortcuts (⌃, ⌥, ⊞), Space,
Enter, Tab, Backspace, arrows, F keys and the keypad always go to the app.

A modifier pressed on its own never reaches the engine (it belongs to the next key and must not
commit: inherited, FKeyIME). Keys without a `KeyboardEvent.code` the core knows go as
`"Unidentified"`; characters injected as Unicode (`VK_PACKET`) too, so they commit first and reach
the app as typed (inherited, jis-moonshot #4).

## 7. Text out

Each Output is applied as bada-compatible §7.1 says, in one TSF edit session: `commit` replaces the
composition (or, with `replace_selection`, the selection, leaving what's marked as shown); `preedit`
becomes the composition, underlined (thick when the menu shows an entry other than entry 0). Then:

- **`need_selection`**: the host reads the selected text (a read-only session; `""` while something is
  marked) and answers with `P_host_selection`, once (§7.5 there).
- **The menu**: a topmost window under the composition that never takes focus and is **excluded from
  screen capture**. Rows above `zero` (negative entries) are set off by a line; `notes` show beside
  their rows; `japanese` rows are drawn in Yu Gothic UI; `horizontal` menus are one row of cells.
  The contract has no "pick a row": clicks do nothing.
- Everything jis-moonshot measured about TSF stays (docs/tsf-notes.md): the engine runs in
  `OnTestKeyDown`; keys not consumed are committed and reported as not eaten (but through the IMM
  layer, a key without modifiers that ends a composition is eaten and sent again after the commit:
  a ja-JP profile's commit would reach the app after it, docs/tsf-notes.md); a key-down tested twice
  runs once; apps through the IMM layer get an interim selection; text an app refuses to compose
  stays as plain text and is replaced at the next key; edit sessions hold the service and never call
  the engine; focus changes commit what's staged.

## 8. Privacy

- **Password fields** (an `ES_PASSWORD` Edit, a password character, a disabled keyboard, an empty
  context, an input scope `IS_PASSWORD` / PIN / private): nothing composes there (only §6's JIS
  characters are typed), nothing typed is published (§ docs/api.md: `host.secure`), and asking the
  API for the staged text is a **security error** (`BADA_WINDOWS_E_SECURE`). Language keys still
  switch mode. Windows keeps TIPs out of most marked password fields altogether (inherited).
- **Off in a program** (the menu; on by default in Korean certificate programs' agents, jis-moonshot's
  list): nothing composes there.
- The menu window is never in screen recordings, screenshots or screen sharing.
- The diagnostic log (off by default) records activations, focus and errors, never typed text.

## 9. Settings

`HKCU\Software\bada-windows\<name>`, else the same key under `HKLM`, else the default. The indicator's
menu writes them; programs pick a change up at their next focus change (`SettingsVersion`).

| Value | Type | Default | |
|---|---|---|---|
| `CapsLock` | SZ | `respect` | `respect`, `toggle`, `transient` (§5) |
| `CapsRule` | SZ | `allcaps` | `allcaps`, `uppersnake` (§5) |
| `PassOn` | SZ | `jis` (`windows` if the manifest's keyboard is `us`) | who types passed-on character keys (§6) |
| `KanaKey` | SZ | `mac-kana` | the profile an Apple keyboard's かな key brings up, or `none` (§4.2) |
| `EngineSettings` | SZ | | a JSON object; its top-level fields replace the manifest's `settings` (superkeyboard's rule) |
| `PublishState` | DWORD | 1 | publish the state for other programs (docs/api.md) |
| `DiagnosticLog` | DWORD | 0 | `%ProgramData%\bada-windows\<name>.log` |
| `Apps\<program.exe>\Off` | DWORD | | 1: nothing composes in that program |
| `SettingsVersion` | DWORD | | changes with every write from the menu |

The user dictionary, for engines with `P_dictionary_load`: `%APPDATA%\bada-windows\<name>\dictionary.json`,
read when the engine loads, and again at a focus change after the file changed or went away (its
time is compared; removed: the dictionary is cleared). The menu's **Open Dictionary Folder** opens its
folder.

## 10. Open questions

- **[measure]** Everything in §4's table marked so, and the virtual keys the Japanese layout gives
  カタカナ/ひらがな (scan `0x70`) with and without ⇧.
- **[measure]** §5's Caps Lock injection.
- **[measure]** Whether TSF tells the text service about a focused password field it keeps the TIP
  out of (`OnSetFocus` with that document), which is what makes the API answer `E_SECURE` there.
- **[decide]** A key for `mac-kana` on PC keyboards (§4.2).
- **[decide]** An installer (jis-moonshot's Inno Setup one would fit: one per IME, from its manifest).
- Not built: ARM64 (the code doesn't block it: `{arch}` is `arm64`, add a preset).

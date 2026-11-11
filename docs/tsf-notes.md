# TSF: what the shell does, and why

The text service is jis-moonshot's (team-mirae/jis-moonshot `windows/src/text_service.cpp`), whose
behaviour was measured on Windows 11 25H2 in three kinds of control. Read its
[`docs/troubleshooting/windows-tsf-notes.md`](https://github.com/team-mirae/jis-moonshot/blob/main/docs/troubleshooting/windows-tsf-notes.md)
before changing `src/tip/text_service.cpp`: each of these was "fixed" the obvious way first, and the
obvious way broke typing. FKeyIME's [`docs/issues/`](https://github.com/themrsung/fkeyime/tree/main/docs/issues)
(archived) has the problems before that.

## Kept as measured there

| What | Why (where) |
|---|---|
| The engine runs in `OnTestKeyDown`; a key it doesn't consume is committed there and reported as not eaten | A key eaten in `OnTestKeyDown` never reaches the app (moonshot) |
| `OnKeyDown` takes the cached result, comparing virtual key, scan code and extended flag only | Its lParam has the repeat count cleared; IMM-layer apps call it for keys not eaten (moonshot) |
| A key-down tested twice runs the engine once (`SameKeyDown`) | Message loops that peek and test (moonshot #5) |
| IMM-layer (transitory) contexts get the composition as an interim selection, set on the first character | A plain caret ends the composition at once there (FKeyIME, moonshot) |
| Text the app refuses or ends right after our update stays as plain text and is replaced next key | Apps that end every composition (moonshot #6) |
| Edit sessions hold the service, the context and their composition, and never call the engine | Queued sessions could run after `Deactivate` (moonshot #7) |
| No context: the key is the app's, nothing stays staged | moonshot #3 |
| `VK_PACKET` commits first, then reaches the app | moonshot #4 |
| A virtual key with scan code 0 gets `MapVirtualKeyW`'s | Injected keys without scan codes (moonshot #9) |
| Modifiers alone never commit | FKeyIME `shift-splits-the-composition.md` |
| The open/close compartment only mirrors the mode; changes from elsewhere are put back | Windows closes it when a profile activates (moonshot) |
| Settings are re-read only when `SettingsVersion` changed; the key grants ALL APPLICATION PACKAGES read | moonshot #9; Store apps couldn't see the user's key (moonshot) |
| Background work holds the DLL until it returns (`FreeLibraryWhenCallbackReturns`) | The DLL unloaded under the preload (moonshot) |
| The menu window is excluded from capture | A certificate password showed in a recording (moonshot) |
| Password fields: ES_PASSWORD, password character, disabled keyboard, empty context, input scopes | moonshot |
| Restart apps after installing | They keep the old DLL, or none (FKeyIME `apps-dont-load-fkeyime.md`) |

## New here, not measured yet

| What | Why | Risk |
|---|---|---|
| The engine is a DLL loaded at run time from the manifest, and freed in `DllCanUnloadNow` | Any bada-compatible engine plugs in | Loader-lock rules: never free it in `DllMain` |
| The mode is a session-wide global compartment keyed by the CLSID, with the profile | One TIP for both modes (docs/spec.md §3) | Apps that are busy follow late |
| The language keys by scan code under the Japanese layout | docs/spec.md §4 | Apple keyboards' 英数 / かな scan codes are a guess |
| Caps Lock set back with an injected press | docs/spec.md §5 | `GetKeyState` timing; UIPI |
| The state table and its API | docs/api.md | AppContainer apps; secure-field detection at focus change |
| One profile under ja-JP | docs/spec.md §2.2 | Windows' Japanese IME hotkeys (半角/全角 toggles open/close) fighting the mirror |

Add what you measure to this file, with the date and how.

## First type test (2026-11-11)

Windows 11 Pro build 26200, the owner's PC (Korean + Japanese keyboards), `tools/type-test.ps1` with
injected keys. First run: 16 of 54 failed, the same each time. Diagnosed with a development build's
per-key log line (`BW_KEY_LOG`, docs/testing.md); now **53 of 54 pass**, the same in two runs.

### Fixed

| Symptom | Cause (measured) | Change |
|---|---|---|
| A case started in the last case's mode: 無変換 typed `hia…` for `HIa…` in rich/tsf after passing in edit; toggle typed `Ab` | `bw_testpad` clears the session's mode compartment (0), but activating TSF on its thread has already activated the TIP, which read the old mode; `OnChange` took 0 for "never set" and ignored it | `OnChange`: 0 is the manifest's mode |
| カタカナ's second press did nothing (`HI` for `hi`); the Caps Lock toggle switched only once (`ab` + a new line from Shift+Enter in muhenkan) | The Japanese layout never releases its state keys: カタカナ goes down as `VK_DBE_HIRAGANA` and the only key-up is a synthesized `VK_DBE_ALPHANUMERIC`, before the press. So every later press has lParam bit 30 ("already down") and was taken for an auto-repeat | Presses of `VK_DBE_ALPHANUMERIC` / `_KATAKANA` / `_HIRAGANA` are never repeats. **[measure]** whether a physical key held down repeats |
| 半角/全角 didn't shout the selection in edit/rich | Through the IMM layer the transitory context holds only the composition: the selection read 0 characters | `ReadSelection` falls back to the focused Edit's / RichEdit's own selection (`EM_GETSEL` + `WM_GETTEXT`, `EM_GETSELTEXT`); never a password control's |
| A TSF RichEdit with `IS_PASSWORD` published `"secure": false` (`-5` for `-3`) | `GetProperty(GUID_PROP_INPUTSCOPE)` fails: the input scope is an app property. Windows sends no key to the TIP there, so only the focus-change check can see it | `GetAppProperty` |

### The IMM layer delivered the passed-on key before the commit (fixed but for VK_PACKET)

In the classic Edit (and once in the IMM RichEdit) a key the engine passes on reaches the app
**before** the text committed for it, though the edit session runs synchronously in
`OnTestKeyDown` (no "queued" log line):

| Case | Control | Got | Expected |
|---|---|---|---|
| HELLO world | edit | ` HELLO` + new line + `world` | `HELLO world` + new line |
| HELLO world | rich | `HELLO worldw` (Enter lost, `w` doubled) | `HELLO world` + new line |
| Backspace edits, then deletes | edit | ` O` | `OK` |
| #4 Unicode-injected characters | edit | `xHI` | `HIx` |

Probes (`bw_testpad --target edit`): `KeyH Space` → ` H`, `KeyH KeyI Enter` → new line + `HI`; in
rich `KeyH KeyI Enter` → `HIH`. So it isn't the composition's length.

**It's the profile's language (measured 2026-11-11).** A copy of shout with new GUIDs registered
under ko-KR, same DLL and engine, passed all four cases in edit and rich; shout under ja-JP failed
them in the same run. jis-moonshot (ko-KR) never met this. So the IMM layer orders a Japanese TIP's
commit after the key that follows it. Registering under another language isn't an option (the
language keys and Windows' Japanese layout come with ja-JP, docs/spec.md §2.2).

**Decided (owner, 2026-11-11): eat the key and send it again after the commit** (moonshot's #7
option). In a transitory context, a key the engine doesn't consume that ends what was composing is
reported as eaten and sent again with `SendInput` (same virtual key, scan code, extended flag); the
copy is let through when it comes back within 500 ms, ahead of the "tested twice" check (it has the
original's lParam). Only keys without modifiers: sending a shortcut or Shift+key again would fake
modifiers the user may have released, so those keep the old order (no failing case: the shortcut
case passes). `VK_PACKET` can't be sent again: TSF gives it `wParam` `0xE7` and lParam `0x1`, no
character (measured). Result: 53 of 54 in two runs; left, `#4` in the Edit (`xHI` for `HIx`).
**[measure]** a physical key held down while composing in a classic control (its auto-repeats are
not sent again: nothing is composing then).

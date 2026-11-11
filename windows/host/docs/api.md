# The state, for other programs (API version 1)

bada-compatible v1.1 lets an engine report its state (`P_state`, §12 there) and asks its wrapper to
expose it, read-only, so users can script around the IME. bada-windows publishes it for every thread
where the IME is active, and any program in the session (AutoHotkey, a status bar, a test) reads it
through four functions of the IME's own DLL, and hears about changes through a window message.
Nothing a reader does changes the IME.

## What's published

The engine's `P_state` object as it is, with bada-windows' fields under `host`:

```json
{"format": "bada-compatible-state/1", "mode": "henkan", "profile": null,
 "staged": "HELLO", "preedit": "HELLO", "menu": ["HELLO", "hello"], "selected": 0, "zero": 0,
 "horizontal": false, "need_selection": null,
 "host": {"name": "bada-windows", "version": "0.1.0", "ime": "shout", "app": "notepad.exe",
          "mode": "henkan", "profile": null, "caps_lock": false, "secure": false, "off": false}}
```

- Top-level fields are the engine's (bada-compatible §12): `staged` is exactly what a flush would
  commit. An engine without `P_state` gets a stand-in from its last Output, with `staged: null`.
- `host.mode` / `host.profile` are the session's (§3 of docs/spec.md); `host.caps_lock` is Windows'
  Caps Lock; `host.app` the program the thread belongs to; `host.off`: the IME is off there.
- **Secure input**: in a password field (docs/spec.md §8) `staged`, `preedit` and `menu` are empty and
  `host.secure` is `true`; **asking for the staged text there is a security error**
  (`BADA_WINDOWS_E_SECURE`, and `GetLastError()` is `ERROR_ACCESS_DENIED`), never `""`.
- It's published after every key the IME handles, every mode or profile switch, and every focus
  change. Turn it off with the menu's "Show State to Other Programs" (`PublishState = 0`): the thread's
  slot is released and the API answers "not active".

## Finding the DLL

The IME's CLSID is its manifest's `windows.clsid` (shout's: `{9C0F2BF6-0B78-447E-A99D-2FBAE9E8995E}`):

```
HKLM\SOFTWARE\Classes\CLSID\{CLSID}\InprocServer32   (default value)
```

A 32-bit program reading this key gets the x86 DLL through registry redirection. Loading the DLL
this way doesn't activate the IME; the functions only read shared memory and map nothing between
calls, so a caller may load and unload the DLL for every call (AutoHotkey's `DllCall("<path>\F")`
does).

## Functions

C declarations: [include/bada_windows_api.h](../include/bada_windows_api.h). All `WINAPI` (stdcall on
x86), undecorated export names. Strings are UTF-16; `capacity` counts `wchar_t` with the terminating
NUL; with no buffer (NULL or 0) a function returns the length it needs.

| Function | Returns |
|---|---|
| `int BadaWindowsApiVersion(void)` | `1` |
| `int BadaWindowsQueryWindow(HWND window)` | The mode on the thread owning `window` (NULL: the focused window of the foreground thread): `1` henkan, `0` muhenkan, or an error below |
| `int BadaWindowsQueryThread(DWORD thread)` | The same for a thread id |
| `int BadaWindowsGetState(HWND window, wchar_t* buffer, int capacity)` | The JSON above; its length, or an error |
| `int BadaWindowsGetStaged(HWND window, wchar_t* buffer, int capacity)` | `staged`; its length (`""`: 0), or an error. **Secure input: `-3`** |
| `UINT BadaWindowsStateMessage(void)` | The registered message `"bada-windows.state"` |

| Value | Name | Meaning |
|---|---|---|
| `-1` | `BADA_WINDOWS_NOT_ACTIVE` | The IME isn't active on that thread, or doesn't publish there |
| `-2` | `BADA_WINDOWS_UNAVAILABLE` | No state table: the IME hasn't run in this session (or the reader can't open it, below) |
| `-3` | `BADA_WINDOWS_E_SECURE` | **Security error**: a password field. What's typed there is never given out |
| `-4` | `BADA_WINDOWS_E_BUFFER` | The buffer is too small (ask again: the state may have grown) |
| `-5` | `BADA_WINDOWS_E_UNKNOWN` | The engine doesn't report what's staged (no `P_state`) |
| `-6` | `BADA_WINDOWS_E_BUSY` | The state kept changing while being read: ask again |

## The message

`RegisterWindowMessage("bada-windows.state")` is posted to every top-level window (`HWND_BROADCAST`)
when a thread's mode, profile, secure input or "something is staged" changes (not at every key):
`wParam` the thread id, `lParam` the mode in bits 0–7 and flags in bits 8–15 (`1` secure input, `2`
off in the program, `4` composing, `8` staged text not reported). For the text, call `GetState` or
`GetStaged`.

## AutoHotkey

[examples/autohotkey/bada-state.ahk](../examples/autohotkey/bada-state.ahk): a tooltip on every
change, and Ctrl+Alt+S for what's staged, which throws `OSError(5)` in a password field:

```autohotkey
Dll := RegRead("HKLM\SOFTWARE\Classes\CLSID\{9C0F2BF6-0B78-447E-A99D-2FBAE9E8995E}\InprocServer32")
mode := DllCall(Dll "\BadaWindowsQueryWindow", "ptr", 0, "int")      ; 1 henkan, 0 muhenkan
OnMessage(DllCall(Dll "\BadaWindowsStateMessage", "uint"), OnBadaState)
```

`tests/state_reader.cpp` (`build\x64\bw_state.exe {CLSID} [--watch]`) is the same in C++.

## How it works, and who can read it

Each thread with the IME active holds a slot in a session-wide shared-memory table
(`Local\bada-windows.state.v1.{CLSID}`, `src/core/state_table.h`): claimed when it first publishes,
written under a sequence lock (readers never see half a state), released on deactivation; slots of
threads that died are ignored and reused. Version 1 of the layout; it only grows. FKeyIME's table
(its `docs/api.md`), with a JSON text per slot.

The table holds what's being typed, so it's readable and writable **only by the user's own processes
at medium integrity or above** (and SYSTEM). Low-integrity and AppContainer processes, sandboxed
browser processes among them, can't read it; for the same reason the IME running inside a Store app
can't publish there, and the API answers `-1` or `-2` for those threads. **[measure]**: which apps
that is in practice.

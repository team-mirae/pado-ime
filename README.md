# bada-windows

A Windows input method for **any bada-compatible engine**: one TSF text input processor that loads an
IME engine speaking [bada-compatible](https://github.com/team-mirae/bada-compatible)'s C ABI (v1.1,
bada ime 0.2.0's plus `P_state`) and hosts it the way [bada ime](https://github.com/team-mirae/bada-ime)'s
macOS shell hosts bada: JIS language keys, muhenkan and henkan modes, henkan profiles, optional Caps
Lock handling, the candidate menu, and the text in every app. The engine is a DLL named in a small
manifest; nothing is linked, so one build hosts any engine.

The example IME is **shout**, bada-compatible's template IME: henkan mode types CAPITALS, Tab or 変換
offer lowercase, 半角/全角 in muhenkan shouts the selection.

> [!WARNING]
> **Early on Windows.** It builds with MSVC for x64 and x86, the core tests pass against the real
> shout engine (241 checks on Windows, 242 on macOS), and the unload test passes. Typed through the
> installed TIP, 53 of 54 cases pass ([docs/tsf-notes.md](docs/tsf-notes.md#first-type-test-2026-11-11));
> not yet measured by hand on a JIS keyboard.
> Start at [docs/testing.md](docs/testing.md#first-run-on-windows).

## How it differs from bada ime on macOS

| | macOS (bada ime) | Windows (bada-windows) |
|---|---|---|
| Input sources | Two: Bada Muhenkan, Bada Henkan | **One** TSF profile that handles both modes (session-wide) |
| Muhenkan / henkan | 英数 / かな | **無変換 / 変換** (and an Apple keyboard's 英数 / かな) |
| Home profile | `mac-kana` (かな) | **`henkan` (変換)**; `mac-kana` only on an Apple keyboard's かな (Right Alt never reaches a TIP) |
| Caps Lock | bada's, toggle by default | **Optional**, Windows' by default; toggle / transient in the menu |
| State for other programs | | Published per thread; **AutoHotkey** reads it with `DllCall` ([docs/api.md](docs/api.md)); password fields are a security error |

Everything in detail: [docs/spec.md](docs/spec.md).

## Build and try it (Windows)

```powershell
rustup target add i686-pc-windows-msvc
tools/build.ps1          # shout's engine + bada_windows.dll, x64 and x86; core tests
tools/install.ps1        # UAC; then Win+Space → shout (bada-windows), under 日本語
tools/type-test.ps1      # types through it into a test window; don't touch the keyboard
```

On macOS / Linux, the core tests: `tools/test-core.sh` (needs a bada-compatible checkout next to this
one, or `BADA_COMPATIBLE=…`).

## Make a Windows IME from a bada-compatible engine

1. Make the engine (bada-compatible's README, "Make a new IME"). Export `state` for the API.
2. Copy `examples/shout/` to a folder of your own. In `engine/Cargo.toml`, depend on your engine crate
   and name the `[lib]` after it (the DLL's name).
3. In `ime.json`: `name`, `library` (`{arch}/<lib>.dll`), `prefix`, `data` (copied next to it),
   `settings`, and under `windows` **two new GUIDs** (`[guid]::NewGuid()`: they're permanent), the
   language and the indicator's labels.
4. `tools/build.ps1 -Ime path\to\folder`, `tools/install.ps1 -Name <name>`.

## Layout

| Path | What |
|---|---|
| `src/core/` | Portable C++20: JSON, Outputs, keys, manifests, the engine loader, **the host logic**, the state table. No Windows headers |
| `src/tip/` | The TSF text service, edit sessions, the menu window, the indicator, registry settings, registration, the public API |
| `include/bada_windows_api.h` | The API other programs call ([docs/api.md](docs/api.md)) |
| `examples/shout/` | The example IME: its manifest and a crate that builds bada-compatible's shout as `shout.dll` |
| `examples/autohotkey/` | Reading the state from AutoHotkey v2 |
| `tests/` | Core tests (any OS), the test window, the unload test, the API reader |
| `tools/` | `build.ps1`, `install.ps1`, `type-test.ps1`, `dev.ps1`, `test-core.sh`, `sandbox/`, `make-icon.py` |
| `docs/` | [spec](docs/spec.md), [api](docs/api.md), [testing](docs/testing.md), [tsf-notes](docs/tsf-notes.md) |

## License

[CC0 1.0](LICENSE), all of it, the parts adapted from jis-moonshot and FKeyIME included (their
authors' dedication). Engines keep their own licenses.

# bada-windows

- The Windows (TSF) host for bada-compatible engines. Source of truth: `docs/spec.md` (behaviour),
  `docs/api.md` (the state for other programs). If code and docs disagree, fix one in the same change.
  The engine contract is team-mirae/bada-compatible's `docs/bada-compatible-v1.md` (v1.3); keep up
  with it, don't fork it.
- `src/core/` stays free of Windows headers (only `engine.cpp`'s loader has an `#ifdef`): its tests
  run on macOS. Put key policy there (`host.*`), not in the text service.
- `src/tip/text_service.cpp` is jis-moonshot's measured TSF code: read `docs/tsf-notes.md` and
  jis-moonshot's `docs/troubleshooting/windows-tsf-notes.md` before changing how keys or edit sessions
  work. Mark facts measured / inherited / **[measure]**; don't implement **[decide]** items silently.
- GUIDs in manifests (`windows.clsid`, `windows.profile`) and in `src/tip/guids.h` are permanent.
- Names: bada (lowercase); muhenkan / henkan mode; bada-windows.
- License: CC0, all of it, code adapted from jis-moonshot or FKeyIME included (credited in file
  headers).
- Ask before registering the TIP or changing language settings on a real PC; test in Windows Sandbox
  (`tools/sandbox/`) or a VM first.

## Building

- macOS / Linux: `tools/test-core.sh` (core tests with ASan/UBSan, against shout from
  `../bada-compatible` or `$BADA_COMPATIBLE`).
- Windows: `tools/build.ps1` (engine + DLL for x64 and x86, core tests), `tools/install.ps1`,
  `tools/type-test.ps1`. Details: `docs/testing.md`.
- PowerShell scripts are UTF-8 **with BOM** (PowerShell 5.1 reads them as ANSI otherwise).

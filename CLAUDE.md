# pado ime

- **pado ime** (파도 입력기, パドIME) is bada ime on Windows: a GitHub fork of `team-mirae/bada-ime` with
  the macOS shell (`mac/`) removed and a TSF input method in its place. Names: pado (lowercase), パド
  (katakana), 파도. The layouts keep bada's names (bada, bada roman); the engine and its C ABI stay
  `bada` (`bada_*`).
- Source of truth: bada's `docs/spec.md` and `docs/layouts/13-bada.md` for behaviour, and
  [docs/windows.md](docs/windows.md) for what pado does on Windows (where it differs from the Mac).
  If code and docs disagree, the docs win or are updated in the same change. Don't implement
  **[decide]** items silently.
- **Stay a fork.** Everything outside `windows/`, `docs/windows.md`, `docs/images/icon/pado-*`,
  `README.md`, `CLAUDE.md` and `.github/workflows/{windows,sync-upstream}.yml` is bada's: change it
  upstream in team-mirae/bada-ime, not here, so merges stay clean. Upstream comes down through
  `.github/workflows/sync-upstream.yml` (docs/windows.md, "Staying a fork").
- `windows/host/` is team-mirae/bada-windows as a git subtree (`git subtree pull --prefix=windows/host
  … --squash`); its own `CLAUDE.md` applies inside it. Keep changes there generic (any engine) and
  send them upstream; pado-specific code goes in `windows/pado`, `windows/settings`, `windows/*.ps1`.
- The settings window (`windows/settings/page/ui.js`) ports `web/src/settings-ui.js`, which ports the
  Mac's SettingsWindow; it imports `web/src/settings.js` unchanged. A change to settings-ui.js needs
  the same change in ui.js.
- GUIDs in `windows/pado/ime.json` are permanent.
- License: MIT © Mirae Research; `windows/host` is CC0.
- Ask before registering the TIP or changing language settings on a real PC; test in Windows Sandbox
  (`windows/host/tools/sandbox/`) or a VM first.

## Building

- Windows: `windows/build.ps1` (engine as `pado_engine.dll` + bada-windows' TIP, x64 and x86, its core
  tests, `pado-settings.exe`, data), `windows/test.ps1` (pado's tests), `windows/install.ps1`
  (`-Update`, `-Uninstall`, `-Log`), `windows/sandbox.ps1` (install + `windows/type-test.ps1` in
  Windows Sandbox; `-Setup` with the installer), `windows/installer.ps1` (Inno Setup 6:
  `windows/installer/pado.iss` → `pado-ime-<version>-setup.exe`). PowerShell scripts are UTF-8 **with BOM**.
- Releases: push a tag `v<engine version>`; `.github/workflows/windows.yml` publishes the installer, with
  `windows/installer/notes/v<version>.md` as the notes.
- `windows/tools/make-icon.py`: `docs/images/icon/pado-icon.svg` → PNGs and `windows/pado/pado.ico`.
- Engine: `cargo test` in `engine/` (bada's tests). The typing robot: `cargo test --release --test robot`
  (docs/robot.md; CI: `.github/workflows/robot.yml`).
- Engine data and the analysis tools: as bada's (docs/layouts/13-bada.md §H9 and the build scripts in
  `engine/tools/`); regenerate them upstream.
- Web simulator (`web/`): bada's, `node web/build.mjs`. pado doesn't deploy it.

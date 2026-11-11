// User settings (docs/settings.md): HKCU\Software\bada-windows\<ime>, else the same key in HKLM
// (for apps that can't see the user's key), else the default.
#pragma once

#include <optional>
#include <string>

#include "host.h"

namespace bw::settings {

// Changes with every write through this file: text services re-read the settings only when it
// differs from the one they applied (jis-moonshot's #9: the registry isn't read at every focus).
unsigned long Version();

unsigned long ReadDword(const wchar_t* name, unsigned long fallback);
void WriteDword(const wchar_t* name, unsigned long value);
std::optional<std::wstring> ReadString(const wchar_t* name);
void WriteString(const wchar_t* name, const std::wstring& value);

// CapsLock, CapsRule, PassOn and KanaKey, with the manifest's keyboard as PassOn's default.
HostSettings ReadHost();
void WriteHost(const HostSettings& host);

// The engine's settings: the manifest's, with EngineSettings (a JSON object) over them.
std::string EngineSettings();

// The running program's file name, lowercase ("notepad.exe").
const std::wstring& ExeName();

// Whether the IME is off in `exe` (every key goes to the app, or types JIS there): for programs
// whose password boxes don't say they are one (jis-moonshot: certificate programs).
bool AppOff(const std::wstring& exe);
void SetAppOff(const std::wstring& exe, bool off);

}  // namespace bw::settings

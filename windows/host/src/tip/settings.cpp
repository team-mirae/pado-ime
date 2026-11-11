#include "settings.h"

#include <windows.h>
#include <aclapi.h>

#include <cwctype>
#include <mutex>
#include <vector>

#include "json.h"
#include "product.h"

namespace bw::settings {
namespace {

std::wstring Key() {
    const Product* p = GetProduct();
    return L"Software\\bada-windows\\" + (p ? p->key_name : std::wstring(L"unknown"));
}

std::wstring AppKey(const std::wstring& exe) { return Key() + L"\\Apps\\" + exe; }

// Programs the IME is off in by default: the local agents of Korean certificate programs, whose
// password boxes don't mark themselves (jis-moonshot windows/src/settings.cpp, kOffByDefault).
constexpr const wchar_t* kOffByDefault[] = {
    L"unicrslocalserver.exe", L"unicrsnoticenter.exe", L"ccdaemon.exe", L"magicline4nx.exe",
    L"skcertservice.exe",
};

// Sandboxed programs (AppContainer: the Start menu's search, Store apps) read only keys that grant
// ALL APPLICATION PACKAGES access; the key the first write creates doesn't (jis-moonshot measured
// it). Grants read, inherited, once per process; a sandboxed program can't, harmlessly.
void ShareWithApps() {
    static std::once_flag once;
    std::call_once(once, [] {
        BYTE sid[SECURITY_MAX_SID_SIZE];
        DWORD sid_size = sizeof(sid);
        if (!CreateWellKnownSid(WinBuiltinAnyPackageSid, nullptr, sid, &sid_size)) return;
        std::wstring path = L"CURRENT_USER\\" + Key();
        PACL old_acl = nullptr;
        PSECURITY_DESCRIPTOR descriptor = nullptr;
        if (GetNamedSecurityInfoW(path.data(), SE_REGISTRY_KEY, DACL_SECURITY_INFORMATION, nullptr, nullptr,
                                  &old_acl, nullptr, &descriptor) != ERROR_SUCCESS) {
            return;
        }
        EXPLICIT_ACCESSW access = {};
        access.grfAccessPermissions = KEY_READ;
        access.grfAccessMode = GRANT_ACCESS;
        access.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
        access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        access.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
        access.Trustee.ptstrName = reinterpret_cast<LPWSTR>(sid);
        PACL acl = nullptr;
        if (SetEntriesInAclW(1, &access, old_acl, &acl) == ERROR_SUCCESS) {
            SetNamedSecurityInfoW(path.data(), SE_REGISTRY_KEY, DACL_SECURITY_INFORMATION, nullptr, nullptr, acl,
                                  nullptr);
            LocalFree(acl);
        }
        LocalFree(descriptor);
    });
}

void Changed() {
    LARGE_INTEGER now = {};
    QueryPerformanceCounter(&now);
    DWORD version = static_cast<DWORD>(now.QuadPart ^ (now.QuadPart >> 32));
    if (version == Version()) ++version;
    RegSetKeyValueW(HKEY_CURRENT_USER, Key().c_str(), L"SettingsVersion", REG_DWORD, &version, sizeof(version));
    ShareWithApps();
}

std::wstring Choice(const wchar_t* name, const wchar_t* fallback) {
    auto value = ReadString(name);
    return value ? *value : fallback;
}

}  // namespace

unsigned long Version() {
    DWORD value = 0;
    DWORD size = sizeof(value);
    RegGetValueW(HKEY_CURRENT_USER, Key().c_str(), L"SettingsVersion", RRF_RT_REG_DWORD, nullptr, &value, &size);
    return value;
}

unsigned long ReadDword(const wchar_t* name, unsigned long fallback) {
    const std::wstring key = Key();
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD value = 0;
        DWORD size = sizeof(value);
        if (RegGetValueW(root, key.c_str(), name, RRF_RT_REG_DWORD, nullptr, &value, &size) == ERROR_SUCCESS) {
            return value;
        }
    }
    return fallback;
}

void WriteDword(const wchar_t* name, unsigned long value) {
    const DWORD v = value;
    RegSetKeyValueW(HKEY_CURRENT_USER, Key().c_str(), name, REG_DWORD, &v, sizeof(v));
    Changed();
}

std::optional<std::wstring> ReadString(const wchar_t* name) {
    const std::wstring key = Key();
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD size = 0;
        if (RegGetValueW(root, key.c_str(), name, RRF_RT_REG_SZ, nullptr, nullptr, &size) != ERROR_SUCCESS) continue;
        std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 1);
        size = static_cast<DWORD>(buffer.size() * sizeof(wchar_t));
        if (RegGetValueW(root, key.c_str(), name, RRF_RT_REG_SZ, nullptr, buffer.data(), &size) == ERROR_SUCCESS) {
            return std::wstring(buffer.data());
        }
    }
    return std::nullopt;
}

void WriteString(const wchar_t* name, const std::wstring& value) {
    RegSetKeyValueW(HKEY_CURRENT_USER, Key().c_str(), name, REG_SZ, value.c_str(),
                    static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    Changed();
}

HostSettings ReadHost() {
    HostSettings h;
    const std::wstring caps = Choice(L"CapsLock", L"respect");
    h.caps = caps == L"toggle" ? CapsSetting::kToggle : caps == L"transient" ? CapsSetting::kTransient
                                                                              : CapsSetting::kRespect;
    h.caps_rule = Choice(L"CapsRule", L"allcaps") == L"uppersnake" ? CapsRule::kUpperSnake : CapsRule::kAllCaps;
    const Product* p = GetProduct();
    const wchar_t* pass = p && p->manifest.keyboard == "us" ? L"windows" : L"jis";
    h.pass_on = Choice(L"PassOn", pass) == L"windows" ? PassOn::kWindows : PassOn::kJis;
    const std::wstring kana = Choice(L"KanaKey", Wide(kProfileMacKana).c_str());
    h.kana_profile = kana == L"none" ? std::string() : Utf8(kana);
    return h;
}

void WriteHost(const HostSettings& h) {
    const Product* p = GetProduct();
    if (!p) return;
    const std::wstring key = Key();
    auto set = [&](const wchar_t* name, const std::wstring& value) {
        RegSetKeyValueW(HKEY_CURRENT_USER, key.c_str(), name, REG_SZ, value.c_str(),
                        static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    };
    set(L"CapsLock", h.caps == CapsSetting::kToggle ? L"toggle" : h.caps == CapsSetting::kTransient ? L"transient"
                                                                                                  : L"respect");
    set(L"CapsRule", h.caps_rule == CapsRule::kUpperSnake ? L"uppersnake" : L"allcaps");
    set(L"PassOn", h.pass_on == PassOn::kWindows ? L"windows" : L"jis");
    set(L"KanaKey", h.kana_profile.empty() ? L"none" : Wide(h.kana_profile));
    Changed();
}

std::string EngineSettings() {
    const Product* p = GetProduct();
    const json::Value* base = p ? &p->manifest.settings : nullptr;
    std::optional<json::Value> user;
    if (auto text = ReadString(L"EngineSettings")) user = json::Parse(Utf8(*text));
    return json::Write(json::MergeTop(base, user ? &*user : nullptr));
}

const std::wstring& ExeName() {
    static const std::wstring name = [] {
        wchar_t path[MAX_PATH];
        const DWORD n = GetModuleFileNameW(nullptr, path, MAX_PATH);
        std::wstring s(path, n);
        const size_t slash = s.find_last_of(L'\\');
        if (slash != std::wstring::npos) s.erase(0, slash + 1);
        for (wchar_t& c : s) c = static_cast<wchar_t>(std::towlower(c));
        return s;
    }();
    return name;
}

bool AppOff(const std::wstring& exe) {
    const std::wstring key = AppKey(exe);
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD value = 0;
        DWORD size = sizeof(value);
        if (RegGetValueW(root, key.c_str(), L"Off", RRF_RT_REG_DWORD, nullptr, &value, &size) == ERROR_SUCCESS) {
            return value != 0;
        }
    }
    for (const wchar_t* name : kOffByDefault) {
        if (exe == name) return true;
    }
    return false;
}

void SetAppOff(const std::wstring& exe, bool off) {
    const DWORD v = off ? 1 : 0;
    RegSetKeyValueW(HKEY_CURRENT_USER, AppKey(exe).c_str(), L"Off", REG_DWORD, &v, sizeof(v));
    Changed();
}

}  // namespace bw::settings

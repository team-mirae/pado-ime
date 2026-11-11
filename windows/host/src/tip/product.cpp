#include "product.h"

#include <objbase.h>
#include <shellapi.h>
#include <shlobj.h>

#include <mutex>
#include <vector>

#include "dll.h"
#include "log.h"

namespace bw {
namespace {

std::once_flag g_product_once;
std::unique_ptr<Product> g_product;
std::wstring g_product_error;

// The module: a mutex rather than call_once, so it can be released and loaded again.
std::mutex g_module_mutex;
std::unique_ptr<Module> g_module;
bool g_module_tried = false;
// The dictionary file's time when it was last loaded; 0: none loaded (or none there).
ULONGLONG g_dictionary_time = 0;

ULONGLONG FileTime(const std::wstring& path) {
    WIN32_FILE_ATTRIBUTE_DATA data;
    if (!GetFileAttributesExW(path.c_str(), GetFileExInfoStandard, &data)) return 0;
    return (static_cast<ULONGLONG>(data.ftLastWriteTime.dwHighDateTime) << 32) | data.ftLastWriteTime.dwLowDateTime;
}

// With g_module_mutex held: (re)loads the dictionary if its file changed or went away.
void RefreshDictionaryLocked() {
    if (!g_module || !g_module->api().dictionary_load) return;
    const std::wstring path = DictionaryPath();
    if (path.empty()) return;
    const ULONGLONG time = FileTime(path);
    if (time == g_dictionary_time) return;
    std::string text;
    if (time && !ReadFile(std::filesystem::path(path), &text)) return;  // being written: next time
    g_dictionary_time = time;
    log::Write(L"dictionary: %s", Wide(g_module->LoadDictionary(text)).c_str());  // "": cleared
}

void LoadProduct() {
    const std::wstring dir = ModuleDirectory();
    for (const std::wstring& candidate : {dir + L"\\ime.json", dir + L"\\..\\ime.json"}) {
        if (GetFileAttributesW(candidate.c_str()) == INVALID_FILE_ATTRIBUTES) continue;
        std::string error;
        auto manifest = LoadManifest(std::filesystem::path(candidate), &error);
        if (!manifest) {
            g_product_error = candidate + L": " + Wide(error);
            return;
        }
        auto p = std::make_unique<Product>();
        p->manifest = std::move(*manifest);
        p->manifest_path = candidate;
        if (FAILED(CLSIDFromString(Wide(p->manifest.clsid).c_str(), &p->clsid)) ||
            FAILED(CLSIDFromString(Wide(p->manifest.profile).c_str(), &p->profile))) {
            g_product_error = candidate + L": invalid GUIDs";
            return;
        }
        p->display_name = Wide(p->manifest.display_name);
        p->key_name = Wide(p->manifest.name);
        g_product = std::move(p);
        return;
    }
    g_product_error = L"no ime.json next to " + dir;
}

void LoadModule() {
    const Product* product = GetProduct();
    if (!product) return;
    std::string error;
    const ULONGLONG start = GetTickCount64();
    auto module = Module::Load(product->manifest, &error);
    if (!module) {
        log::Write(L"engine not loaded: %s", Wide(error).c_str());
        return;
    }
    log::Write(L"engine loaded in %llu ms", GetTickCount64() - start);
    g_module = std::move(module);
    RefreshDictionaryLocked();
}

}  // namespace

std::wstring Wide(const std::string& s) {
    if (s.empty()) return {};
    const int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    std::wstring out(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), out.data(), n);
    return out;
}

std::string Utf8(const std::wstring& s) {
    if (s.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0, nullptr, nullptr);
    std::string out(static_cast<size_t>(n), '\0');
    WideCharToMultiByte(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), out.data(), n, nullptr, nullptr);
    return out;
}

std::wstring ModuleDirectory() {
    wchar_t path[MAX_PATH];
    const DWORD n = GetModuleFileNameW(DllInstance(), path, MAX_PATH);
    std::wstring s(path, n);
    const size_t slash = s.find_last_of(L'\\');
    return slash == std::wstring::npos ? s : s.substr(0, slash);
}

const Product* GetProduct() {
    std::call_once(g_product_once, LoadProduct);
    return g_product.get();
}

const std::wstring& ProductError() {
    GetProduct();
    return g_product_error;
}

void PreloadModule() {
    {
        std::lock_guard<std::mutex> lock(g_module_mutex);
        if (g_module_tried) return;
    }
    RunInBackground([] { SharedModule(); });
}

const Module* SharedModule() {
    std::lock_guard<std::mutex> lock(g_module_mutex);
    if (!g_module_tried) {
        g_module_tried = true;
        LoadModule();
    }
    return g_module.get();
}

void ReleaseSharedModule() {
    std::lock_guard<std::mutex> lock(g_module_mutex);
    g_module.reset();
    g_module_tried = false;
    g_dictionary_time = 0;
}

std::wstring DictionaryPath() {
    const Product* product = GetProduct();
    if (!product) return {};
    const std::wstring pattern = L"%APPDATA%\\bada-windows\\" + product->key_name + L"\\dictionary.json";
    wchar_t path[MAX_PATH];
    const DWORD n = ExpandEnvironmentStringsW(pattern.c_str(), path, MAX_PATH);
    return n && n <= MAX_PATH ? std::wstring(path) : std::wstring();
}

void RefreshDictionary() {
    // Never waits for a module still loading on the thread pool (it loads the dictionary itself).
    std::unique_lock<std::mutex> lock(g_module_mutex, std::try_to_lock);
    if (lock.owns_lock()) RefreshDictionaryLocked();
}

void OpenSettingsApp() {
    const Product* product = GetProduct();
    if (!product || product->manifest.settings_app.empty()) return;
    const std::filesystem::path app = product->manifest.root / PathFromUtf8(product->manifest.settings_app);
    const HINSTANCE result = ShellExecuteW(nullptr, L"open", app.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
    if (reinterpret_cast<INT_PTR>(result) <= 32) {
        log::Write(L"can't start %s (%d)", app.c_str(), static_cast<int>(reinterpret_cast<INT_PTR>(result)));
    }
}

void OpenDictionaryFolder() {
    const std::wstring path = DictionaryPath();
    const size_t slash = path.find_last_of(L'\\');
    if (slash == std::wstring::npos) return;
    const std::wstring folder = path.substr(0, slash);
    SHCreateDirectoryExW(nullptr, folder.c_str(), nullptr);
    ShellExecuteW(nullptr, L"open", folder.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
}

}  // namespace bw

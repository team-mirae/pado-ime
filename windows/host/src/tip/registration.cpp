// Adapted from team-mirae/jis-moonshot windows/src/registration.cpp.
#include "registration.h"

#include <msctf.h>
#include <olectl.h>
#include <wrl/client.h>

#include <string>

#include "product.h"

using Microsoft::WRL::ComPtr;

namespace bw {
namespace {

// As FKeyIME, Mozc and jis-moonshot register (FKeyIME docs/issues/settings-doesnt-list-fkeyime.md).
// Immersive support lets Store apps load it; UI-less, it runs in every kind of app.
const GUID* const kCategories[] = {
    &GUID_TFCAT_TIP_KEYBOARD,
    &GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    &GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    &GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    &GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    &GUID_TFCAT_TIPCAP_COMLESS,
    &GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
};

std::wstring ClsidKeyPath(const Product& p) {
    wchar_t clsid[39];
    StringFromGUID2(p.clsid, clsid, ARRAYSIZE(clsid));
    return std::wstring(L"Software\\Classes\\CLSID\\") + clsid;
}

}  // namespace

HRESULT RegisterServer(HINSTANCE module) {
    const Product* p = GetProduct();
    if (!p) return SELFREG_E_CLASS;
    wchar_t dll[MAX_PATH];
    const DWORD length = GetModuleFileNameW(module, dll, MAX_PATH);
    if (!length || length == MAX_PATH) return E_FAIL;
    HKEY clsid = nullptr;
    if (RegCreateKeyExW(HKEY_LOCAL_MACHINE, ClsidKeyPath(*p).c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, &clsid,
                        nullptr) != ERROR_SUCCESS) {
        return E_ACCESSDENIED;
    }
    RegSetValueExW(clsid, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE*>(p->display_name.c_str()),
                   static_cast<DWORD>((p->display_name.size() + 1) * sizeof(wchar_t)));
    HKEY server = nullptr;
    LSTATUS status = RegCreateKeyExW(clsid, L"InprocServer32", 0, nullptr, 0, KEY_WRITE, nullptr, &server, nullptr);
    if (status == ERROR_SUCCESS) {
        constexpr wchar_t kApartment[] = L"Apartment";
        status = RegSetValueExW(server, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE*>(dll),
                                (length + 1) * sizeof(wchar_t));
        if (status == ERROR_SUCCESS) {
            status = RegSetValueExW(server, L"ThreadingModel", 0, REG_SZ, reinterpret_cast<const BYTE*>(kApartment),
                                    sizeof(kApartment));
        }
        RegCloseKey(server);
    }
    RegCloseKey(clsid);
    return HRESULT_FROM_WIN32(status);
}

HRESULT UnregisterServer() {
    const Product* p = GetProduct();
    if (!p) return S_OK;
    const LSTATUS status = RegDeleteTreeW(HKEY_LOCAL_MACHINE, ClsidKeyPath(*p).c_str());
    return status == ERROR_FILE_NOT_FOUND ? S_OK : HRESULT_FROM_WIN32(status);
}

HRESULT RegisterProfiles(HINSTANCE module) {
    const Product* p = GetProduct();
    if (!p) return SELFREG_E_CLASS;
    ComPtr<ITfInputProcessorProfileMgr> manager;
    HRESULT hr = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(&manager));
    if (FAILED(hr)) return hr;
    ComPtr<ITfInputProcessorProfiles> profiles;
    hr = manager.As(&profiles);
    if (FAILED(hr)) return hr;
    // The TIP-level "Enable" value: Windows Settings lists a TIP's keyboards only with it.
    hr = profiles->Register(p->clsid);
    if (FAILED(hr)) return hr;
    // The icon: the manifest's .ico, else the DLL's own (index 0).
    std::wstring icon;
    if (!p->manifest.icon.empty()) {
        icon = (p->manifest.root / PathFromUtf8(p->manifest.icon)).wstring();
    } else {
        wchar_t dll[MAX_PATH];
        icon.assign(dll, GetModuleFileNameW(module, dll, MAX_PATH));
    }
    const LANGID language = static_cast<LANGID>(p->manifest.language);
    return manager->RegisterProfile(p->clsid, language, p->profile, p->display_name.c_str(),
                                    static_cast<ULONG>(p->display_name.size()), icon.c_str(),
                                    static_cast<ULONG>(icon.size()), 0, nullptr, 0, TRUE, 0);
}

HRESULT UnregisterProfiles() {
    const Product* p = GetProduct();
    if (!p) return S_OK;
    ComPtr<ITfInputProcessorProfileMgr> manager;
    HRESULT hr = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(&manager));
    if (FAILED(hr)) return hr;
    manager->UnregisterProfile(p->clsid, static_cast<LANGID>(p->manifest.language), p->profile, 0);
    ComPtr<ITfInputProcessorProfiles> profiles;
    if (SUCCEEDED(manager.As(&profiles))) profiles->Unregister(p->clsid);
    return S_OK;
}

HRESULT RegisterCategories() {
    const Product* p = GetProduct();
    if (!p) return SELFREG_E_CLASS;
    ComPtr<ITfCategoryMgr> categories;
    HRESULT hr = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&categories));
    if (FAILED(hr)) return hr;
    for (const GUID* category : kCategories) {
        hr = categories->RegisterCategory(p->clsid, *category, p->clsid);
        if (FAILED(hr)) return hr;
    }
    return S_OK;
}

HRESULT UnregisterCategories() {
    const Product* p = GetProduct();
    if (!p) return S_OK;
    ComPtr<ITfCategoryMgr> categories;
    HRESULT hr = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&categories));
    if (FAILED(hr)) return hr;
    for (const GUID* category : kCategories) categories->UnregisterCategory(p->clsid, *category, p->clsid);
    return S_OK;
}

}  // namespace bw

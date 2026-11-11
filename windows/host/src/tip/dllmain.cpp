// The TSF text input processor (an in-proc COM server): DLL exports and the class factory.
// Adapted from team-mirae/jis-moonshot windows/src/dllmain.cpp.
#include <windows.h>

#include <new>

#include "candidate_window.h"
#include "dll.h"
#include "log.h"
#include "product.h"
#include "registration.h"
#include "text_service.h"

namespace bw {
namespace {

HINSTANCE g_module = nullptr;
LONG g_refs = 0;

class ClassFactory final : public IClassFactory {
public:
    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        if (riid == IID_IUnknown || riid == IID_IClassFactory) {
            *object = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        *object = nullptr;
        return E_NOINTERFACE;
    }
    // Static object: its references are lock counts on the DLL.
    STDMETHODIMP_(ULONG) AddRef() override {
        DllAddRef();
        return 2;
    }
    STDMETHODIMP_(ULONG) Release() override {
        DllRelease();
        return 1;
    }
    STDMETHODIMP CreateInstance(IUnknown* outer, REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        *object = nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        log::Write(L"CreateInstance");
        auto* service = new (std::nothrow) TextService();
        if (!service) return E_OUTOFMEMORY;
        const HRESULT hr = service->QueryInterface(riid, object);
        service->Release();
        return hr;
    }
    STDMETHODIMP LockServer(BOOL lock) override {
        lock ? DllAddRef() : DllRelease();
        return S_OK;
    }
};

ClassFactory g_factory;

}  // namespace

HINSTANCE DllInstance() { return g_module; }
void DllAddRef() { InterlockedIncrement(&g_refs); }
void DllRelease() { InterlockedDecrement(&g_refs); }

bool RunInBackground(void (*job)()) {
    // A reference the thread pool drops only after the callback has returned
    // (FreeLibraryWhenCallbackReturns), so the DLL can't unload under it (jis-moonshot's notes).
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, reinterpret_cast<LPCWSTR>(&RunInBackground),
                            &module)) {
        return false;
    }
    struct Job {
        void (*run)();
        HMODULE module;
    };
    auto* context = new (std::nothrow) Job{job, module};
    DllAddRef();
    const BOOL queued = context && TrySubmitThreadpoolCallback(
                                       [](PTP_CALLBACK_INSTANCE instance, void* context) {
                                           auto* job = static_cast<Job*>(context);
                                           const HMODULE module = job->module;
                                           job->run();
                                           delete job;
                                           DllRelease();
                                           FreeLibraryWhenCallbackReturns(instance, module);
                                       },
                                       context, nullptr);
    if (!queued) {
        delete context;
        DllRelease();
        FreeLibrary(module);
    }
    return queued != FALSE;
}

}  // namespace bw

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    if (reason == DLL_PROCESS_ATTACH) {
        bw::g_module = instance;
        DisableThreadLibraryCalls(instance);
    } else if (reason == DLL_PROCESS_DETACH && !reserved) {
        bw::CandidateWindow::Unregister();
    }
    return TRUE;
}

// The engine module is freed here, outside the loader lock, once nothing references the DLL: the
// engine's own DLL must not be unloaded from DllMain.
STDAPI DllCanUnloadNow() {
    if (bw::g_refs) return S_FALSE;
    bw::ReleaseSharedModule();
    return S_OK;
}

STDAPI DllGetClassObject(REFCLSID clsid, REFIID riid, void** object) {
    if (!object) return E_INVALIDARG;
    *object = nullptr;
    const bw::Product* product = bw::GetProduct();
    if (!product) {
        bw::log::Write(L"no product: %s", bw::ProductError().c_str());
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    if (!IsEqualCLSID(clsid, product->clsid)) return CLASS_E_CLASSNOTAVAILABLE;
    bw::log::Write(L"DllGetClassObject");
    return bw::g_factory.QueryInterface(riid, object);
}

STDAPI DllUnregisterServer() {
    bw::UnregisterCategories();
    bw::UnregisterProfiles();
    return bw::UnregisterServer();
}

STDAPI DllRegisterServer() {
    HRESULT hr = bw::RegisterServer(bw::g_module);
    if (SUCCEEDED(hr)) hr = bw::RegisterProfiles(bw::g_module);
    if (SUCCEEDED(hr)) hr = bw::RegisterCategories();
    if (FAILED(hr)) DllUnregisterServer();
    return hr;
}

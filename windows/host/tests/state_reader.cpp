// bw_state: reads the public API (include/bada_windows_api.h) as another program would, through the
// IME's DLL found by its CLSID.
//
//   bw_state {CLSID}            the focused window's mode, staged text and state, once
//   bw_state {CLSID} --watch    again at every state message, until Ctrl+C
#include <windows.h>

#include <cstdio>
#include <string>

#include "bada_windows_api.h"

namespace {

using Version = int(WINAPI*)();
using Query = int(WINAPI*)(HWND);
using Get = int(WINAPI*)(HWND, wchar_t*, int);
using Message = UINT(WINAPI*)();

void Print(const wchar_t* what, int result, const wchar_t* text) {
    const wchar_t* error = result == BADA_WINDOWS_NOT_ACTIVE ? L"not active"
                           : result == BADA_WINDOWS_UNAVAILABLE ? L"unavailable"
                           : result == BADA_WINDOWS_E_SECURE ? L"SECURITY ERROR: a password field"
                           : result == BADA_WINDOWS_E_UNKNOWN ? L"not reported by the engine"
                           : result == BADA_WINDOWS_E_BUSY ? L"busy"
                           : result == BADA_WINDOWS_E_BUFFER ? L"buffer too small" : nullptr;
    wprintf(L"%s: %s\n", what, result >= 0 ? text : error ? error : L"?");
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        wprintf(L"usage: bw_state {CLSID} [--watch]\n");
        return 2;
    }
    wchar_t key[128], dll[MAX_PATH] = L"";
    DWORD size = sizeof(dll);
    swprintf_s(key, L"SOFTWARE\\Classes\\CLSID\\%s\\InprocServer32", argv[1]);
    if (RegGetValueW(HKEY_LOCAL_MACHINE, key, nullptr, RRF_RT_REG_SZ, nullptr, dll, &size) != ERROR_SUCCESS) {
        wprintf(L"not installed: no %s\n", key);
        return 1;
    }
    HMODULE api = LoadLibraryW(dll);
    auto version = api ? reinterpret_cast<Version>(GetProcAddress(api, "BadaWindowsApiVersion")) : nullptr;
    auto query = api ? reinterpret_cast<Query>(GetProcAddress(api, "BadaWindowsQueryWindow")) : nullptr;
    auto state = api ? reinterpret_cast<Get>(GetProcAddress(api, "BadaWindowsGetState")) : nullptr;
    auto staged = api ? reinterpret_cast<Get>(GetProcAddress(api, "BadaWindowsGetStaged")) : nullptr;
    auto message = api ? reinterpret_cast<Message>(GetProcAddress(api, "BadaWindowsStateMessage")) : nullptr;
    if (!version || !query || !state || !staged || !message) {
        wprintf(L"%s has no bada-windows API\n", dll);
        return 1;
    }
    wprintf(L"API version %d (%s)\n", version(), dll);
    auto show = [&] {
        const int mode = query(nullptr);
        wprintf(L"mode: %s\n", mode == 1 ? L"henkan" : mode == 0 ? L"muhenkan" : L"(not active)");
        static wchar_t buffer[16384];
        int n = staged(nullptr, buffer, 16384);
        Print(L"staged", n, buffer);
        n = state(nullptr, buffer, 16384);
        Print(L"state", n, buffer);
    };
    show();
    if (argc < 3 || std::wstring(argv[2]) != L"--watch") return 0;
    // A message-only window can't receive broadcasts: a hidden top-level one.
    WNDCLASSW wc = {};
    wc.lpfnWndProc = DefWindowProcW;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = L"BadaWindowsStateReader";
    RegisterClassW(&wc);
    HWND window = CreateWindowW(wc.lpszClassName, L"", WS_OVERLAPPED, 0, 0, 0, 0, nullptr, nullptr, wc.hInstance, nullptr);
    const UINT changed = message();
    MSG msg;
    while (window && GetMessageW(&msg, nullptr, 0, 0) > 0) {
        if (msg.message == changed) {
            wprintf(L"-- thread %llu, mode %lld, flags %#llx\n", static_cast<unsigned long long>(msg.wParam),
                    static_cast<long long>(msg.lParam & 0xFF), static_cast<unsigned long long>((msg.lParam >> 8) & 0xFF));
            show();
        }
        DispatchMessageW(&msg);
    }
    return 0;
}

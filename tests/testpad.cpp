// bw_testpad: a classic Edit control, a RichEdit through the IMM compatibility layer (its default)
// and a RichEdit using TSF directly (SES_USECTF), for typing through the installed TIP.
// Adapted from team-mirae/jis-moonshot windows/tests/testpad.cpp.
//
//   bw_testpad                                         a window to type in by hand
//   bw_testpad --clsid {…} --profile {…} --lang 0x0411
//              --target edit|rich|tsf|password|pwscope|nocontext|cancel|canceltsf|readonly
//              --keys "KeyH KeyI Space" --out result.txt [--peek 1] [--probe 1]
//
// Keys: KeyboardEvent.code names, sent by scan code (S- holds Shift, C- Ctrl, A- Alt; D- is the
// key-down alone, again for an auto-repeat, and R- its release), the JIS language keys NonConvert
// (無変換), Convert (変換), KanaMode (カタカナ/ひらがな), Lang1 / Lang2 (an Apple keyboard's かな / 英数),
// CapsLock; VK:XX sends a virtual key without a scan code, U+XXXX injects a character as Unicode;
// @off / @on / @focus=<target> / @wait=<ms> / @state=<file> (the API's state and staged text) are
// commands.
//
// With --keys it selects the IME's profile for itself, injects the keys with SendInput, writes the
// target's text as UTF-8 to --out and exits. It's the foreground window meanwhile: don't touch the
// keyboard.
#include <windows.h>
#include <imm.h>
#include <msctf.h>
#include <shellapi.h>
#include <InputScope.h>
#include <richedit.h>

#include <cstdio>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

namespace {

GUID g_clsid = {};
GUID g_profile_guid = {};
LANGID g_lang = 0x0411;
constexpr UINT kDone = WM_APP + 1;
constexpr UINT kCommand = WM_APP + 2;   // lParam: const std::wstring* (TypeKeys' @ commands)
constexpr UINT kComplete = WM_APP + 3;  // the cancel target ends the IME composition
constexpr UINT kTerminate = WM_APP + 4;  // the canceltsf target ends the TSF composition

HWND g_edit = nullptr;
HWND g_rich = nullptr;
HWND g_tsf = nullptr;
HWND g_password = nullptr;  // a classic password Edit (ES_PASSWORD)
HWND g_pwscope = nullptr;   // a TSF RichEdit with the password input scope, as browsers' inputs
HWND g_nocontext = nullptr;  // a plain window with no input context (ImmAssociateContext NULL), as games
HWND g_cancel = nullptr;     // an Edit that completes every IME composition at once (#6)
HWND g_cancel_tsf = nullptr; // a TSF RichEdit that ends the composition after every key (#6)
HWND g_readonly = nullptr;   // a read-only TSF RichEdit

std::wstring g_clsid_text, g_profile, g_lang_text = L"0x0411", g_target = L"edit", g_keys, g_out;
bool g_peek = false;   // --peek: the message loop peeks each key with PM_NOREMOVE first (#5)
bool g_probe = false;  // --probe: and asks TSF to test it (ITfKeystrokeMgr::TestKeyDown) (#5)
std::vector<unsigned> g_reached;  // virtual keys whose WM_KEYDOWN reached the target

HWND Target() {
    if (g_target == L"rich") return g_rich;
    if (g_target == L"tsf") return g_tsf;
    if (g_target == L"password") return g_password;
    if (g_target == L"pwscope") return g_pwscope;
    if (g_target == L"nocontext") return g_nocontext;
    if (g_target == L"cancel") return g_cancel;
    if (g_target == L"canceltsf") return g_cancel_tsf;
    if (g_target == L"readonly") return g_readonly;
    return g_edit;
}

struct Key {
    WORD scan;
    bool extended;
    WORD vk;  // for keys sent by virtual key (Hangul, NonConvert, Convert)
};

bool Lookup(const std::wstring& name, Key* key) {
    struct Named {
        const wchar_t* name;
        Key key;
    };
    static const Named kKeys[] = {
        {L"Digit1", {0x02}}, {L"Digit2", {0x03}}, {L"Digit3", {0x04}}, {L"Digit4", {0x05}},
        {L"Digit5", {0x06}}, {L"Digit6", {0x07}}, {L"Digit7", {0x08}}, {L"Digit8", {0x09}},
        {L"Digit9", {0x0A}}, {L"Digit0", {0x0B}}, {L"Minus", {0x0C}}, {L"Equal", {0x0D}},
        {L"KeyQ", {0x10}}, {L"KeyW", {0x11}}, {L"KeyE", {0x12}}, {L"KeyR", {0x13}},
        {L"KeyT", {0x14}}, {L"KeyY", {0x15}}, {L"KeyU", {0x16}}, {L"KeyI", {0x17}},
        {L"KeyO", {0x18}}, {L"KeyP", {0x19}}, {L"BracketLeft", {0x1A}}, {L"BracketRight", {0x1B}},
        {L"KeyA", {0x1E}}, {L"KeyS", {0x1F}}, {L"KeyD", {0x20}}, {L"KeyF", {0x21}},
        {L"KeyG", {0x22}}, {L"KeyH", {0x23}}, {L"KeyJ", {0x24}}, {L"KeyK", {0x25}},
        {L"KeyL", {0x26}}, {L"Semicolon", {0x27}}, {L"Quote", {0x28}}, {L"Backquote", {0x29}},
        {L"Backslash", {0x2B}}, {L"IntlBackslash", {0x56}}, {L"KeyZ", {0x2C}}, {L"KeyX", {0x2D}}, {L"KeyC", {0x2E}},
        {L"KeyV", {0x2F}}, {L"KeyB", {0x30}}, {L"KeyN", {0x31}}, {L"KeyM", {0x32}},
        {L"Comma", {0x33}}, {L"Period", {0x34}}, {L"Slash", {0x35}}, {L"Space", {0x39}},
        {L"Enter", {0x1C}}, {L"Backspace", {0x0E}}, {L"Escape", {0x01}}, {L"Tab", {0x0F}},
        {L"F6", {0x40}}, {L"F7", {0x41}}, {L"F8", {0x42}}, {L"F9", {0x43}}, {L"F10", {0x44}},
        {L"ArrowUp", {0x48, true}}, {L"ArrowDown", {0x50, true}},
        {L"ArrowLeft", {0x4B, true}}, {L"ArrowRight", {0x4D, true}},
        {L"IntlRo", {0x73}}, {L"IntlYen", {0x7D}},
        {L"Home", {0x47, true}}, {L"End", {0x4F, true}}, {L"Delete", {0x53, true}},
        // The JIS language keys, by scan code: Windows gives them the layout's virtual keys.
        {L"NonConvert", {0x7B}}, {L"Convert", {0x79}}, {L"KanaMode", {0x70}}, {L"CapsLock", {0x3A}},
        {L"Lang1", {0xF2}}, {L"Lang2", {0xF1}},
    };
    for (const Named& n : kKeys) {
        if (name == n.name) {
            *key = n.key;
            return true;
        }
    }
    return false;
}

INPUT Input(const Key& key, bool up) {
    INPUT in = {INPUT_KEYBOARD};
    if (key.vk) {
        in.ki.wVk = key.vk;
    } else {
        in.ki.wScan = key.scan;
        in.ki.dwFlags = KEYEVENTF_SCANCODE | (key.extended ? KEYEVENTF_EXTENDEDKEY : 0);
    }
    if (up) in.ki.dwFlags |= KEYEVENTF_KEYUP;
    return in;
}

struct Held {
    bool shift = false, ctrl = false, alt = false;
};

void Send(const Key& key, Held held) {
    const Key lshift = {0x2A}, lctrl = {0x1D}, lalt = {0x38};
    std::vector<INPUT> in;
    if (held.ctrl) in.push_back(Input(lctrl, false));
    if (held.alt) in.push_back(Input(lalt, false));
    if (held.shift) in.push_back(Input(lshift, false));
    in.push_back(Input(key, false));
    in.push_back(Input(key, true));
    if (held.shift) in.push_back(Input(lshift, true));
    if (held.alt) in.push_back(Input(lalt, true));
    if (held.ctrl) in.push_back(Input(lctrl, true));
    SendInput(static_cast<UINT>(in.size()), in.data(), sizeof(INPUT));
}

// A character injected as Unicode (KEYEVENTF_UNICODE): Windows delivers it as VK_PACKET.
void SendUnicode(wchar_t c) {
    INPUT in[2] = {{INPUT_KEYBOARD}, {INPUT_KEYBOARD}};
    in[0].ki.wScan = in[1].ki.wScan = c;
    in[0].ki.dwFlags = KEYEVENTF_UNICODE;
    in[1].ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
    SendInput(2, in, sizeof(INPUT));
}

void TypeKeys(HWND window) {
    Sleep(500);  // let the profile switch and focus settle
    std::wistringstream words(g_keys);
    std::wstring word;
    while (words >> word) {
        Held held;
        while (word.size() > 2 && word[1] == L'-' && (word[0] == L'S' || word[0] == L'C' || word[0] == L'A')) {
            if (word[0] == L'S') held.shift = true;
            if (word[0] == L'C') held.ctrl = true;
            if (word[0] == L'A') held.alt = true;
            word = word.substr(2);
        }
        Key key;
        if ((word.rfind(L"D-", 0) == 0 || word.rfind(L"R-", 0) == 0) && Lookup(word.substr(2), &key)) {
            // D-: the key-down alone (again: an auto-repeat), R-: its release.
            INPUT in = Input(key, word[0] == L'R');
            SendInput(1, &in, sizeof(INPUT));
        } else if (word.rfind(L"VK:", 0) == 0) {
            // A virtual key with no scan code, as some injectors send (#9).
            Key vk_only = {0, false, static_cast<WORD>(wcstoul(word.c_str() + 3, nullptr, 16))};
            Send(vk_only, held);
        } else if (word.rfind(L"U+", 0) == 0) {
            SendUnicode(static_cast<wchar_t>(wcstoul(word.c_str() + 2, nullptr, 16)));
        } else if (word[0] == L'@') {
            // A command, run on the window's thread: @off / @on (switch to the Microsoft IME and
            // back), @focus=<target>, @wait=<ms>.
            if (word.rfind(L"@wait=", 0) == 0) {
                Sleep(wcstoul(word.c_str() + 6, nullptr, 10));
            } else {
                static std::wstring command;
                command = word.substr(1);
                SendMessageW(window, kCommand, 0, reinterpret_cast<LPARAM>(&command));
            }
        } else if (Lookup(word, &key)) {
            Send(key, held);
        } else {
            std::fwprintf(stderr, L"unknown key %ls\n", word.c_str());
        }
        if (g_target == L"canceltsf") {
            Sleep(15);
            PostMessageW(window, kTerminate, 0, 0);
        }
        Sleep(40);
    }
    Sleep(400);
    PostMessageW(window, kDone, 0, 0);
}

// Starts in the manifest's mode: the session's mode compartment (text_service.h) cleared.
void ResetMode() {
    ITfThreadMgr* threads = nullptr;
    if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&threads)))) return;
    TfClientId id = TF_CLIENTID_NULL;
    ITfCompartmentMgr* global = nullptr;
    ITfCompartment* mode = nullptr;
    if (SUCCEEDED(threads->Activate(&id)) && SUCCEEDED(threads->GetGlobalCompartment(&global)) &&
        SUCCEEDED(global->GetCompartment(g_clsid, &mode))) {
        VARIANT v;
        v.vt = VT_I4;
        v.lVal = 0;
        mode->SetValue(id, &v);
    }
    if (mode) mode->Release();
    if (global) global->Release();
    threads->Release();  // stays activated for the process: the edit controls use it
}

void ActivateProfile(REFCLSID clsid, REFGUID profile) {
    ITfInputProcessorProfileMgr* profiles = nullptr;
    if (FAILED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                IID_PPV_ARGS(&profiles)))) {
        return;
    }
    const HRESULT hr = profiles->ActivateProfile(
        TF_PROFILETYPE_INPUTPROCESSOR, g_lang, clsid, profile,
        nullptr, TF_IPPMF_FORPROCESS | TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE);
    if (FAILED(hr)) std::fwprintf(stderr, L"ActivateProfile failed %#lx\n", hr);
    profiles->Release();
}

void SelectProfile() {
    if (g_clsid_text.empty()) return;
    ResetMode();
    ActivateProfile(g_clsid, g_profile_guid);
}

// The Microsoft Japanese IME, to switch the IME off mid-composition (@off).
void SelectMicrosoftIme() {
    constexpr GUID kMsClsid = {0x03b5835f, 0xf03c, 0x411b, {0x9c, 0xe2, 0xaa, 0x23, 0xe1, 0x17, 0x1e, 0x36}};
    constexpr GUID kMsProfile = {0xa76c93d9, 0x5523, 0x4e90, {0xaa, 0xfa, 0x4d, 0xb1, 0x12, 0xf9, 0xac, 0x76}};
    const LANGID saved = g_lang;
    g_lang = 0x0411;
    ActivateProfile(kMsClsid, kMsProfile);
    g_lang = saved;
}

// What an app does to end the IME's composition itself (the canceltsf target).
void TerminateTsfComposition() {
    ITfThreadMgr* threads = nullptr;
    ITfDocumentMgr* document = nullptr;
    ITfContext* context = nullptr;
    ITfContextOwnerCompositionServices* services = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&threads))) &&
        SUCCEEDED(threads->GetFocus(&document)) && document && SUCCEEDED(document->GetTop(&context)) && context &&
        SUCCEEDED(context->QueryInterface(IID_PPV_ARGS(&services)))) {
        services->TerminateComposition(nullptr);
    }
    if (services) services->Release();
    if (context) context->Release();
    if (document) document->Release();
    if (threads) threads->Release();
}

// The public API's answer for the target, as the IME's own DLL gives it (found by its CLSID), to
// `file`: "<GetStaged's result> <staged>\n<GetState's result> <state>".
void WriteState(const std::wstring& file) {
    wchar_t key[128], dll[MAX_PATH] = L"";
    DWORD size = sizeof(dll);
    swprintf_s(key, L"SOFTWARE\\Classes\\CLSID\\%s\\InprocServer32", g_clsid_text.c_str());
    RegGetValueW(HKEY_LOCAL_MACHINE, key, nullptr, RRF_RT_REG_SZ, nullptr, dll, &size);
    HMODULE api = LoadLibraryW(dll);
    using Get = int(WINAPI*)(HWND, wchar_t*, int);
    std::wstring line;
    if (api) {
        for (const char* name : {"BadaWindowsGetStaged", "BadaWindowsGetState"}) {
            auto get = reinterpret_cast<Get>(GetProcAddress(api, name));
            wchar_t buffer[8192] = L"";
            const int n = get ? get(Target(), buffer, 8192) : -100;
            line += std::to_wstring(n) + L" " + (n >= 0 ? buffer : L"") + L"\n";
        }
        FreeLibrary(api);
    }
    const int bytes = WideCharToMultiByte(CP_UTF8, 0, line.data(), static_cast<int>(line.size()), nullptr, 0, nullptr, nullptr);
    std::string utf8(static_cast<size_t>(bytes), '\0');
    WideCharToMultiByte(CP_UTF8, 0, line.data(), static_cast<int>(line.size()), utf8.data(), bytes, nullptr, nullptr);
    FILE* f = nullptr;
    if (_wfopen_s(&f, file.c_str(), L"wb") == 0 && f) {
        std::fwrite(utf8.data(), 1, utf8.size(), f);
        std::fclose(f);
    }
}

void RunCommand(const std::wstring& command) {
    if (command == L"off") {
        SelectMicrosoftIme();
    } else if (command == L"on") {
        ActivateProfile(g_clsid, g_profile_guid);
    } else if (command.rfind(L"state=", 0) == 0) {
        WriteState(command.substr(6));
    } else if (command.rfind(L"focus=", 0) == 0) {
        g_target = command.substr(6);
        SetFocus(Target());
    } else {
        std::fwprintf(stderr, L"unknown command %ls\n", command.c_str());
    }
}

void WriteResult() {
    HWND target = Target();
    const int n = GetWindowTextLengthW(target);
    std::wstring text(static_cast<size_t>(n) + 1, L'\0');
    GetWindowTextW(target, text.data(), n + 1);
    text.resize(static_cast<size_t>(n));
    const int bytes = WideCharToMultiByte(CP_UTF8, 0, text.data(), n, nullptr, 0, nullptr, nullptr);
    std::string utf8(static_cast<size_t>(bytes), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text.data(), n, utf8.data(), bytes, nullptr, nullptr);
    FILE* f = nullptr;
    if (_wfopen_s(&f, g_out.c_str(), L"wb") == 0 && f) {
        std::fwrite(utf8.data(), 1, utf8.size(), f);
        std::fclose(f);
    }
    // The keys the app got (the TIP didn't eat), as hex virtual keys: <out>.keys.
    if (_wfopen_s(&f, (g_out + L".keys").c_str(), L"wb") == 0 && f) {
        for (unsigned vk : g_reached) std::fprintf(f, "%02X ", vk);
        std::fclose(f);
    }
}

LRESULT CALLBACK ControlProc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    if ((message == WM_KEYDOWN || message == WM_SYSKEYDOWN) && window == Target()) {
        g_reached.push_back(static_cast<unsigned>(wparam));
    }
    // The cancel target ends each composition as soon as it shows, as some apps do (#6).
    if (window == g_cancel && message == WM_IME_COMPOSITION && (lparam & GCS_COMPSTR)) {
        PostMessageW(GetParent(window), kComplete, 0, 0);
    }
    auto original = reinterpret_cast<WNDPROC>(GetPropW(window, L"bw.proc"));
    return CallWindowProcW(original, window, message, wparam, lparam);
}

void Watch(HWND control) {
    SetPropW(control, L"bw.proc", reinterpret_cast<HANDLE>(GetWindowLongPtrW(control, GWLP_WNDPROC)));
    SetWindowLongPtrW(control, GWLP_WNDPROC, reinterpret_cast<LONG_PTR>(ControlProc));
}

LRESULT CALLBACK Proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    switch (message) {
        case WM_SIZE: {
            const int w = LOWORD(lparam), h = HIWORD(lparam);
            const int row = (h - 80) / 3;
            MoveWindow(g_edit, 8, 8, w - 16, row - 12, TRUE);
            MoveWindow(g_rich, 8, row + 4, w - 16, row - 12, TRUE);
            MoveWindow(g_tsf, 8, 2 * row + 4, w - 16, row - 12, TRUE);
            MoveWindow(g_password, 8, 3 * row + 8, w / 2 - 12, 32, TRUE);
            MoveWindow(g_pwscope, w / 2 + 4, 3 * row + 8, w / 2 - 12, 32, TRUE);
            MoveWindow(g_nocontext, 8, 3 * row + 44, w / 2 - 12, 32, TRUE);
            MoveWindow(g_cancel, w / 2 + 4, 3 * row + 44, w / 2 - 12, 32, TRUE);
            MoveWindow(g_cancel_tsf, 8, 3 * row + 80, w / 2 - 12, 32, TRUE);
            MoveWindow(g_readonly, w / 2 + 4, 3 * row + 80, w / 2 - 12, 32, TRUE);
            return 0;
        }
        case kCommand:
            RunCommand(*reinterpret_cast<const std::wstring*>(lparam));
            return 0;
        case kTerminate:
            TerminateTsfComposition();
            return 0;
        case kComplete:
            if (HIMC context = ImmGetContext(g_cancel)) {
                ImmNotifyIME(context, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
                ImmReleaseContext(g_cancel, context);
            }
            return 0;
        case kDone:
            if (!g_out.empty()) WriteResult();
            DestroyWindow(window);
            return 0;
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
    }
    return DefWindowProcW(window, message, wparam, lparam);
}

}  // namespace

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int show) {
    int argc = 0;
    wchar_t** argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    for (int i = 1; i + 1 < argc; i += 2) {
        const std::wstring flag = argv[i];
        if (flag == L"--clsid") g_clsid_text = argv[i + 1];
        if (flag == L"--profile") g_profile = argv[i + 1];
        if (flag == L"--lang") g_lang_text = argv[i + 1];
        if (flag == L"--target") g_target = argv[i + 1];
        if (flag == L"--keys") g_keys = argv[i + 1];
        if (flag == L"--out") g_out = argv[i + 1];
        if (flag == L"--peek") g_peek = std::wstring(argv[i + 1]) != L"0";
        if (flag == L"--probe") g_probe = std::wstring(argv[i + 1]) != L"0";
    }
    if (!g_clsid_text.empty() &&
        (FAILED(CLSIDFromString(g_clsid_text.c_str(), &g_clsid)) || FAILED(CLSIDFromString(g_profile.c_str(), &g_profile_guid)))) {
        std::fwprintf(stderr, L"--clsid and --profile are GUIDs\n");
        return 2;
    }
    g_lang = static_cast<LANGID>(wcstoul(g_lang_text.c_str(), nullptr, 0));
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    LoadLibraryW(L"Msftedit.dll");

    WNDCLASSEXW wc = {sizeof(wc)};
    wc.lpfnWndProc = Proc;
    wc.hInstance = instance;
    wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_BTNFACE + 1);
    wc.lpszClassName = L"BadaWindowsTestpad";
    RegisterClassExW(&wc);
    HWND window = CreateWindowExW(0, wc.lpszClassName, L"bada-windows testpad", WS_OVERLAPPEDWINDOW,
                                  CW_USEDEFAULT, CW_USEDEFAULT, 640, 560, nullptr, nullptr,
                                  instance, nullptr);
    g_edit = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", L"",
                             WS_CHILD | WS_VISIBLE | ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL,
                             0, 0, 0, 0, window, nullptr, instance, nullptr);
    g_rich = CreateWindowExW(WS_EX_CLIENTEDGE, MSFTEDIT_CLASS, L"",
                             WS_CHILD | WS_VISIBLE | ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL,
                             0, 0, 0, 0, window, nullptr, instance, nullptr);
    g_tsf = CreateWindowExW(WS_EX_CLIENTEDGE, MSFTEDIT_CLASS, L"",
                            WS_CHILD | WS_VISIBLE | ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL,
                            0, 0, 0, 0, window, nullptr, instance, nullptr);
    SendMessageW(g_tsf, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
    g_password = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", L"", WS_CHILD | WS_VISIBLE | ES_PASSWORD,
                                 0, 0, 0, 0, window, nullptr, instance, nullptr);
    g_pwscope = CreateWindowExW(WS_EX_CLIENTEDGE, MSFTEDIT_CLASS, L"", WS_CHILD | WS_VISIBLE,
                                0, 0, 0, 0, window, nullptr, instance, nullptr);
    SendMessageW(g_pwscope, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
    // SetInputScope is in msctf.dll, which has no import library.
    using SetInputScopeFn = HRESULT(WINAPI*)(HWND, InputScope);
    if (auto set = reinterpret_cast<SetInputScopeFn>(
            GetProcAddress(LoadLibraryW(L"msctf.dll"), "SetInputScope"))) {
        set(g_pwscope, IS_PASSWORD);
    }
    WNDCLASSEXW plain = {sizeof(plain)};
    plain.lpfnWndProc = DefWindowProcW;
    plain.hInstance = instance;
    plain.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
    plain.lpszClassName = L"BadaWindowsNoContext";
    RegisterClassExW(&plain);
    g_nocontext = CreateWindowExW(WS_EX_CLIENTEDGE, plain.lpszClassName, L"", WS_CHILD | WS_VISIBLE,
                                  0, 0, 0, 0, window, nullptr, instance, nullptr);
    ImmAssociateContext(g_nocontext, nullptr);
    g_cancel = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", L"", WS_CHILD | WS_VISIBLE,
                               0, 0, 0, 0, window, nullptr, instance, nullptr);
    g_cancel_tsf = CreateWindowExW(WS_EX_CLIENTEDGE, MSFTEDIT_CLASS, L"", WS_CHILD | WS_VISIBLE,
                                   0, 0, 0, 0, window, nullptr, instance, nullptr);
    SendMessageW(g_cancel_tsf, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
    g_readonly = CreateWindowExW(WS_EX_CLIENTEDGE, MSFTEDIT_CLASS, L"", WS_CHILD | WS_VISIBLE | ES_READONLY,
                                 0, 0, 0, 0, window, nullptr, instance, nullptr);
    SendMessageW(g_readonly, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
    for (HWND control : {g_edit, g_rich, g_tsf, g_password, g_pwscope, g_nocontext, g_cancel, g_cancel_tsf, g_readonly}) {
        Watch(control);
    }
    HFONT font = CreateFontW(-20, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, 0, 0, 0, 0,
                             L"Yu Gothic UI");
    SendMessageW(g_edit, WM_SETFONT, reinterpret_cast<WPARAM>(font), TRUE);
    SendMessageW(g_rich, WM_SETFONT, reinterpret_cast<WPARAM>(font), TRUE);
    SendMessageW(g_tsf, WM_SETFONT, reinterpret_cast<WPARAM>(font), TRUE);
    ShowWindow(window, show);
    SetForegroundWindow(window);
    SetFocus(Target());
    SelectProfile();

    std::thread typist;
    if (!g_keys.empty()) typist = std::thread(TypeKeys, window);
    ITfKeystrokeMgr* keystrokes = nullptr;
    if (g_probe) {
        ITfThreadMgr* threads = nullptr;
        if (SUCCEEDED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&threads)))) {
            threads->QueryInterface(IID_PPV_ARGS(&keystrokes));
            threads->Release();
        }
    }
    MSG msg;
    while (true) {
        if (g_peek) {
            // Look at the next key before taking it, as some message loops do: TSF may test it
            // more than once (#5).
            MSG peeked;
            while (!PeekMessageW(&peeked, nullptr, 0, 0, PM_NOREMOVE)) WaitMessage();
            if (keystrokes && peeked.message == WM_KEYDOWN && peeked.wParam != VK_PROCESSKEY) {
                BOOL eaten = FALSE;
                keystrokes->TestKeyDown(peeked.wParam, peeked.lParam, &eaten);
                keystrokes->TestKeyDown(peeked.wParam, peeked.lParam, &eaten);
            }
        }
        if (GetMessageW(&msg, nullptr, 0, 0) <= 0) break;
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    if (typist.joinable()) typist.join();
    if (keystrokes) keystrokes->Release();
    CoUninitialize();
    return 0;
}

// Adapted from team-mirae/jis-moonshot windows/src/text_service.*.
#include "text_service.h"

#include <InputScope.h>
#include <richedit.h>

#include <cwchar>
#include <iterator>
#include <new>

#include "display_attribute.h"
#include "dll.h"
#include "edit_session.h"
#include "guids.h"
#include "log.h"
#include "product.h"
#include "settings.h"
#include "state_shared.h"

using Microsoft::WRL::ComPtr;

namespace bw {
namespace {

// In a host that refuses a composition, or ends it right after each update, the unconfirmed text
// stays on screen as plain text and is replaced at every key (jis-moonshot #6, option A).
constexpr bool kKeepTextWithoutComposition = true;
// An app that ends the composition this soon after our update did it by itself (measured: 15-31 ms).
constexpr ULONGLONG kOwnTermination = 100;
// Our injected Caps Lock press comes back through the key sink within this long.
constexpr ULONGLONG kCapsEcho = 500;

constexpr const char* kProfiles[] = {kProfileHenkan, kProfileKatakana, kProfileMacKana};

enum MenuId : UINT {
    kMenuHenkan = 1,
    kMenuMuhenkan,
    kMenuProfileFirst = 10,  // + index in kProfiles
    kMenuCapsFirst = 20,     // + CapsSetting
    kMenuRuleFirst = 30,     // + CapsRule
    kMenuPassJis = 40,
    kMenuPassWindows,
    kMenuKanaMacKana = 50,
    kMenuKanaNone,
    kMenuAppOff = 60,
    kMenuPublish,
    kMenuLog,
    kMenuAbout,
};

bool Down(int vk) { return (GetKeyState(vk) & 0x8000) != 0; }

// A context the IMM compatibility layer made for a classic control.
bool Transitory(ITfContext* context) {
    TF_STATUS status = {};
    return context && SUCCEEDED(context->GetStatus(&status)) && (status.dwStaticFlags & TF_SS_TRANSITORY) != 0;
}

Mods Modifiers() {
    Mods m;
    m.shift = Down(VK_SHIFT);
    m.ctrl = Down(VK_CONTROL);
    m.alt = Down(VK_MENU);
    m.meta = Down(VK_LWIN) || Down(VK_RWIN);
    return m;
}

bool SameObject(IUnknown* a, IUnknown* b) {
    ComPtr<IUnknown> x, y;
    if (!a || !b || FAILED(a->QueryInterface(IID_PPV_ARGS(&x))) || FAILED(b->QueryInterface(IID_PPV_ARGS(&y)))) {
        return false;
    }
    return x == y;
}

HRESULT GetCompartment(IUnknown* owner, REFGUID guid, ComPtr<ITfCompartment>* out) {
    ComPtr<ITfCompartmentMgr> manager;
    HRESULT hr = owner->QueryInterface(IID_PPV_ARGS(&manager));
    if (FAILED(hr)) return hr;
    return manager->GetCompartment(guid, &*out);
}

LONG ReadCompartment(ITfCompartment* compartment) {
    VARIANT v;
    VariantInit(&v);
    if (!compartment || FAILED(compartment->GetValue(&v))) return 0;
    const LONG value = v.vt == VT_I4 ? v.lVal : 0;
    VariantClear(&v);  // another TIP or app may have stored a string or an object there
    return value;
}

void WriteCompartment(ITfCompartment* compartment, TfClientId client_id, LONG value) {
    if (!compartment) return;
    VARIANT v;
    v.vt = VT_I4;
    v.lVal = value;
    compartment->SetValue(client_id, &v);
}

}  // namespace

// The document, as the host sees it (src/core/host.h): every change is an edit session.
class TextService::TsfField final : public Field {
public:
    TsfField(TextService* service, ITfContext* context) : service_(service), context_(context) {}
    void Apply(const Output& out) override { service_->ApplyOutput(context_, out); }
    std::string Selection() override { return service_->ReadSelection(context_); }
    void Type(const std::string& text) override {
        service_->UpdateText(context_, Wide(text), {}, false, false);
    }

private:
    TextService* service_;
    ITfContext* context_;
};

TextService::TextService() { DllAddRef(); }

TextService::~TextService() {
    host_.reset();
    engine_.reset();
    DllRelease();
}

STDMETHODIMP TextService::QueryInterface(REFIID riid, void** object) {
    if (!object) return E_INVALIDARG;
    if (riid == IID_IUnknown || riid == IID_ITfTextInputProcessor || riid == IID_ITfTextInputProcessorEx) {
        *object = static_cast<ITfTextInputProcessorEx*>(this);
    } else if (riid == IID_ITfKeyEventSink) {
        *object = static_cast<ITfKeyEventSink*>(this);
    } else if (riid == IID_ITfThreadMgrEventSink) {
        *object = static_cast<ITfThreadMgrEventSink*>(this);
    } else if (riid == IID_ITfActiveLanguageProfileNotifySink) {
        *object = static_cast<ITfActiveLanguageProfileNotifySink*>(this);
    } else if (riid == IID_ITfCompositionSink) {
        *object = static_cast<ITfCompositionSink*>(this);
    } else if (riid == IID_ITfCompartmentEventSink) {
        *object = static_cast<ITfCompartmentEventSink*>(this);
    } else if (riid == IID_ITfDisplayAttributeProvider) {
        *object = static_cast<ITfDisplayAttributeProvider*>(this);
    } else {
        *object = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) TextService::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) TextService::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (!refs) delete this;
    return refs;
}

// --- activation

STDMETHODIMP TextService::Activate(ITfThreadMgr* thread_mgr, TfClientId client_id) {
    return ActivateEx(thread_mgr, client_id, 0);
}

STDMETHODIMP TextService::ActivateEx(ITfThreadMgr* thread_mgr, TfClientId client_id, DWORD) {
    const Product* product = GetProduct();
    if (!product) return E_FAIL;
    thread_mgr_ = thread_mgr;
    client_id_ = client_id;
    active_ = true;
    log::Write(L"ActivateEx tid=%lu", GetCurrentThreadId());

    ComPtr<ITfCategoryMgr> categories;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&categories)))) {
        categories->RegisterGUID(kDisplayAttributeInput, &input_attribute_);
        categories->RegisterGUID(kDisplayAttributeConverted, &converted_attribute_);
    }
    ComPtr<ITfKeystrokeMgr> keystrokes;
    if (SUCCEEDED(thread_mgr_.As(&keystrokes))) {
        const HRESULT hr = keystrokes->AdviseKeyEventSink(client_id_, this, TRUE);
        if (FAILED(hr)) log::Write(L"AdviseKeyEventSink failed %#lx", hr);
    }
    ComPtr<ITfSource> source;
    if (SUCCEEDED(thread_mgr_.As(&source))) {
        source->AdviseSink(IID_ITfThreadMgrEventSink, static_cast<ITfThreadMgrEventSink*>(this), &thread_sink_cookie_);
        source->AdviseSink(IID_ITfActiveLanguageProfileNotifySink,
                           static_cast<ITfActiveLanguageProfileNotifySink*>(this), &profile_sink_cookie_);
    }
    lang_bar_ = new (std::nothrow) LangBarButton(
        [this](ITfMenu* menu) { InitMenu(menu); }, [this](UINT id) { OnMenu(id); },
        [this] {
            // A click switches mode, as Caps Lock's toggle or ⌃⌥⇧変換.
            if (!EnsureEngine()) return;
            TsfField field(this, Target(nullptr));
            host_->SwitchMode(host_->mode() == Mode::kHenkan ? Mode::kMuhenkan : Mode::kHenkan, field);
            AfterHost();
        });
    ComPtr<ITfLangBarItemMgr> items;
    if (lang_bar_ && SUCCEEDED(thread_mgr_.As(&items))) items->AddItem(lang_bar_);

    // The session's mode (every app shares it), else the manifest's.
    mode_ = product->manifest.mode == "muhenkan" ? Mode::kMuhenkan : Mode::kHenkan;
    ReadSession(&mode_, &profile_);
    AdviseCompartments(true);
    PreloadModule();
    ApplySettings(true);
    SetOpen(mode_ == Mode::kHenkan);
    UpdateIndicator();
    Publish();
    return S_OK;
}

STDMETHODIMP TextService::Deactivate() {
    log::Write(L"Deactivate tid=%lu", GetCurrentThreadId());
    if (ITfContext* pending = Target(nullptr)) Flush(pending);
    active_ = false;
    refused_ = false;
    orphan_.Reset();
    orphan_context_.Reset();
    list_.Hide();
    ReleaseSlot();
    AdviseCompartments(false);
    if (thread_mgr_) {
        ComPtr<ITfLangBarItemMgr> items;
        if (lang_bar_ && SUCCEEDED(thread_mgr_.As(&items))) items->RemoveItem(lang_bar_);
        ComPtr<ITfKeystrokeMgr> keystrokes;
        if (SUCCEEDED(thread_mgr_.As(&keystrokes))) keystrokes->UnadviseKeyEventSink(client_id_);
        ComPtr<ITfSource> source;
        if (SUCCEEDED(thread_mgr_.As(&source))) {
            if (thread_sink_cookie_ != TF_INVALID_COOKIE) source->UnadviseSink(thread_sink_cookie_);
            if (profile_sink_cookie_ != TF_INVALID_COOKIE) source->UnadviseSink(profile_sink_cookie_);
        }
    }
    if (lang_bar_) {
        lang_bar_->Detach();
        lang_bar_->Release();
        lang_bar_ = nullptr;
    }
    thread_sink_cookie_ = profile_sink_cookie_ = TF_INVALID_COOKIE;
    composition_.Reset();
    composition_context_.Reset();
    host_.reset();
    engine_.reset();
    thread_mgr_.Reset();
    client_id_ = TF_CLIENTID_NULL;
    return S_OK;
}

bool TextService::EnsureEngine() {
    if (host_) return true;
    const Module* module = SharedModule();  // waits for the preload
    if (!module) return false;
    auto engine = std::make_unique<Engine>(*module);
    if (!engine->ok()) {
        static bool logged = false;
        if (!logged) log::Write(L"P_new failed");
        logged = true;
        return false;
    }
    engine_ = std::move(engine);
    engine_->SetSettings(settings::EngineSettings());
    host_ = std::make_unique<Host>(*engine_, settings::ReadHost(), mode_);
    // The engine's first P_set_mode (and profile), with nothing to show yet.
    TsfField none(this, nullptr);
    host_->Start(mode_, profile_, none);
    return true;
}

void TextService::ApplySettings(bool force) {
    const unsigned long version = settings::Version();
    if (!force && applied_version_ == version) return;
    applied_version_ = version;
    app_off_ = settings::AppOff(settings::ExeName());
    publish_ = settings::ReadDword(L"PublishState", 1) != 0;
    if (host_) {
        host_->set_settings(settings::ReadHost());
        engine_->SetSettings(settings::EngineSettings());
    }
    if (!publish_) ReleaseSlot();
}

// --- keys

bool TextService::Composing() const { return host_ && host_->composing(); }

ITfContext* TextService::Target(ITfContext* context) const {
    if (context) return context;
    return composition_context_ ? composition_context_.Get() : orphan_context_.Get();
}

HRESULT TextService::NoContext(unsigned vk) {
    // No document to type into (a game window without an input context): the key is the app's, and
    // nothing stays in the engine where it can't be seen (jis-moonshot #3).
    static bool logged = false;
    if (!logged) log::Write(L"a key without an input context: passed on");
    logged = true;
    if (Composing()) DropInvisible();
    if (vk < eaten_down_.size()) eaten_down_[vk] = false;
    return S_OK;
}

void TextService::DropInvisible() {
    orphan_.Reset();
    orphan_context_.Reset();
    if (engine_) engine_->Flush();  // its commit has nowhere to go
    if (host_) host_->Forget();
    menu_ = {};
    list_.Hide();
}

void TextService::CheckOrphan() {
    if (!orphan_) return;
    bool inside = true, gone = false;
    ComPtr<ITfRange> orphan = orphan_;
    ComPtr<ITfContext> context = orphan_context_;
    RunReadSessionNow(context.Get(), client_id_, [&](TfEditCookie ec) -> HRESULT {
        BOOL empty = FALSE;
        gone = SUCCEEDED(orphan->IsEmpty(ec, &empty)) && empty;
        inside = CaretInside(ec, context.Get(), orphan.Get());
        return S_OK;
    });
    if (gone || !inside) DropInvisible();
}

void TextService::Settle() {
    if (!refused_) return;
    refused_ = false;
    // The preedit went in as plain text: the engine lets go of it (its commit is already there).
    if (engine_) engine_->Flush();
    if (host_) host_->Forget();
    menu_ = {};
    list_.Hide();
}

bool TextService::PrivateField(ITfContext* context) {
    // A password-style edit control: ES_PASSWORD, or a password character (Edit and RichEdit only).
    if (HWND focus = GetFocus()) {
        wchar_t name[64] = L"";
        GetClassNameW(focus, name, 64);
        for (wchar_t& c : name) c = static_cast<wchar_t>(towlower(c));
        if (wcsstr(name, L"edit")) {
            if (GetWindowLongW(focus, GWL_STYLE) & ES_PASSWORD) return Private(L"ES_PASSWORD");
            if (SendMessageW(focus, EM_GETPASSWORDCHAR, 0, 0) != 0) return Private(L"password character");
        }
    }
    if (!context) return false;
    // The app turned the keyboard off for this field (password boxes through the IMM layer).
    for (const GUID* guid : {&GUID_COMPARTMENT_KEYBOARD_DISABLED, &GUID_COMPARTMENT_EMPTYCONTEXT}) {
        ComPtr<ITfCompartment> compartment;
        if (SUCCEEDED(GetCompartment(context, *guid, &compartment)) && ReadCompartment(compartment.Get()) != 0) {
            return Private(guid == &GUID_COMPARTMENT_KEYBOARD_DISABLED ? L"keyboard disabled" : L"empty context");
        }
    }
    // A password or PIN by its input scope (browsers' password inputs, XAML PasswordBox...).
    // It's an app property (GetAppProperty): GetProperty fails for it (measured 2026-11-11).
    bool secret = false;
    RunReadSessionNow(context, client_id_, [&](TfEditCookie ec) -> HRESULT {
        ComPtr<ITfReadOnlyProperty> property;
        TF_SELECTION selection = {};
        ULONG fetched = 0;
        if (FAILED(context->GetAppProperty(kPropInputScope, &property)) || !property ||
            FAILED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &fetched)) || !fetched ||
            !selection.range) {
            return S_OK;
        }
        VARIANT value;
        VariantInit(&value);
        ComPtr<ITfInputScope> input_scope;
        if (SUCCEEDED(property->GetValue(ec, selection.range, &value)) && value.vt == VT_UNKNOWN && value.punkVal &&
            SUCCEEDED(value.punkVal->QueryInterface(IID_PPV_ARGS(&input_scope)))) {
            InputScope* list = nullptr;
            UINT count = 0;
            if (SUCCEEDED(input_scope->GetInputScopes(&list, &count)) && list) {
                for (UINT i = 0; i < count; ++i) {
                    switch (list[i]) {
                        case IS_PASSWORD: case IS_PRIVATE: case IS_NUMERIC_PASSWORD:
                        case IS_NUMERIC_PIN: case IS_ALPHANUMERIC_PIN: case IS_ALPHANUMERIC_PIN_SET:
                            secret = true;
                            break;
                        default:
                            break;
                    }
                }
                CoTaskMemFree(list);
            }
        }
        VariantClear(&value);
        selection.range->Release();
        return S_OK;
    });
    return secret ? Private(L"input scope") : false;
}

bool TextService::Private(const wchar_t* why) {
    if (!private_) log::Write(L"private field (%s): nothing composes, nothing typed is published", why);
    return true;
}

STDMETHODIMP TextService::OnSetFocus(BOOL foreground) {
    tested_ = {};
    if (foreground) {
        ApplySettings();
        Publish();
    }
    return S_OK;
}

bool TextService::Process(ITfContext* context, WPARAM wparam, LPARAM lparam) {
    KeyDown k;
    k.vk = static_cast<unsigned>(wparam);
    k.scan = ScanOrMapped(k.vk, static_cast<unsigned>((lparam >> 16) & 0xFF),
                          [](unsigned v) { return static_cast<unsigned>(MapVirtualKeyW(v, MAPVK_VK_TO_VSC)); });
    k.extended = (lparam & (LPARAM{1} << 24)) != 0;
    k.repeat = (lparam & (LPARAM{1} << 30)) != 0;
    k.mods = Modifiers();
    k.caps = (GetKeyState(VK_CAPITAL) & 1) != 0;
    // Our own Caps Lock press (SetCapsLock), coming back: Windows' alone.
    if (k.vk == VK_CAPITAL && ignore_caps_ > 0) {
        --ignore_caps_;
        if (GetTickCount64() - caps_injected_tick_ < kCapsEcho) return false;
        ignore_caps_ = 0;
    }
    const LangKey lang = LangKeyOf(k);
    // The Japanese layout's state keys (VK_DBE_ALPHANUMERIC, _KATAKANA, _HIRAGANA: 英数 and カタカナ)
    // never come back released (measured 2026-11-11, injected: the only key-up is a synthesized one
    // of the previous state key, before the press), so every press after the first says "already
    // down". Each press counts. [measure] whether a physical key held down repeats.
    if (k.vk == vk::kDbeAlphanumeric || k.vk == vk::kDbeKatakana || k.vk == vk::kDbeHiragana) k.repeat = false;
    const std::string code = CodeOf(k);
    if (lang == LangKey::kNone && code.empty()) return false;  // a modifier on its own
    if (!Target(context) && lang == LangKey::kNone) {
        NoContext(k.vk);
        return false;
    }
    if (!EnsureEngine()) return false;
    Settle();
    CheckOrphan();
    if (!Composing() && lang == LangKey::kNone) private_ = PrivateField(Target(context));
    TsfField field(this, Target(context));
    // A password field or a program the IME is off in: nothing composes there (the language keys
    // still switch mode, for the next field).
#ifdef BW_KEY_LOG  // a development build's per-key line (docs/testing.md): never in a release, it logs keys
    const int mode_before = static_cast<int>(host_->mode());
#endif
    const bool had_text = Composing() || orphan_.Get() != nullptr;
    bool eaten = (private_ || app_off_) && lang == LangKey::kNone ? host_->TypeOnly(k, field)
                                                                   : host_->Key(k, field);
    // A key that commits and then reaches the app. Through the IMM layer a Japanese profile's commit
    // reaches the app after that key (measured 2026-11-11: Space typed " H" for "H "; a ko-KR profile
    // keeps the order), so the key is eaten and sent again, to arrive after the commit. Only keys
    // without modifiers: sending a shortcut again would fake modifiers the user may have let go of.
    // VK_PACKET can't be sent again (its lParam doesn't carry the character).
    if (!eaten && had_text && lang == LangKey::kNone && k.vk != VK_PACKET && !k.mods.shift && k.mods.Plain() &&
        Transitory(Target(context))) {
        SendAgain(k);
        eaten = true;
    }
#ifdef BW_KEY_LOG
    log::Write(L"KEY vk=%02X scan=%02X ext=%d rep=%d sh=%d caps=%d lang=%d code=%S mode %d->%d eaten=%d composing=%d private=%d ctx=%d",
               k.vk, k.scan, k.extended, k.repeat, k.mods.shift, k.caps, static_cast<int>(lang), code.c_str(),
               mode_before, static_cast<int>(host_->mode()), eaten, Composing(), private_, context != nullptr);
#endif
    AfterHost();
    return eaten;
}

void TextService::AfterHost() {
    if (!host_) return;
    if (host_->TakeStateChanged()) {
        mode_ = host_->mode();
        profile_ = host_->profile();
        WriteSession();
        SetOpen(mode_ == Mode::kHenkan);
    }
    if (const auto caps = host_->TakeCapsWanted()) SetCapsLock(*caps);
    UpdateIndicator();
    ShowList();
    Publish();
}

STDMETHODIMP TextService::OnTestKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) {
    // The engine runs here, not in OnKeyDown: a key reported as eaten here never reaches the app,
    // even if OnKeyDown then hands it back (jis-moonshot measured Enter, arrows and Tab lost). So a
    // key the engine doesn't consume is committed before and reported as not eaten; OnKeyDown, which
    // follows for eaten keys, takes the result from here.
    *eaten = FALSE;
    const unsigned vk = static_cast<unsigned>(wparam);
    // The same key-down tested again (a message loop that peeks before it takes): the engine
    // already ran for it (jis-moonshot #5).
    const LONG time = GetMessageTime();
    const ULONGLONG tick = GetTickCount64();
    // A key we sent again after its commit (Process), coming back: the app's. Before the check
    // below, which would take it for the original (same lParam) while that is still held.
    if (resent_vk_ && vk == resent_vk_) {
        resent_vk_ = 0;
        if (tick - resent_tick_ < kCapsEcho) {
            tested_ = {wparam, lparam, time, tick, FALSE, true, true};
            if (vk < eaten_down_.size()) eaten_down_[vk] = false;
            return S_OK;
        }
    }
    if (tested_.valid &&
        SameKeyDown(tested_.wparam, tested_.lparam, tested_.time, tested_.tick, wparam, lparam, time, tick)) {
        *eaten = tested_.eaten;
        return S_OK;
    }
    tested_ = {};
    *eaten = Process(context, wparam, lparam) ? TRUE : FALSE;
    tested_ = {wparam, lparam, time, tick, *eaten, true, true};
    if (vk < eaten_down_.size()) eaten_down_[vk] = *eaten != FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) {
    *eaten = FALSE;
    // The same key, already handled by OnTestKeyDown. Its lParam comes without the repeat count here
    // (measured), so compare the scan code and extended flag. Apps going through the IMM layer call
    // OnKeyDown even for keys OnTestKeyDown didn't eat (measured: Enter).
    constexpr LPARAM kScanAndExtended = 0x01FF0000;
    if (tested_.pending && tested_.wparam == wparam &&
        (tested_.lparam & kScanAndExtended) == (lparam & kScanAndExtended)) {
        tested_.pending = false;
        *eaten = tested_.eaten;
        return S_OK;
    }
    tested_ = {};
    const unsigned vk = static_cast<unsigned>(wparam);
    *eaten = Process(context, wparam, lparam) ? TRUE : FALSE;
    if (vk < eaten_down_.size()) eaten_down_[vk] = *eaten != FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnTestKeyUp(ITfContext*, WPARAM wparam, LPARAM, BOOL* eaten) {
    if (tested_.wparam == wparam) tested_ = {};
    *eaten = wparam < eaten_down_.size() && eaten_down_[wparam];
    return S_OK;
}

STDMETHODIMP TextService::OnKeyUp(ITfContext*, WPARAM wparam, LPARAM, BOOL* eaten) {
    if (tested_.wparam == wparam) tested_ = {};
    // The key-up of a key-down the app never saw stays away from it too.
    *eaten = wparam < eaten_down_.size() && eaten_down_[wparam];
    if (wparam < eaten_down_.size()) eaten_down_[wparam] = false;
    return S_OK;
}

STDMETHODIMP TextService::OnPreservedKey(ITfContext*, REFGUID, BOOL* eaten) {
    *eaten = FALSE;
    return S_OK;
}

void TextService::SendAgain(const KeyDown& key) {
    INPUT in[2] = {};
    in[0].type = in[1].type = INPUT_KEYBOARD;
    in[0].ki.wVk = in[1].ki.wVk = static_cast<WORD>(key.vk);
    in[0].ki.wScan = in[1].ki.wScan = static_cast<WORD>(key.scan);
    in[0].ki.dwFlags = key.extended ? KEYEVENTF_EXTENDEDKEY : 0;
    in[1].ki.dwFlags = in[0].ki.dwFlags | KEYEVENTF_KEYUP;
    resent_vk_ = key.vk;
    resent_tick_ = GetTickCount64();
    if (SendInput(2, in, sizeof(INPUT)) != 2) {
        resent_vk_ = 0;
        log::Write(L"sending a key again after its commit: SendInput failed (%lu)", GetLastError());
    }
}

void TextService::SetCapsLock(bool on) {
    if (((GetKeyState(VK_CAPITAL) & 1) != 0) == on) return;
    // Windows' lock state is set by a key press only: one injected press (FKeyIME's SetCapsLock),
    // let through when it comes back (Process). Not measured: whether GetKeyState already holds the
    // toggle while the Caps Lock key-down itself is processed (docs/spec.md §9).
    INPUT in[2] = {};
    in[0].type = in[1].type = INPUT_KEYBOARD;
    in[0].ki.wVk = in[1].ki.wVk = VK_CAPITAL;
    in[1].ki.dwFlags = KEYEVENTF_KEYUP;
    ++ignore_caps_;
    caps_injected_tick_ = GetTickCount64();
    if (SendInput(2, in, sizeof(INPUT)) != 2) {
        --ignore_caps_;
        log::Write(L"Caps Lock: SendInput failed (%lu)", GetLastError());
    }
}

// --- text out

void TextService::ApplyOutput(ITfContext* context, const Output& out) {
    // A row other than entry 0 shown in place: the thick underline.
    const bool converted = !out.menu.empty() && out.selected < out.menu.size() && out.selected != out.zero;
    const bool synchronous = UpdateText(context, Wide(out.commit), Wide(out.preedit), converted, out.replace_selection);
    menu_ = out;
    // The engine never holds text the user can't see (jis-moonshot #3).
    if (context && synchronous && !out.preedit.empty() && !composition_ && !orphan_) {
        static bool logged = false;
        if (!logged) log::Write(L"unconfirmed text with nothing showing it: dropped");
        logged = true;
        DropInvisible();
    }
}

std::string TextService::ReadSelection(ITfContext* context) {
    if (!context) return {};
    std::wstring text;
    RunReadSessionNow(context, client_id_, [&](TfEditCookie ec) -> HRESULT {
        TF_SELECTION selection = {};
        ULONG fetched = 0;
        if (FAILED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &fetched)) || !fetched ||
            !selection.range) {
            return S_OK;
        }
        WCHAR buffer[256];
        ULONG got = 0;
        while (text.size() < kSelectionLimit &&
               SUCCEEDED(selection.range->GetText(ec, TF_TF_MOVESTART, buffer, ARRAYSIZE(buffer), &got)) && got) {
            text.append(buffer, got);
        }
        selection.range->Release();
        return S_OK;
    });
    // Classic Edit and RichEdit controls reached through the IMM layer: the transitory context holds
    // only the composition, never the app's text (measured 2026-11-11: 0 characters selected). Ask
    // the control itself; the text service runs in its process.
    if (text.empty()) text = ControlSelection(kSelectionLimit);
    return Utf8(text);
}

std::wstring TextService::ControlSelection(size_t limit) {
    HWND focus = GetFocus();
    if (!focus) return {};
    wchar_t name[64] = L"";
    GetClassNameW(focus, name, 64);
    for (wchar_t& c : name) c = static_cast<wchar_t>(towlower(c));
    if (!wcsstr(name, L"edit")) return {};
    // Never a password control's text, whatever called this.
    if ((GetWindowLongW(focus, GWL_STYLE) & ES_PASSWORD) || SendMessageW(focus, EM_GETPASSWORDCHAR, 0, 0) != 0) {
        return {};
    }
    DWORD start = 0, end = 0;
    SendMessageW(focus, EM_GETSEL, reinterpret_cast<WPARAM>(&start), reinterpret_cast<LPARAM>(&end));
    if (end <= start) return {};
    std::wstring text;
    if (wcsstr(name, L"richedit")) {
        // RichEdit counts a new line as one character; EM_GETSELTEXT gives the text as it counts it.
        text.resize(static_cast<size_t>(end - start) + 1);
        const LRESULT got = SendMessageW(focus, EM_GETSELTEXT, 0, reinterpret_cast<LPARAM>(text.data()));
        text.resize(got > 0 ? static_cast<size_t>(got) : 0);
    } else {
        const LRESULT length = SendMessageW(focus, WM_GETTEXTLENGTH, 0, 0);
        if (length <= 0 || static_cast<DWORD>(length) < end) return {};
        std::wstring all(static_cast<size_t>(length) + 1, L'\0');
        const LRESULT got = SendMessageW(focus, WM_GETTEXT, all.size(), reinterpret_cast<LPARAM>(all.data()));
        if (got < static_cast<LRESULT>(end)) return {};
        text = all.substr(start, end - start);
    }
    if (text.size() > limit) text.resize(limit);
    return text;
}

void TextService::ShowList() {
    if (!menu_.menu.empty()) {
        list_.Show(menu_, anchor_);
    } else {
        list_.Hide();
    }
}

void TextService::Flush(ITfContext* context) {
    Settle();
    if (!host_) return;
    TsfField field(this, context);
    host_->Flush(field);
    AfterHost();
}

// --- the session's mode

bool TextService::ReadSession(Mode* mode, std::string* profile) {
    const Product* product = GetProduct();
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> compartment;
    if (!product || !thread_mgr_ || FAILED(thread_mgr_->GetGlobalCompartment(&global)) ||
        FAILED(global->GetCompartment(product->clsid, &compartment))) {
        return false;
    }
    // 0: never set in this session. Else 1 + mode + 2 × the profile's index in kProfiles.
    const LONG value = ReadCompartment(compartment.Get());
    if (value <= 0) return false;
    *mode = (value - 1) % 2 ? Mode::kHenkan : Mode::kMuhenkan;
    const LONG index = (value - 1) / 2;
    if (index >= 0 && index < static_cast<LONG>(std::size(kProfiles))) *profile = kProfiles[index];
    return true;
}

void TextService::WriteSession() {
    const Product* product = GetProduct();
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> compartment;
    if (!product || !thread_mgr_ || FAILED(thread_mgr_->GetGlobalCompartment(&global)) ||
        FAILED(global->GetCompartment(product->clsid, &compartment))) {
        return;
    }
    LONG index = 0;
    for (LONG i = 0; i < static_cast<LONG>(std::size(kProfiles)); ++i) {
        if (profile_ == kProfiles[i]) index = i;
    }
    WriteCompartment(compartment.Get(), client_id_, 1 + static_cast<LONG>(mode_) + 2 * index);
}

void TextService::SetOpen(bool open) {
    // The thread's open/close compartment is what the input indicator and apps read: open in henkan.
    ComPtr<ITfCompartment> openclose;
    if (thread_mgr_ && SUCCEEDED(GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose)) &&
        (ReadCompartment(openclose.Get()) != 0) != open) {
        WriteCompartment(openclose.Get(), client_id_, open ? 1 : 0);
    }
}

void TextService::AdviseCompartments(bool on) {
    const Product* product = GetProduct();
    if (!thread_mgr_ || !product) return;
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> session, openclose;
    if (SUCCEEDED(thread_mgr_->GetGlobalCompartment(&global))) global->GetCompartment(product->clsid, &session);
    GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose);
    struct Pair {
        ITfCompartment* compartment;
        DWORD* cookie;
    };
    for (Pair p : {Pair{session.Get(), &session_sink_cookie_}, Pair{openclose.Get(), &openclose_sink_cookie_}}) {
        ComPtr<ITfSource> source;
        if (!p.compartment || FAILED(p.compartment->QueryInterface(IID_PPV_ARGS(&source)))) continue;
        if (on && *p.cookie == TF_INVALID_COOKIE) {
            source->AdviseSink(IID_ITfCompartmentEventSink, static_cast<ITfCompartmentEventSink*>(this), p.cookie);
        } else if (!on && *p.cookie != TF_INVALID_COOKIE) {
            source->UnadviseSink(*p.cookie);
            *p.cookie = TF_INVALID_COOKIE;
        }
    }
}

STDMETHODIMP TextService::OnChange(REFGUID guid) {
    const Product* product = GetProduct();
    if (product && IsEqualGUID(guid, product->clsid)) {
        // Another thread or app switched mode or profile: follow it, committing what's staged.
        // Cleared (0, as bw_testpad does): back to the manifest's mode.
        Mode mode = product->manifest.mode == "muhenkan" ? Mode::kMuhenkan : Mode::kHenkan;
        std::string profile = profile_;
        ReadSession(&mode, &profile);
        if (mode == mode_ && profile == profile_) return S_OK;
        mode_ = mode;
        profile_ = profile;
        if (host_) {
            TsfField field(this, Target(nullptr));
            host_->Follow(mode, profile, field);
            host_->TakeStateChanged();
        }
        SetOpen(mode_ == Mode::kHenkan);
        UpdateIndicator();
        ShowList();
        Publish();
    } else if (IsEqualGUID(guid, GUID_COMPARTMENT_KEYBOARD_OPENCLOSE) && thread_mgr_) {
        // The open/close compartment only mirrors the mode. Windows closes it when a profile activates
        // (jis-moonshot measured it for Korean), so a change from elsewhere is put back: the language
        // keys, Caps Lock, the indicator and the menu switch.
        ComPtr<ITfCompartment> openclose;
        const bool open = mode_ == Mode::kHenkan;
        if (SUCCEEDED(GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose)) &&
            (ReadCompartment(openclose.Get()) != 0) != open) {
            log::Write(L"open/close set from outside; put back");
            WriteCompartment(openclose.Get(), client_id_, open ? 1 : 0);
        }
    }
    return S_OK;
}

void TextService::UpdateIndicator() {
    const Product* product = GetProduct();
    if (!lang_bar_ || !product) return;
    const Manifest& m = product->manifest;
    std::wstring tooltip = product->display_name;
    if (!SharedModule() && ProductError().empty()) tooltip += L" (the engine didn't load: see the log)";
    tooltip += mode_ == Mode::kHenkan ? L": henkan" : L": muhenkan";
    if (mode_ == Mode::kHenkan && engine_ && engine_->HasProfiles()) tooltip += L" (" + Wide(profile_) + L")";
    if (app_off_) tooltip += L", off in " + settings::ExeName();
    lang_bar_->SetState(Wide(mode_ == Mode::kHenkan ? m.label_henkan : m.label_muhenkan), tooltip);
}

// --- the published state (docs/api.md)

void TextService::Publish() {
    if (!publish_ || !active_) return;
    state::Table* table = shared::Writable();
    if (!table) return;
    if (!slot_) {
        slot_ = state::Claim(table, GetCurrentThreadId(), GetCurrentProcessId(), shared::ThreadAlive);
        if (!slot_) {
            static bool logged = false;
            if (!logged) log::Write(L"state table full");
            logged = true;
            return;
        }
    }
    const Product* product = GetProduct();
    HostInfo info;
    info.ime = product ? product->manifest.name : "";
    info.version = BW_VERSION;
    info.app = Utf8(settings::ExeName());
    info.mode = mode_;
    info.profile = profile_;
    info.profiles = engine_ && engine_->HasProfiles();
    info.caps_lock = (GetKeyState(VK_CAPITAL) & 1) != 0;
    info.secure = private_;
    info.off = app_off_;
    std::uint32_t flags = 0;
    if (private_) flags |= state::kFlagSecure;
    if (app_off_) flags |= state::kFlagOff;
    if (Composing()) flags |= state::kFlagComposing;
    if (!engine_ || !engine_->HasState()) flags |= state::kFlagNoStaged;
    const std::string engine_state = host_ ? host_->EngineState() : std::string();
    state::Publish(table, slot_, static_cast<std::int32_t>(mode_), flags, PublishedState(engine_state, info),
                   static_cast<std::int64_t>(GetTickCount64()));
    // Readers that don't poll hear about mode, profile, secure input and composing changes.
    LONG index = 0;
    for (LONG i = 0; i < static_cast<LONG>(std::size(kProfiles)); ++i) {
        if (profile_ == kProfiles[i]) index = i;
    }
    const LPARAM notice = static_cast<LPARAM>(static_cast<LONG>(mode_) | (flags << 8) | (index << 16));
    if (notice != notified_) {
        notified_ = notice;
        PostMessageW(HWND_BROADCAST, shared::StateMessage(), GetCurrentThreadId(), notice & 0xFFFF);
    }
}

void TextService::ReleaseSlot() {
    if (!slot_) return;
    if (state::Table* table = shared::Writable()) state::Release(table, GetCurrentThreadId());
    slot_ = nullptr;
    notified_ = -1;
}

// --- the menu

void TextService::InitMenu(ITfMenu* menu) {
    auto add = [](ITfMenu* m, UINT id, const std::wstring& text, bool checked, bool enabled = true) {
        DWORD flags = checked ? TF_LBMENUF_CHECKED : 0;
        if (!enabled) flags |= TF_LBMENUF_GRAYED;
        m->AddMenuItem(id, flags, nullptr, nullptr, text.c_str(), static_cast<ULONG>(text.size()), nullptr);
    };
    auto separator = [](ITfMenu* m) { m->AddMenuItem(0, TF_LBMENUF_SEPARATOR, nullptr, nullptr, L"", 0, nullptr); };
    auto submenu = [](ITfMenu* m, const std::wstring& title) {
        ITfMenu* sub = nullptr;
        m->AddMenuItem(0, TF_LBMENUF_SUBMENU, nullptr, nullptr, title.c_str(), static_cast<ULONG>(title.size()), &sub);
        return sub;
    };
    const HostSettings h = host_ ? host_->settings() : settings::ReadHost();
    add(menu, kMenuHenkan, L"Henkan (変換)", mode_ == Mode::kHenkan);
    add(menu, kMenuMuhenkan, L"Muhenkan (無変換)", mode_ == Mode::kMuhenkan);
    if (engine_ && engine_->HasProfiles()) {
        if (ITfMenu* sub = submenu(menu, L"Henkan Profile")) {
            const wchar_t* labels[] = {L"henkan (変換)", L"katakana (カタカナ)", L"mac-kana (かな)"};
            for (UINT i = 0; i < std::size(kProfiles); ++i) {
                add(sub, kMenuProfileFirst + i, labels[i], profile_ == kProfiles[i]);
            }
            sub->Release();
        }
    }
    separator(menu);
    if (ITfMenu* sub = submenu(menu, L"Caps Lock")) {
        add(sub, kMenuCapsFirst + 0, L"Windows' Caps Lock", h.caps == CapsSetting::kRespect);
        add(sub, kMenuCapsFirst + 1, L"Switch Mode (Shift + Caps Lock: Caps Lock, in Muhenkan)",
            h.caps == CapsSetting::kToggle);
        add(sub, kMenuCapsFirst + 2, L"Caps Lock in Muhenkan Only", h.caps == CapsSetting::kTransient);
        separator(sub);
        add(sub, kMenuRuleFirst + 0, L"ALL CAPS", h.caps_rule == CapsRule::kAllCaps);
        add(sub, kMenuRuleFirst + 1, L"UPPER_SNAKE (Space Types _)", h.caps_rule == CapsRule::kUpperSnake);
        sub->Release();
    }
    if (ITfMenu* sub = submenu(menu, L"Keys the Engine Passes On")) {
        add(sub, kMenuPassJis, L"Typed as on a JIS Keyboard", h.pass_on == PassOn::kJis);
        add(sub, kMenuPassWindows, L"Windows' Keyboard Layout", h.pass_on == PassOn::kWindows);
        sub->Release();
    }
    if (ITfMenu* sub = submenu(menu, L"かな Key (Apple Keyboards)")) {
        add(sub, kMenuKanaMacKana, L"Henkan, mac-kana Profile", !h.kana_profile.empty());
        add(sub, kMenuKanaNone, L"Passed to the App", h.kana_profile.empty());
        sub->Release();
    }
    separator(menu);
    add(menu, kMenuAppOff, L"Off in " + settings::ExeName(), app_off_);
    add(menu, kMenuPublish, L"Show State to Other Programs (docs/api.md)", publish_);
    add(menu, kMenuLog, L"Diagnostic Log", settings::ReadDword(L"DiagnosticLog", 0) != 0);
    separator(menu);
    const Product* product = GetProduct();
    std::wstring about = (product ? product->display_name : L"") + L", bada-windows " + Wide(BW_VERSION);
    add(menu, kMenuAbout, about, false, false);
}

void TextService::OnMenu(UINT id) {
    HostSettings h = host_ ? host_->settings() : settings::ReadHost();
    if (id == kMenuHenkan || id == kMenuMuhenkan) {
        if (!EnsureEngine()) return;
        TsfField field(this, Target(nullptr));
        host_->SwitchMode(id == kMenuHenkan ? Mode::kHenkan : Mode::kMuhenkan, field);
        AfterHost();
        return;
    }
    if (id >= kMenuProfileFirst && id < kMenuProfileFirst + std::size(kProfiles)) {
        if (!EnsureEngine()) return;
        TsfField field(this, Target(nullptr));
        if (host_->mode() == Mode::kMuhenkan) host_->SwitchMode(Mode::kHenkan, field);
        host_->SwitchProfile(kProfiles[id - kMenuProfileFirst], field);
        AfterHost();
        return;
    }
    if (id >= kMenuCapsFirst && id < kMenuCapsFirst + 3) {
        h.caps = static_cast<CapsSetting>(id - kMenuCapsFirst);
    } else if (id >= kMenuRuleFirst && id < kMenuRuleFirst + 2) {
        h.caps_rule = static_cast<CapsRule>(id - kMenuRuleFirst);
    } else if (id == kMenuPassJis || id == kMenuPassWindows) {
        h.pass_on = id == kMenuPassJis ? PassOn::kJis : PassOn::kWindows;
    } else if (id == kMenuKanaMacKana || id == kMenuKanaNone) {
        h.kana_profile = id == kMenuKanaMacKana ? kProfileMacKana : "";
    } else if (id == kMenuAppOff) {
        if (ITfContext* pending = Target(nullptr); !app_off_ && pending) Flush(pending);
        settings::SetAppOff(settings::ExeName(), !app_off_);
        ApplySettings();
        UpdateIndicator();
        Publish();
        return;
    } else if (id == kMenuPublish) {
        settings::WriteDword(L"PublishState", publish_ ? 0 : 1);
        ApplySettings();
        Publish();
        return;
    } else if (id == kMenuLog) {
        settings::WriteDword(L"DiagnosticLog", settings::ReadDword(L"DiagnosticLog", 0) ? 0 : 1);
        return;
    } else {
        return;
    }
    settings::WriteHost(h);
    ApplySettings();
}

// --- focus and profiles

STDMETHODIMP TextService::OnSetFocus(ITfDocumentMgr* focus, ITfDocumentMgr*) {
    // Focus moved to another document: what was unconfirmed is committed where it was.
    tested_ = {};
    if (ITfContext* pending = Target(nullptr)) {
        Flush(pending);
    } else if (Composing()) {
        DropInvisible();  // text that never reached a document doesn't follow focus (jis-moonshot #3)
    }
    list_.Hide();
    ApplySettings();
    // Whether the new field is a password field, for what's published (docs/api.md).
    ComPtr<ITfContext> top;
    private_ = focus && SUCCEEDED(focus->GetTop(&top)) && top ? PrivateField(top.Get()) : false;
    UpdateIndicator();
    Publish();
    return S_OK;
}

STDMETHODIMP TextService::OnActivated(REFCLSID clsid, REFGUID, BOOL activated) {
    const Product* product = GetProduct();
    if (!product || !IsEqualGUID(clsid, product->clsid)) return S_OK;
    if (activated) {
        ApplySettings(true);
        Publish();
    } else {
        if (ITfContext* pending = Target(nullptr)) Flush(pending);
        ReleaseSlot();
    }
    return S_OK;
}

STDMETHODIMP TextService::OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) {
    // The app ended the composition (a click elsewhere...): its text stays as shown, and the engine
    // starts fresh.
    if (!composition_ || composition != composition_.Get()) return S_OK;
    const ULONGLONG since = GetTickCount64() - updated_tick_;
    static bool logged = false;
    if (!logged) log::Write(L"composition terminated %s%llu ms after update", in_update_ ? L"during the update, " : L"", since);
    logged = true;
    ComPtr<ITfRange> range;
    if (FAILED(composition->GetRange(&range))) range.Reset();
    if (range && composition_context_ && CaretInside(ec, composition_context_.Get(), range.Get())) {
        CaretToEnd(ec, composition_context_.Get(), range.Get());
    }
    // Ended by the app right after our own update, not by the user (jis-moonshot #6): the text stays
    // as plain text, the engine keeps its segment, and the next update replaces it. Not through the
    // IMM layer (a transitory context): the text leaves the document there (measured).
    TF_STATUS status = {};
    const bool transitory = composition_context_ && SUCCEEDED(composition_context_->GetStatus(&status)) &&
                            (status.dwStaticFlags & TF_SS_TRANSITORY) != 0;
    if (kKeepTextWithoutComposition && range && !transitory && Composing() && (in_update_ || since < kOwnTermination)) {
        ComPtr<ITfRange> orphan;
        if (SUCCEEDED(range->Clone(&orphan)) && orphan) {
            orphan->SetGravity(ec, TF_GRAVITY_FORWARD, TF_GRAVITY_BACKWARD);
            orphan_ = orphan;
            orphan_context_ = composition_context_;
            composition_.Reset();
            composition_context_.Reset();
            return S_OK;
        }
    }
    composition_.Reset();
    composition_context_.Reset();
    DropInvisible();
    Publish();
    return S_OK;
}

STDMETHODIMP TextService::EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** infos) {
    if (!infos) return E_INVALIDARG;
    *infos = NewDisplayAttributeEnum();
    return *infos ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP TextService::GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo** info) {
    if (!info) return E_INVALIDARG;
    *info = NewDisplayAttributeInfo(guid);
    return *info ? S_OK : E_INVALIDARG;
}

// --- the composition

bool TextService::UpdateText(ITfContext* context, const std::wstring& commit, const std::wstring& preedit,
                             bool converted, bool replace_selection) {
    if (!context) return true;
    // A composition left in another document (focus moved) ends there as it is.
    if (composition_ && composition_context_ && !SameObject(composition_context_.Get(), context)) {
        ComPtr<ITfComposition> old = composition_;
        RunEditSession(composition_context_.Get(), client_id_, [old](TfEditCookie ec) { return old->EndComposition(ec); });
        composition_.Reset();
        composition_context_.Reset();
    }
    if (orphan_ && !SameObject(orphan_context_.Get(), context)) {
        orphan_.Reset();
        orphan_context_.Reset();
    }
    if (!composition_ && !orphan_ && commit.empty() && preedit.empty()) return true;
    // When TSF can't run the session at once it runs after this returns, maybe after Deactivate: it
    // holds the service, the context and the composition it was meant for, and copies of the text,
    // and never calls the engine (jis-moonshot #7).
    ComPtr<TextService> self(this);
    ComPtr<ITfContext> context_ref = context;
    ComPtr<ITfComposition> expected = composition_;
    const TfGuidAtom attribute = converted ? converted_attribute_ : input_attribute_;
    auto edit = [self, context_ref, expected, commit, preedit, attribute, replace_selection](TfEditCookie ec) -> HRESULT {
        return self->EditText(ec, context_ref.Get(), expected.Get(), commit, preedit, attribute, replace_selection);
    };
    bool async = false;
    in_update_ = true;
    const HRESULT hr = RunEditSession(context, client_id_, edit, &async);
    in_update_ = false;
    updated_tick_ = GetTickCount64();
    if (FAILED(hr)) log::Write(L"edit session failed %#lx", hr);
    if (async) {
        static bool logged = false;
        if (!logged) log::Write(L"edit session queued (async): the text lands after the key");
        logged = true;
        return false;
    }
    return true;
}

HRESULT TextService::EditText(TfEditCookie ec, ITfContext* context, ITfComposition* expected,
                              const std::wstring& commit, const std::wstring& preedit, TfGuidAtom attribute_atom,
                              bool replace_selection) {
    // Deactivated since this was requested: end what's shown with its commit, start nothing.
    if (!active_) {
        ComPtr<ITfRange> range;
        if (expected && SUCCEEDED(expected->GetRange(&range)) && range && !commit.empty()) {
            range->SetText(ec, 0, commit.data(), static_cast<LONG>(commit.size()));
        }
        if (expected) expected->EndComposition(ec);
        return S_OK;
    }
    if (expected && composition_.Get() != expected) return S_OK;  // ended in between: left as shown
    ComPtr<ITfProperty> attribute;
    if (FAILED(context->GetProperty(GUID_PROP_ATTRIBUTE, &attribute))) attribute.Reset();
    // Apps that reach TSF through the IMM compatibility layer (a transitory context) end a
    // composition with a plain caret at once: the composition is selected as an interim character
    // there, set on the first character and kept by TSF as it grows (FKeyIME
    // docs/issues/korean-jamo-not-composed.md, jis-moonshot's notes).
    TF_STATUS status = {};
    const bool transitory = SUCCEEDED(context->GetStatus(&status)) && (status.dwStaticFlags & TF_SS_TRANSITORY) != 0;
    auto show = [&](ITfRange* range) -> HRESULT {
        if (!transitory) return CaretToEnd(ec, context, range);
        TF_SELECTION selection = {};
        selection.range = range;
        selection.style.ase = TF_AE_NONE;
        selection.style.fInterimChar = TRUE;
        if (SUCCEEDED(context->SetSelection(ec, 1, &selection))) return S_OK;
        TF_SELECTION now = {};
        ULONG fetched = 0;
        bool kept = false;
        if (SUCCEEDED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &now, &fetched)) && fetched && now.range) {
            LONG start = 1, end = 1;
            kept = now.style.fInterimChar && SUCCEEDED(now.range->CompareStart(ec, range, TF_ANCHOR_START, &start)) &&
                   SUCCEEDED(now.range->CompareEnd(ec, range, TF_ANCHOR_END, &end)) && start == 0 && end == 0;
            now.range->Release();
        }
        if (kept) return S_OK;
        return CaretToEnd(ec, context, range);
    };
    auto place = [&](ITfRange* range) {
        RECT rect = {};
        BOOL clipped = FALSE;
        ComPtr<ITfContextView> view;
        if (FAILED(context->GetActiveView(&view)) || FAILED(view->GetTextExt(ec, range, &rect, &clipped)) ||
            IsRectEmpty(&rect)) {
            GUITHREADINFO gui = {sizeof(gui)};
            if (GetGUIThreadInfo(GetCurrentThreadId(), &gui) && gui.hwndCaret) {
                rect = gui.rcCaret;
                MapWindowPoints(gui.hwndCaret, nullptr, reinterpret_cast<POINT*>(&rect), 2);
            }
        }
        anchor_ = rect;
    };
    auto mark = [&](ITfRange* range) {
        if (attribute && attribute_atom != TF_INVALID_GUIDATOM) {
            VARIANT v;
            v.vt = VT_I4;
            v.lVal = static_cast<LONG>(attribute_atom);
            attribute->SetValue(ec, range, &v);
        }
        place(range);
    };
    auto set_text = [&](ITfRange* range, const std::wstring& text) {
        const HRESULT hr = range->SetText(ec, 0, text.data(), static_cast<LONG>(text.size()));
        if (FAILED(hr)) log::Write(L"SetText on the composition failed %#lx", hr);
        return hr;
    };
    ComPtr<ITfInsertAtSelection> insert;
    ComPtr<ITfContextComposition> compositions;
    if (FAILED(context->QueryInterface(IID_PPV_ARGS(&insert))) ||
        FAILED(context->QueryInterface(IID_PPV_ARGS(&compositions)))) {
        insert.Reset();
        compositions.Reset();
    }
    auto compose = [&](ITfRange* over) -> bool {
        ComPtr<ITfComposition> started;
        ComPtr<ITfRange> range;
        HRESULT hr = compositions ? compositions->StartComposition(ec, over, this, &started) : E_NOINTERFACE;
        if (SUCCEEDED(hr) && started && (FAILED(started->GetRange(&range)) || !range)) {
            started->EndComposition(ec);
            started.Reset();
            hr = E_FAIL;
        }
        if (FAILED(hr) || !started) {
            static bool logged = false;
            if (!logged) log::Write(L"composition refused hr=%#lx", hr);
            logged = true;
            return false;
        }
        composition_ = started;
        composition_context_ = context;
        set_text(range.Get(), preedit);
        mark(range.Get());
        show(range.Get());
        return true;
    };

    if (replace_selection && !commit.empty()) {
        // bada-compatible §7.1: the commit replaces the selection, even with text marked. What's
        // marked stays as it is shown.
        if (composition_) {
            ComPtr<ITfRange> range;
            if (SUCCEEDED(composition_->GetRange(&range)) && range && attribute) attribute->Clear(ec, range.Get());
            composition_->EndComposition(ec);
            composition_.Reset();
            composition_context_.Reset();
        }
        orphan_.Reset();
        orphan_context_.Reset();
        InsertAtSelection(ec, context, commit);
    } else if (orphan_) {
        ComPtr<ITfRange> orphan = orphan_;
        orphan_.Reset();
        orphan_context_.Reset();
        if (!commit.empty() || preedit.empty()) {
            set_text(orphan.Get(), commit);
            CaretToEnd(ec, context, orphan.Get());
        } else {
            if (compose(orphan.Get())) return S_OK;
            if (FAILED(set_text(orphan.Get(), preedit))) {
                refused_ = true;
                return E_FAIL;
            }
            orphan_ = orphan;
            orphan_context_ = context;
            place(orphan.Get());
            return CaretToEnd(ec, context, orphan.Get());
        }
    } else if (composition_) {
        ComPtr<ITfRange> range;
        if (FAILED(composition_->GetRange(&range)) || !range) {
            log::Write(L"the composition has no range: left as shown");
            composition_.Reset();
            composition_context_.Reset();
        } else if (commit.empty() && !preedit.empty()) {  // still composing: just update it
            set_text(range.Get(), preedit);
            mark(range.Get());
            return show(range.Get());
        } else {
            // The composition ends; its text becomes `commit`. The caret goes after it only if it's
            // still inside: a key the app already handled may have moved it.
            const bool inside = CaretInside(ec, context, range.Get());
            if (attribute) attribute->Clear(ec, range.Get());
            set_text(range.Get(), commit);
            if (inside) CaretToEnd(ec, context, range.Get());
            composition_->EndComposition(ec);
            composition_.Reset();
            composition_context_.Reset();
        }
    } else if (!commit.empty()) {
        InsertAtSelection(ec, context, commit);
    }
    if (preedit.empty()) return S_OK;

    ComPtr<ITfRange> at;
    if (!insert || FAILED(insert->InsertTextAtSelection(ec, TF_IAS_QUERYONLY, nullptr, 0, &at)) || !at) {
        refused_ = true;
        return E_FAIL;
    }
    if (compose(at.Get())) return S_OK;
    // The app refused a composition: the preedit goes in as plain text, replaced at the next update.
    ComPtr<ITfRange> plain;
    if (kKeepTextWithoutComposition &&
        SUCCEEDED(insert->InsertTextAtSelection(ec, 0, preedit.data(), static_cast<LONG>(preedit.size()), &plain)) &&
        plain) {
        plain->SetGravity(ec, TF_GRAVITY_FORWARD, TF_GRAVITY_BACKWARD);
        orphan_ = plain;
        orphan_context_ = context;
        place(plain.Get());
        return CaretToEnd(ec, context, plain.Get());
    }
    refused_ = true;
    return plain ? CaretToEnd(ec, context, plain.Get()) : InsertAtSelection(ec, context, preedit);
}

}  // namespace bw

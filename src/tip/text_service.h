// The text service: one COM object per thread with TSF, holding one engine. All typing behaviour is
// the engine's, and the host logic (modes, profiles, Caps Lock, keys passed on) is src/core/host.*:
// this class moves key events in and text out as a TSF composition, shows the engine's menu, keeps
// the mode the session's, and publishes the state for other programs (docs/api.md).
//
// One TSF language profile does everything (docs/spec.md §3): muhenkan and henkan are this
// service's own state, shared by every app in the session (a TSF global compartment), mirrored in
// the open/close compartment for the input indicator.
//
// Adapted from team-mirae/jis-moonshot windows/src/text_service.*: the key handling, the
// composition and the edit sessions keep the measured fixes of its docs/troubleshooting/
// windows-tsf-notes.md (see docs/tsf-notes.md here).
#pragma once

#include <msctf.h>
#include <windows.h>
#include <wrl/client.h>

#include <bitset>
#include <memory>
#include <optional>
#include <string>

#include "candidate_window.h"
#include "engine.h"
#include "host.h"
#include "lang_bar.h"
#include "state_table.h"

namespace bw {

class TextService final : public ITfTextInputProcessorEx,
                          public ITfKeyEventSink,
                          public ITfThreadMgrEventSink,
                          public ITfActiveLanguageProfileNotifySink,
                          public ITfCompositionSink,
                          public ITfCompartmentEventSink,
                          public ITfDisplayAttributeProvider {
public:
    TextService();
    TextService(const TextService&) = delete;
    TextService& operator=(const TextService&) = delete;

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** object) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // ITfTextInputProcessor(Ex)
    STDMETHODIMP Activate(ITfThreadMgr* thread_mgr, TfClientId client_id) override;
    STDMETHODIMP ActivateEx(ITfThreadMgr* thread_mgr, TfClientId client_id, DWORD flags) override;
    STDMETHODIMP Deactivate() override;

    // ITfKeyEventSink
    STDMETHODIMP OnSetFocus(BOOL foreground) override;
    STDMETHODIMP OnTestKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnTestKeyUp(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnKeyUp(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnPreservedKey(ITfContext* context, REFGUID guid, BOOL* eaten) override;

    // ITfThreadMgrEventSink
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    STDMETHODIMP OnSetFocus(ITfDocumentMgr* focus, ITfDocumentMgr* previous) override;
    STDMETHODIMP OnPushContext(ITfContext*) override { return S_OK; }
    STDMETHODIMP OnPopContext(ITfContext*) override { return S_OK; }

    // ITfActiveLanguageProfileNotifySink
    STDMETHODIMP OnActivated(REFCLSID clsid, REFGUID profile, BOOL activated) override;

    // ITfCompositionSink
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) override;

    // ITfCompartmentEventSink
    STDMETHODIMP OnChange(REFGUID guid) override;

    // ITfDisplayAttributeProvider
    STDMETHODIMP EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** infos) override;
    STDMETHODIMP GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo** info) override;

private:
    class TsfField;
    friend class TsfField;
    ~TextService();

    // The engine and the host, once the shared module has loaded. False: no engine (every key passes).
    bool EnsureEngine();
    // A key-down through the host. True: eaten.
    bool Process(ITfContext* context, WPARAM wparam, LPARAM lparam);
    // After any host call: the session's mode, Caps Lock, the indicator, the menu, the published state.
    void AfterHost();
    void ApplySettings(bool force = false);  // unless forced, only when settings::Version changed

    // A password or PIN field, or one the app turned the keyboard off for (jis-moonshot's checks).
    bool PrivateField(ITfContext* context);
    bool Private(const wchar_t* why);
    bool Composing() const;
    ITfContext* Target(ITfContext* context) const;
    HRESULT NoContext(unsigned vk);
    void DropInvisible();
    void CheckOrphan();
    void Flush(ITfContext* context);
    void Settle();

    // The engine's Output, applied to the document (bada-compatible §7.1).
    void ApplyOutput(ITfContext* context, const Output& out);
    std::string ReadSelection(ITfContext* context);
    static constexpr size_t kSelectionLimit = 4096;  // a longer selection is taken in part
    static std::wstring ControlSelection(size_t limit);  // a classic Edit or RichEdit's, via messages
    void ShowList();

    // The session's mode and profile (a global compartment), and the open/close mirror.
    bool ReadSession(Mode* mode, std::string* profile);
    void WriteSession();
    void SetOpen(bool open);
    void AdviseCompartments(bool on);
    void UpdateIndicator();
    void SetCapsLock(bool on);
    void Publish();
    void ReleaseSlot();

    void InitMenu(ITfMenu* menu);
    void OnMenu(UINT id);

    bool UpdateText(ITfContext* context, const std::wstring& commit, const std::wstring& preedit, bool converted,
                    bool replace_selection);
    HRESULT EditText(TfEditCookie ec, ITfContext* context, ITfComposition* expected, const std::wstring& commit,
                     const std::wstring& preedit, TfGuidAtom attribute, bool replace_selection);

    LONG refs_ = 1;
    std::unique_ptr<Engine> engine_;
    std::unique_ptr<Host> host_;
    Mode mode_ = Mode::kHenkan;  // before the engine: the session's, or the manifest's
    std::string profile_ = kProfileHenkan;
    bool active_ = false;
    bool refused_ = false;
    bool in_update_ = false;
    ULONGLONG updated_tick_ = 0;
    Microsoft::WRL::ComPtr<ITfThreadMgr> thread_mgr_;
    TfClientId client_id_ = TF_CLIENTID_NULL;
    DWORD thread_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD profile_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD session_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD openclose_sink_cookie_ = TF_INVALID_COOKIE;
    TfGuidAtom input_attribute_ = TF_INVALID_GUIDATOM;
    TfGuidAtom converted_attribute_ = TF_INVALID_GUIDATOM;
    Microsoft::WRL::ComPtr<ITfComposition> composition_;
    Microsoft::WRL::ComPtr<ITfContext> composition_context_;
    Microsoft::WRL::ComPtr<ITfRange> orphan_;
    Microsoft::WRL::ComPtr<ITfContext> orphan_context_;
    RECT anchor_ = {};
    Output menu_;  // the last Output with the menu to show
    CandidateWindow list_;
    LangBarButton* lang_bar_ = nullptr;
    std::bitset<256> eaten_down_;
    struct Tested {
        WPARAM wparam = 0;
        LPARAM lparam = 0;
        LONG time = 0;
        ULONGLONG tick = 0;
        BOOL eaten = FALSE;
        bool valid = false;
        bool pending = false;
    } tested_;
    bool private_ = false;
    bool app_off_ = false;
    bool publish_ = true;
    std::optional<unsigned long> applied_version_;
    int ignore_caps_ = 0;  // our own injected Caps Lock presses, to let through
    ULONGLONG caps_injected_tick_ = 0;
    // A key sent again after its commit (IMM layer, Process): let through when it comes back.
    unsigned resent_vk_ = 0;
    ULONGLONG resent_tick_ = 0;
    void SendAgain(const KeyDown& key);
    state::Slot* slot_ = nullptr;
    LPARAM notified_ = -1;  // what the last state message said
};

}  // namespace bw

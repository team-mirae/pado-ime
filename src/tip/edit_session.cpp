// Adapted from team-mirae/jis-moonshot windows/src/edit_session.*.
#include "edit_session.h"

#include <wrl/client.h>

#include <new>

#include "dll.h"

using Microsoft::WRL::ComPtr;

namespace bw {
namespace {

class EditSession final : public ITfEditSession {
public:
    explicit EditSession(std::function<HRESULT(TfEditCookie)> body) : body_(std::move(body)) {
        DllAddRef();
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        if (riid == IID_IUnknown || riid == IID_ITfEditSession) {
            *object = static_cast<ITfEditSession*>(this);
            AddRef();
            return S_OK;
        }
        *object = nullptr;
        return E_NOINTERFACE;
    }
    STDMETHODIMP_(ULONG) AddRef() override { return InterlockedIncrement(&refs_); }
    STDMETHODIMP_(ULONG) Release() override {
        const LONG refs = InterlockedDecrement(&refs_);
        if (!refs) delete this;
        return refs;
    }
    STDMETHODIMP DoEditSession(TfEditCookie ec) override { return body_(ec); }

private:
    ~EditSession() { DllRelease(); }
    LONG refs_ = 1;
    std::function<HRESULT(TfEditCookie)> body_;
};

}  // namespace

HRESULT RunEditSession(ITfContext* context, TfClientId client_id,
                       std::function<HRESULT(TfEditCookie)> body, bool* async) {
    if (async) *async = false;
    if (!context) return E_INVALIDARG;
    ComPtr<EditSession> session;
    session.Attach(new (std::nothrow) EditSession(std::move(body)));
    if (!session) return E_OUTOFMEMORY;
    HRESULT session_hr = E_FAIL;
    HRESULT hr = context->RequestEditSession(client_id, session.Get(),
                                             TF_ES_SYNC | TF_ES_READWRITE, &session_hr);
    if (SUCCEEDED(hr) && session_hr == TF_E_SYNCHRONOUS) {
        hr = context->RequestEditSession(client_id, session.Get(),
                                         TF_ES_ASYNCDONTCARE | TF_ES_READWRITE, &session_hr);
        // Queued (TF_S_ASYNC), or run at once after all (TSF may still grant it synchronously).
        if (async && SUCCEEDED(hr) && session_hr == TF_S_ASYNC) *async = true;
    }
    return FAILED(hr) ? hr : session_hr;
}

HRESULT RunReadSessionNow(ITfContext* context, TfClientId client_id,
                          std::function<HRESULT(TfEditCookie)> body) {
    if (!context) return E_INVALIDARG;
    ComPtr<EditSession> session;
    session.Attach(new (std::nothrow) EditSession(std::move(body)));
    if (!session) return E_OUTOFMEMORY;
    HRESULT session_hr = E_FAIL;
    const HRESULT hr = context->RequestEditSession(client_id, session.Get(), TF_ES_SYNC | TF_ES_READ,
                                                   &session_hr);
    return FAILED(hr) ? hr : session_hr;
}

bool CaretInside(TfEditCookie ec, ITfContext* context, ITfRange* range) {
    TF_SELECTION selection = {};
    ULONG fetched = 0;
    if (FAILED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &fetched)) ||
        !fetched || !selection.range) {
        return false;
    }
    LONG start = 0, end = 0;
    const bool inside =
        SUCCEEDED(selection.range->CompareStart(ec, range, TF_ANCHOR_START, &start)) &&
        SUCCEEDED(selection.range->CompareEnd(ec, range, TF_ANCHOR_END, &end)) && start >= 0 &&
        end <= 0;
    selection.range->Release();
    return inside;
}

HRESULT CaretToEnd(TfEditCookie ec, ITfContext* context, ITfRange* range) {
    ComPtr<ITfRange> caret;
    HRESULT hr = range->Clone(&caret);
    if (FAILED(hr)) return hr;
    caret->Collapse(ec, TF_ANCHOR_END);
    TF_SELECTION selection = {};
    selection.range = caret.Get();
    selection.style.ase = TF_AE_NONE;
    selection.style.fInterimChar = FALSE;
    return context->SetSelection(ec, 1, &selection);
}

HRESULT InsertAtSelection(TfEditCookie ec, ITfContext* context, std::wstring_view text) {
    ComPtr<ITfInsertAtSelection> insert;
    HRESULT hr = context->QueryInterface(IID_PPV_ARGS(&insert));
    if (FAILED(hr)) return hr;
    ComPtr<ITfRange> range;
    hr = insert->InsertTextAtSelection(ec, 0, text.data(), static_cast<LONG>(text.size()), &range);
    if (FAILED(hr)) return hr;
    return CaretToEnd(ec, context, range.Get());
}

}  // namespace bw

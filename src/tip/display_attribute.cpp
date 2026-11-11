// Adapted from team-mirae/jis-moonshot windows/src/display_attribute.*.
#include "display_attribute.h"

#include <new>

#include "dll.h"
#include "guids.h"

namespace bw {
namespace {

struct AttributeSpec {
    const GUID* guid;
    const wchar_t* description;
    TF_DA_LINESTYLE line;
    BOOL bold;
    TF_DA_ATTR_INFO info;
};

// Unconfirmed text: a solid underline, as on macOS. A reading shown in its place (a
// prediction): a thick one. Colors are the app's (TF_CT_NONE).
constexpr AttributeSpec kAttributes[] = {
    {&kDisplayAttributeInput, L"bada-windows unconfirmed text", TF_LS_SOLID, FALSE, TF_ATTR_INPUT},
    {&kDisplayAttributeConverted, L"bada-windows menu entry shown", TF_LS_SOLID, TRUE,
     TF_ATTR_TARGET_CONVERTED},
};
constexpr ULONG kCount = ARRAYSIZE(kAttributes);

class AttributeInfo final : public ITfDisplayAttributeInfo {
public:
    explicit AttributeInfo(const AttributeSpec& spec) : spec_(spec) { DllAddRef(); }

    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        if (riid == IID_IUnknown || riid == IID_ITfDisplayAttributeInfo) {
            *object = static_cast<ITfDisplayAttributeInfo*>(this);
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
    STDMETHODIMP GetGUID(GUID* guid) override {
        if (!guid) return E_INVALIDARG;
        *guid = *spec_.guid;
        return S_OK;
    }
    STDMETHODIMP GetDescription(BSTR* description) override {
        if (!description) return E_INVALIDARG;
        *description = SysAllocString(spec_.description);
        return *description ? S_OK : E_OUTOFMEMORY;
    }
    STDMETHODIMP GetAttributeInfo(TF_DISPLAYATTRIBUTE* attribute) override {
        if (!attribute) return E_INVALIDARG;
        *attribute = {};
        attribute->crText.type = TF_CT_NONE;
        attribute->crBk.type = TF_CT_NONE;
        attribute->lsStyle = spec_.line;
        attribute->fBoldLine = spec_.bold;
        attribute->crLine.type = TF_CT_NONE;
        attribute->bAttr = spec_.info;
        return S_OK;
    }
    STDMETHODIMP SetAttributeInfo(const TF_DISPLAYATTRIBUTE*) override { return E_NOTIMPL; }
    STDMETHODIMP Reset() override { return S_OK; }

private:
    ~AttributeInfo() { DllRelease(); }
    const AttributeSpec& spec_;
    LONG refs_ = 1;
};

class AttributeEnum final : public IEnumTfDisplayAttributeInfo {
public:
    explicit AttributeEnum(ULONG position = 0) : position_(position) { DllAddRef(); }

    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        if (riid == IID_IUnknown || riid == IID_IEnumTfDisplayAttributeInfo) {
            *object = static_cast<IEnumTfDisplayAttributeInfo*>(this);
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
    STDMETHODIMP Clone(IEnumTfDisplayAttributeInfo** clone) override {
        if (!clone) return E_INVALIDARG;
        *clone = new (std::nothrow) AttributeEnum(position_);
        return *clone ? S_OK : E_OUTOFMEMORY;
    }
    STDMETHODIMP Next(ULONG count, ITfDisplayAttributeInfo** infos, ULONG* fetched) override {
        if (!infos) return E_INVALIDARG;
        ULONG n = 0;
        while (n < count && position_ < kCount) {
            infos[n] = new (std::nothrow) AttributeInfo(kAttributes[position_]);
            if (!infos[n]) break;
            ++n;
            ++position_;
        }
        if (fetched) *fetched = n;
        return n == count ? S_OK : S_FALSE;
    }
    STDMETHODIMP Reset() override {
        position_ = 0;
        return S_OK;
    }
    STDMETHODIMP Skip(ULONG count) override {
        position_ = position_ + count > kCount ? kCount : position_ + count;
        return position_ < kCount ? S_OK : S_FALSE;
    }

private:
    ~AttributeEnum() { DllRelease(); }
    LONG refs_ = 1;
    ULONG position_;
};

}  // namespace

IEnumTfDisplayAttributeInfo* NewDisplayAttributeEnum() { return new (std::nothrow) AttributeEnum(); }

ITfDisplayAttributeInfo* NewDisplayAttributeInfo(REFGUID guid) {
    for (const AttributeSpec& spec : kAttributes) {
        if (IsEqualGUID(guid, *spec.guid)) return new (std::nothrow) AttributeInfo(spec);
    }
    return nullptr;
}

}  // namespace bw

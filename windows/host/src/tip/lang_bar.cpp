// Adapted from team-mirae/jis-moonshot windows/src/lang_bar.*.
#include "lang_bar.h"

#include <olectl.h>

#include <algorithm>
#include <memory>
#include <new>
#include <string>
#include <vector>

#include "dll.h"
#include "guids.h"
#include "product.h"

namespace bw {
namespace {

constexpr DWORD kSinkCookie = 1;

// Records what InitMenu adds, to show it as a Win32 popup menu: the Windows 8+ input indicator
// only reports a right-click (OnClick, TF_LBI_CLK_RIGHT) and leaves the menu to the IME
// (measured on Windows 11: InitMenu is never called).
class MenuRecorder final : public ITfMenu {
public:
    struct Item {
        UINT id;
        DWORD flags;
        std::wstring text;
        MenuRecorder* submenu;  // owned, or nullptr
    };

    MenuRecorder() { DllAddRef(); }

    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (!object) return E_INVALIDARG;
        if (riid == IID_IUnknown || riid == IID_ITfMenu) {
            *object = static_cast<ITfMenu*>(this);
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
    STDMETHODIMP AddMenuItem(UINT id, DWORD flags, HBITMAP, HBITMAP, const WCHAR* text, ULONG length,
                             ITfMenu** submenu) override {
        Item item = {id, flags, text ? std::wstring(text, length) : std::wstring(), nullptr};
        if (flags & TF_LBMENUF_SUBMENU) {
            item.submenu = new (std::nothrow) MenuRecorder();
            if (!item.submenu) return E_OUTOFMEMORY;
            if (submenu) {
                *submenu = item.submenu;
                item.submenu->AddRef();
            }
        } else if (submenu) {
            *submenu = nullptr;
        }
        items_.push_back(std::move(item));
        return S_OK;
    }

    // The recorded items as a popup menu (the caller destroys it).
    HMENU Build() const {
        HMENU menu = CreatePopupMenu();
        for (const Item& item : items_) {
            if (item.flags & TF_LBMENUF_SEPARATOR) {
                AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
                continue;
            }
            UINT flags = MF_STRING;
            if (item.flags & TF_LBMENUF_CHECKED) flags |= MF_CHECKED;
            if (item.flags & TF_LBMENUF_GRAYED) flags |= MF_GRAYED;
            if (item.submenu) {
                AppendMenuW(menu, flags | MF_POPUP, reinterpret_cast<UINT_PTR>(item.submenu->Build()),
                            item.text.c_str());
            } else {
                AppendMenuW(menu, flags, item.id, item.text.c_str());
            }
        }
        return menu;
    }

private:
    ~MenuRecorder() {
        for (Item& item : items_) {
            if (item.submenu) item.submenu->Release();
        }
        DllRelease();
    }
    LONG refs_ = 1;
    std::vector<Item> items_;
};

// Shows the menu at the indicator and returns the item picked (0: none). TrackPopupMenu needs an
// owner window on this thread: a hidden one, made foreground so a click elsewhere closes the menu.
UINT ShowMenu(const MenuRecorder& recorder, POINT pt, const RECT* area) {
    HMENU menu = recorder.Build();
    HWND owner = CreateWindowExW(WS_EX_TOOLWINDOW, L"STATIC", L"", WS_POPUP, 0, 0, 0, 0, nullptr,
                                 nullptr, DllInstance(), nullptr);
    UINT id = 0;
    if (menu && owner) {
        SetForegroundWindow(owner);
        // Above the taskbar button, as the system's own menus there open.
        const int y = area ? area->top : pt.y;
        id = static_cast<UINT>(TrackPopupMenuEx(
            menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, y, owner,
            nullptr));
        PostMessageW(owner, WM_NULL, 0, 0);  // lets the menu close properly
    }
    if (owner) DestroyWindow(owner);
    if (menu) DestroyMenu(menu);
    return id;
}

// The taskbar's theme, so the label reads like the Microsoft IME's: dark text on a light taskbar.
bool LightTaskbar() {
    DWORD value = 0;
    DWORD size = sizeof(value);
    return RegGetValueW(HKEY_CURRENT_USER,
                        L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                        L"SystemUsesLightTheme", RRF_RT_REG_DWORD, nullptr, &value,
                        &size) == ERROR_SUCCESS &&
           value != 0;
}

// `text` drawn as a square icon of `size` pixels, as large as fits.
HICON TextIcon(const wchar_t* text, int size) {
    BITMAPINFO bi = {};
    bi.bmiHeader.biSize = sizeof(bi.bmiHeader);
    bi.bmiHeader.biWidth = size;
    bi.bmiHeader.biHeight = -size;  // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;
    void* bits = nullptr;
    HDC screen = GetDC(nullptr);
    HDC dc = CreateCompatibleDC(screen);
    ReleaseDC(nullptr, screen);
    HBITMAP color = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (!dc || !color || !bits) {
        if (color) DeleteObject(color);
        if (dc) DeleteDC(dc);
        return nullptr;
    }
    HGDIOBJ old_bitmap = SelectObject(dc, color);
    auto* pixels = static_cast<DWORD*>(bits);
    std::fill(pixels, pixels + size * size, 0);

    // White on black with grayscale antialiasing: each pixel's brightness is its coverage.
    const int length = static_cast<int>(wcslen(text));
    HFONT font = nullptr;
    for (int height = size; height > 4; --height) {
        if (font) DeleteObject(font);
        font = CreateFontW(-height, 0, 0, 0, FW_SEMIBOLD, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                           OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY,
                           DEFAULT_PITCH, L"Yu Gothic UI");
        SelectObject(dc, font);
        SIZE extent;
        GetTextExtentPoint32W(dc, text, length, &extent);
        if (extent.cx <= size) break;
    }
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    RECT rect = {0, 0, size, size};
    DrawTextW(dc, text, length, &rect, DT_SINGLELINE | DT_CENTER | DT_VCENTER | DT_NOPREFIX);
    GdiFlush();

    const BYTE ink = LightTaskbar() ? 0 : 255;
    for (int i = 0; i < size * size; ++i) {
        const DWORD p = pixels[i];
        const BYTE a = static_cast<BYTE>(std::max({p & 0xFF, (p >> 8) & 0xFF, (p >> 16) & 0xFF}));
        const BYTE c = static_cast<BYTE>(ink * a / 255);  // premultiplied
        pixels[i] = (static_cast<DWORD>(a) << 24) | (c << 16) | (c << 8) | c;
    }
    SelectObject(dc, old_bitmap);
    if (font) DeleteObject(font);
    DeleteDC(dc);

    std::vector<BYTE> zeros(static_cast<size_t>((size + 15) / 16 * 2 * size), 0);
    HBITMAP mask = CreateBitmap(size, size, 1, 1, zeros.data());
    ICONINFO info = {TRUE, 0, 0, mask, color};
    HICON icon = CreateIconIndirect(&info);
    DeleteObject(mask);
    DeleteObject(color);
    return icon;
}

}  // namespace

LangBarButton::LangBarButton(std::function<void(ITfMenu*)> init_menu,
                             std::function<void(UINT)> on_menu, std::function<void()> on_click)
    : init_menu_(std::move(init_menu)),
      on_menu_(std::move(on_menu)),
      on_click_(std::move(on_click)) {
    DllAddRef();
}

LangBarButton::~LangBarButton() {
    if (sink_) sink_->Release();
    DllRelease();
}

void LangBarButton::SetState(const std::wstring& label, const std::wstring& tooltip) {
    if (label == label_ && tooltip == tooltip_) return;
    label_ = label;
    tooltip_ = tooltip;
    if (sink_) sink_->OnUpdate(TF_LBI_ICON | TF_LBI_TOOLTIP | TF_LBI_TEXT);
}

void LangBarButton::Detach() {
    init_menu_ = nullptr;
    on_menu_ = nullptr;
    on_click_ = nullptr;
}

STDMETHODIMP LangBarButton::QueryInterface(REFIID riid, void** object) {
    if (!object) return E_INVALIDARG;
    if (riid == IID_IUnknown || riid == IID_ITfLangBarItem || riid == IID_ITfLangBarItemButton) {
        *object = static_cast<ITfLangBarItemButton*>(this);
    } else if (riid == IID_ITfSource) {
        *object = static_cast<ITfSource*>(this);
    } else {
        *object = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) LangBarButton::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) LangBarButton::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (!refs) delete this;
    return refs;
}

STDMETHODIMP LangBarButton::GetInfo(TF_LANGBARITEMINFO* info) {
    if (!info) return E_INVALIDARG;
    const Product* product = GetProduct();
    info->clsidService = product ? product->clsid : CLSID{};
    // GUID_LBI_INPUTMODE: Windows 8 and later show this button in the taskbar's input indicator.
    info->guidItem = kLangBarItemInputMode;
    info->dwStyle = TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY;
    info->ulSort = 0;
    wcsncpy_s(info->szDescription, product ? product->display_name.c_str() : L"bada-windows", _TRUNCATE);
    return S_OK;
}

STDMETHODIMP LangBarButton::GetStatus(DWORD* status) {
    if (!status) return E_INVALIDARG;
    *status = 0;
    return S_OK;
}

STDMETHODIMP LangBarButton::Show(BOOL) { return E_NOTIMPL; }

STDMETHODIMP LangBarButton::GetTooltipString(BSTR* tooltip) {
    if (!tooltip) return E_INVALIDARG;
    *tooltip = SysAllocString(tooltip_.c_str());
    return *tooltip ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP LangBarButton::OnClick(TfLBIClick click, POINT pt, const RECT* area) {
    if (click == TF_LBI_CLK_LEFT && on_click_) on_click_();
    if (click == TF_LBI_CLK_RIGHT && init_menu_) {
        // Hold a reference: picking an item may change state that releases this button.
        AddRef();
        if (MenuRecorder* recorder = new (std::nothrow) MenuRecorder()) {
            init_menu_(recorder);
            const UINT id = ShowMenu(*recorder, pt, area);
            recorder->Release();
            if (id && on_menu_) on_menu_(id);
        }
        Release();
    }
    return S_OK;
}

STDMETHODIMP LangBarButton::InitMenu(ITfMenu* menu) {
    if (menu && init_menu_) init_menu_(menu);
    return S_OK;
}

STDMETHODIMP LangBarButton::OnMenuSelect(UINT id) {
    if (on_menu_) on_menu_(id);
    return S_OK;
}

STDMETHODIMP LangBarButton::GetIcon(HICON* icon) {
    if (!icon) return E_INVALIDARG;
    *icon = TextIcon(label_.c_str(), GetSystemMetrics(SM_CXSMICON));
    return *icon ? S_OK : E_FAIL;
}

STDMETHODIMP LangBarButton::GetText(BSTR* text) {
    if (!text) return E_INVALIDARG;
    *text = SysAllocString(label_.c_str());
    return *text ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP LangBarButton::AdviseSink(REFIID riid, IUnknown* sink, DWORD* cookie) {
    if (!sink || !cookie) return E_INVALIDARG;
    if (riid != IID_ITfLangBarItemSink) return CONNECT_E_CANNOTCONNECT;
    if (sink_) return CONNECT_E_ADVISELIMIT;
    if (FAILED(sink->QueryInterface(IID_PPV_ARGS(&sink_)))) {
        sink_ = nullptr;
        return E_NOINTERFACE;
    }
    *cookie = kSinkCookie;
    return S_OK;
}

STDMETHODIMP LangBarButton::UnadviseSink(DWORD cookie) {
    if (cookie != kSinkCookie || !sink_) return CONNECT_E_NOCONNECTION;
    sink_->Release();
    sink_ = nullptr;
    return S_OK;
}

}  // namespace bw

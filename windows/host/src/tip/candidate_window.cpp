// Adapted from team-mirae/jis-moonshot windows/src/candidate_window.*.
#include "candidate_window.h"

#include <algorithm>

#include "dll.h"
#include "product.h"

namespace bw {
namespace {

constexpr wchar_t kClassName[] = L"BadaWindowsMenu";

int Scale(int value, UINT dpi) { return MulDiv(value, static_cast<int>(dpi), 96); }

}  // namespace

CandidateWindow::~CandidateWindow() {
    if (hwnd_) DestroyWindow(hwnd_);
    if (font_) DeleteObject(font_);
    if (japanese_font_) DeleteObject(japanese_font_);
}

void CandidateWindow::UpdateFont() {
    const UINT dpi = hwnd_ ? GetDpiForWindow(hwnd_) : 96;
    if (font_ && dpi == font_dpi_) return;
    if (font_) DeleteObject(font_);
    if (japanese_font_) DeleteObject(japanese_font_);
    NONCLIENTMETRICSW metrics = {sizeof(metrics)};
    SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0, dpi);
    LOGFONTW lf = metrics.lfMessageFont;
    lf.lfHeight = -MulDiv(11, static_cast<int>(dpi), 72);  // 11 pt
    // The system UI font (Segoe UI, with font linking for Korean and CJK); Japanese rows in Yu Gothic
    // UI, so kanji take their Japanese forms (bada-compatible §6 `japanese`).
    font_ = CreateFontIndirectW(&lf);
    lf.lfCharSet = SHIFTJIS_CHARSET;
    wcscpy_s(lf.lfFaceName, L"Yu Gothic UI");
    japanese_font_ = CreateFontIndirectW(&lf);
    font_dpi_ = dpi;
}

bool CandidateWindow::Visible() const { return hwnd_ && IsWindowVisible(hwnd_); }

void CandidateWindow::Show(const Output& out, const RECT& anchor) {
    if (out.menu.empty()) {
        Hide();
        return;
    }
    if (!hwnd_) {
        WNDCLASSEXW wc = {sizeof(wc)};
        wc.style = CS_DROPSHADOW;
        wc.lpfnWndProc = Proc;
        wc.hInstance = DllInstance();
        wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        wc.lpszClassName = kClassName;
        RegisterClassExW(&wc);  // fails harmlessly when already registered
        hwnd_ = CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, kClassName, L"",
                                WS_POPUP | WS_BORDER, 0, 0, 1, 1, nullptr, nullptr, DllInstance(), nullptr);
        if (!hwnd_) return;
        SetWindowLongPtrW(hwnd_, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(this));
        // Never in screen recordings, screenshots or screen sharing: the menu shows what's being
        // typed (jis-moonshot: a certificate password showed in its list).
        SetWindowDisplayAffinity(hwnd_, WDA_EXCLUDEFROMCAPTURE);
    }
    rows_.clear();
    for (size_t i = 0; i < out.menu.size(); ++i) {
        rows_.push_back({Wide(out.menu[i]), i < out.notes.size() ? Wide(out.notes[i]) : std::wstring(),
                         i < out.japanese.size() && out.japanese[i]});
    }
    selected_ = out.selected < rows_.size() ? static_cast<int>(out.selected) : -1;
    zero_ = out.zero < rows_.size() ? out.zero : 0;
    horizontal_ = out.horizontal;

    UpdateFont();
    const UINT dpi = font_dpi_;
    HDC dc = GetDC(hwnd_);
    HGDIOBJ old = SelectObject(dc, font_);
    TEXTMETRICW tm;
    GetTextMetricsW(dc, &tm);
    padding_ = Scale(6, dpi);
    row_height_ = tm.tmHeight + Scale(4, dpi);
    SIZE size;
    text_width_ = 0;
    widths_.clear();
    int width = 0;
    for (const Row& r : rows_) {
        SelectObject(dc, FontFor(r));
        GetTextExtentPoint32W(dc, r.text.c_str(), static_cast<int>(r.text.size()), &size);
        text_width_ = std::max(text_width_, static_cast<int>(size.cx));
        widths_.push_back(static_cast<int>(size.cx) + 2 * padding_);
    }
    SelectObject(dc, font_);
    if (horizontal_) {
        for (int w : widths_) width += w;
    } else {
        for (const Row& r : rows_) {
            int w = text_width_;
            if (!r.note.empty()) {
                GetTextExtentPoint32W(dc, r.note.c_str(), static_cast<int>(r.note.size()), &size);
                w += Scale(12, dpi) + size.cx;
            }
            width = std::max(width, w);
        }
        width += padding_ * 2;
    }
    SelectObject(dc, old);
    ReleaseDC(hwnd_, dc);
    width += Scale(2, dpi);
    const int lines = horizontal_ ? 1 : static_cast<int>(rows_.size());
    const int height = padding_ * 2 + row_height_ * lines + Scale(2, dpi);

    MONITORINFO mi = {sizeof(mi)};
    GetMonitorInfoW(MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST), &mi);
    const int x = std::clamp(static_cast<int>(anchor.left), static_cast<int>(mi.rcWork.left),
                             std::max(static_cast<int>(mi.rcWork.left), static_cast<int>(mi.rcWork.right) - width));
    int y = anchor.bottom + Scale(2, dpi);
    if (y + height > mi.rcWork.bottom) y = anchor.top - height - Scale(2, dpi);
    SetWindowPos(hwnd_, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    InvalidateRect(hwnd_, nullptr, TRUE);
}

void CandidateWindow::Unregister() { UnregisterClassW(kClassName, DllInstance()); }

void CandidateWindow::Hide() {
    if (hwnd_) ShowWindow(hwnd_, SW_HIDE);
}

void CandidateWindow::Paint(HDC dc) {
    RECT client;
    GetClientRect(hwnd_, &client);
    FillRect(dc, &client, GetSysColorBrush(COLOR_WINDOW));
    HGDIOBJ old = SelectObject(dc, font_);
    SetBkMode(dc, TRANSPARENT);
    const int flags = DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX;
    int x = client.left + 1;
    int y = padding_;
    for (size_t i = 0; i < rows_.size(); ++i) {
        const Row& r = rows_[i];
        const bool focused = static_cast<int>(i) == selected_;
        RECT cell = horizontal_ ? RECT{x, y, x + widths_[i], y + row_height_}
                                : RECT{client.left + padding_ / 2, y, client.right - padding_ / 2, y + row_height_};
        if (focused) FillRect(dc, &cell, GetSysColorBrush(COLOR_HIGHLIGHT));
        // Negative entries (above entry 0) are set off from the ranked ones by a line.
        if (!horizontal_ && zero_ > 0 && i == zero_) {
            RECT line = {client.left + padding_, y - 1, client.right - padding_, y};
            FillRect(dc, &line, GetSysColorBrush(COLOR_GRAYTEXT));
        }
        SetTextColor(dc, GetSysColor(focused ? COLOR_HIGHLIGHTTEXT : COLOR_WINDOWTEXT));
        SelectObject(dc, FontFor(r));
        RECT text = {cell.left + padding_ / 2 + (horizontal_ ? padding_ / 2 : 0), y, cell.right, y + row_height_};
        if (!horizontal_) text.left = client.left + padding_;
        DrawTextW(dc, r.text.c_str(), -1, &text, flags | (horizontal_ ? DT_CENTER : DT_LEFT));
        SelectObject(dc, font_);
        if (!horizontal_ && !r.note.empty()) {
            SetTextColor(dc, GetSysColor(focused ? COLOR_HIGHLIGHTTEXT : COLOR_GRAYTEXT));
            RECT note = {client.left + padding_ + text_width_ + MulDiv(12, font_dpi_, 96), y, client.right - padding_,
                         y + row_height_};
            DrawTextW(dc, r.note.c_str(), -1, &note, flags | DT_LEFT);
        }
        if (horizontal_) {
            x += widths_[i];
        } else {
            y += row_height_;
        }
    }
    SelectObject(dc, old);
}

LRESULT CALLBACK CandidateWindow::Proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    auto* self = reinterpret_cast<CandidateWindow*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    switch (message) {
        case WM_MOUSEACTIVATE:
            return MA_NOACTIVATE;  // clicks never take focus from the app (the menu has no picking)
        case WM_PAINT: {
            PAINTSTRUCT ps;
            HDC dc = BeginPaint(hwnd, &ps);
            if (self) self->Paint(dc);
            EndPaint(hwnd, &ps);
            return 0;
        }
        case WM_ERASEBKGND:
            return 1;
    }
    return DefWindowProcW(hwnd, message, wparam, lparam);
}

}  // namespace bw

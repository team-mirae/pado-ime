// The engine's menu (bada-compatible §6: menu, selected, zero, horizontal, notes, japanese), drawn
// under the text. A topmost popup that never takes focus, excluded from screen capture.
// Adapted from team-mirae/jis-moonshot windows/src/candidate_window.*.
#pragma once

#include <windows.h>

#include <string>
#include <vector>

#include "output.h"

namespace bw {

class CandidateWindow {
public:
    CandidateWindow() = default;
    ~CandidateWindow();
    CandidateWindow(const CandidateWindow&) = delete;
    CandidateWindow& operator=(const CandidateWindow&) = delete;

    // Shows `out`'s menu below `anchor` (screen coordinates of the text), or above it when there's
    // no room below. Rows above `zero` (negative entries) are set off by a line.
    void Show(const Output& out, const RECT& anchor);
    void Hide();
    bool Visible() const;
    static void Unregister();  // DLL unload; fails harmlessly while a window exists

private:
    struct Row {
        std::wstring text;
        std::wstring note;
        bool japanese = false;
    };
    static LRESULT CALLBACK Proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam);
    void Paint(HDC dc);
    void UpdateFont();
    HFONT FontFor(const Row& row) const { return row.japanese && japanese_font_ ? japanese_font_ : font_; }

    HWND hwnd_ = nullptr;
    HFONT font_ = nullptr;
    HFONT japanese_font_ = nullptr;
    UINT font_dpi_ = 0;
    std::vector<Row> rows_;
    std::vector<int> widths_;  // horizontal: each cell's width
    int selected_ = -1;
    size_t zero_ = 0;
    bool horizontal_ = false;
    int row_height_ = 0;
    int text_width_ = 0;
    int padding_ = 0;
};

}  // namespace bw

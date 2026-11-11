#include "keys.h"

#include <cstring>

namespace bw {
namespace {

// Scan code set 1 → KeyboardEvent.code, for keys that aren't extended: the JIS kana space and the
// few others that type. nullptr: not a character key.
struct ScanTable {
    const char* codes[0x80] = {};
    constexpr ScanTable() {
        const char* digits[] = {"Digit1", "Digit2", "Digit3", "Digit4", "Digit5",
                                "Digit6", "Digit7", "Digit8", "Digit9", "Digit0"};
        for (int i = 0; i < 10; ++i) codes[0x02 + i] = digits[i];
        codes[0x0C] = "Minus";
        codes[0x0D] = "Equal";  // JIS ^
        codes[0x7D] = "IntlYen";
        const char* q_row[] = {"KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY",
                               "KeyU", "KeyI", "KeyO", "KeyP", "BracketLeft", "BracketRight"};
        for (int i = 0; i < 12; ++i) codes[0x10 + i] = q_row[i];
        const char* a_row[] = {"KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH",
                               "KeyJ", "KeyK", "KeyL", "Semicolon", "Quote"};
        for (int i = 0; i < 11; ++i) codes[0x1E + i] = a_row[i];
        codes[0x29] = "Backquote";  // 半角/全角 on JIS
        codes[0x2B] = "Backslash";  // JIS ]
        const char* z_row[] = {"KeyZ", "KeyX", "KeyC", "KeyV", "KeyB",
                               "KeyN", "KeyM", "Comma", "Period", "Slash"};
        for (int i = 0; i < 10; ++i) codes[0x2C + i] = z_row[i];
        codes[0x73] = "IntlRo";         // ろ
        codes[0x56] = "IntlBackslash";  // ISO only
    }
};
constexpr ScanTable kScan;

// JIS legends (macOS ABC on a JIS keyboard): code, unshifted, shifted. Letters are separate.
struct Legend {
    const char* code;
    const char* plain;
    const char* shifted;
};
constexpr Legend kJis[] = {
    {"Digit1", "1", "!"},  {"Digit2", "2", "\""}, {"Digit3", "3", "#"},  {"Digit4", "4", "$"},
    {"Digit5", "5", "%"},  {"Digit6", "6", "&"},  {"Digit7", "7", "'"},  {"Digit8", "8", "("},
    {"Digit9", "9", ")"},  {"Digit0", "0", "0"},  {"Minus", "-", "="},   {"Equal", "^", "~"},
    {"IntlYen", "\xC2\xA5", "|"},  // ¥
    {"BracketLeft", "@", "`"},     {"BracketRight", "[", "{"}, {"Semicolon", ";", "+"},
    {"Quote", ":", "*"},           {"Backslash", "]", "}"},    {"Comma", ",", "<"},
    {"Period", ".", ">"},          {"Slash", "/", "?"},        {"IntlRo", "\\", "_"},
};

bool IsModifier(unsigned v) {
    switch (v) {
        case vk::kShift: case vk::kLShift: case vk::kRShift:
        case vk::kControl: case vk::kLControl: case vk::kRControl:
        case vk::kMenu: case vk::kLMenu: case vk::kRMenu:
        case vk::kLWin: case vk::kRWin:
        case vk::kCapital: case vk::kNumLock: case vk::kScroll:
        case vk::kProcessKey:
            return true;
    }
    return false;
}

const char* NamedByVk(unsigned v) {
    switch (v) {
        case vk::kBack: return "Backspace";
        case vk::kTab: return "Tab";
        case vk::kReturn: return "Enter";  // the keypad's Enter too, as on macOS
        case vk::kEscape: return "Escape";
        case vk::kSpace: return "Space";
        case vk::kLeft: return "ArrowLeft";
        case vk::kUp: return "ArrowUp";
        case vk::kRight: return "ArrowRight";
        case vk::kDown: return "ArrowDown";
        case vk::kHome: return "Home";
        case vk::kEnd: return "End";
        case vk::kPrior: return "PageUp";
        case vk::kNext: return "PageDown";
        case vk::kInsert: return "Insert";
        case vk::kDelete: return "Delete";
        case vk::kPause: return "Pause";
        case vk::kApps: return "ContextMenu";
        case vk::kMultiply: return "NumpadMultiply";
        case vk::kAdd: return "NumpadAdd";
        case vk::kSeparator: return "NumpadComma";
        case vk::kSubtract: return "NumpadSubtract";
        case vk::kDecimal: return "NumpadDecimal";
        case vk::kDivide: return "NumpadDivide";
    }
    return nullptr;
}

}  // namespace

LangKey LangKeyOf(const KeyDown& key) {
    if (key.vk == vk::kPacket) return LangKey::kNone;  // a character injected as Unicode
    if (!key.extended && key.scan) {
        switch (key.scan) {
            case 0x7B: return LangKey::kMuhenkan;
            case 0x79: return LangKey::kHenkan;
            case 0x70: return LangKey::kKatakana;
            // An Apple JIS keyboard's 英数 and かな (HID LANG2 / LANG1). Windows' HID mapping gives
            // them 0xF1 / 0xF2; some keyboards send 0x71 / 0x72 (the Korean Hanja / Hangul codes).
            // Not measured yet (docs/spec.md §9).
            case 0xF1: case 0x71: return LangKey::kEisu;
            case 0xF2: case 0x72: return LangKey::kKana;
            case 0x3A: return LangKey::kCapsLock;
        }
        // A scan code of another key: by its virtual key only if it has none of its own.
        if (kScan.codes[key.scan & 0x7F] && key.scan < 0x80) return LangKey::kNone;
    }
    switch (key.vk) {
        case vk::kNonConvert: return LangKey::kMuhenkan;
        case vk::kConvert: return LangKey::kHenkan;
        case vk::kDbeKatakana: case vk::kDbeHiragana: return LangKey::kKatakana;
        case vk::kDbeAlphanumeric: return LangKey::kCapsLock;  // the JIS layout's unshifted Caps Lock key
        case vk::kCapital: return LangKey::kCapsLock;
    }
    return LangKey::kNone;
}

std::string CodeOf(const KeyDown& key) {
    if (key.vk == vk::kPacket) return "Unidentified";  // commits first, then reaches the app (#4)
    if (IsModifier(key.vk)) return "";
    if (key.vk >= vk::kF1 && key.vk <= vk::kF24) return "F" + std::to_string(key.vk - vk::kF1 + 1);
    if (key.vk >= vk::kNumpad0 && key.vk <= vk::kNumpad0 + 9) {
        return "Numpad" + std::to_string(key.vk - vk::kNumpad0);
    }
    if (const char* named = NamedByVk(key.vk)) return named;
    // 半角/全角 under the Japanese layout is VK_OEM_AUTO / VK_OEM_ENLW, ⌥ + it VK_KANJI (FKeyIME
    // docs/issues/modifier-taps-unreachable.md): the Backquote position, whatever the virtual key.
    if (!key.extended && key.scan < 0x80 && kScan.codes[key.scan]) return kScan.codes[key.scan];
    if (key.vk == vk::kOemAuto || key.vk == vk::kOemEnlw || key.vk == vk::kKanji) return "Backquote";
    return "Unidentified";
}

std::string JisCharacter(const std::string& code, bool shift, bool caps) {
    if (code.size() == 4 && code.compare(0, 3, "Key") == 0 && code[3] >= 'A' && code[3] <= 'Z') {
        const bool upper = shift || caps;  // bada-compatible §7.3: Caps Lock types capitals, ⇧ or not
        return std::string(1, upper ? code[3] : static_cast<char>(code[3] - 'A' + 'a'));
    }
    for (const Legend& l : kJis) {
        if (code == l.code) return shift ? l.shifted : l.plain;
    }
    return {};
}

unsigned ScanOrMapped(unsigned v, unsigned scan, unsigned (*scan_of)(unsigned)) {
    if (scan == 0 && v != vk::kPacket && scan_of) return scan_of(v) & 0xFF;
    return scan;
}

bool SameKeyDown(std::uintptr_t wparam, std::intptr_t lparam, std::int32_t time, std::uint64_t tick,
                 std::uintptr_t wparam2, std::intptr_t lparam2, std::int32_t time2, std::uint64_t tick2) {
    if (wparam != wparam2 || lparam != lparam2) return false;
    if (time == time2) return true;
    // A missed key-up (focus moved to another thread in between) must not swallow a later press.
    constexpr std::uint64_t kRetest = 500;
    const bool first_press = (lparam & (std::intptr_t{1} << 30)) == 0;
    return first_press && tick2 >= tick && tick2 - tick < kRetest;
}

}  // namespace bw

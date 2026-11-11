// Windows key messages → bada-compatible key events (docs/spec.md §3). Pure: no Windows headers, so
// it's unit-tested on any OS.
//
// Character keys go by scan code (the physical position, which is what KeyboardEvent.code names),
// never by the virtual key, which depends on the keyboard layout Windows has loaded under the
// profile's language. Keys whose position doesn't matter (Space, Enter, arrows, F keys, editing
// keys) go by virtual key, so injected input without scan codes still works.
#pragma once

#include <cstdint>
#include <string>

namespace bw {

// Virtual keys used here, as in winuser.h.
namespace vk {
inline constexpr unsigned kBack = 0x08, kTab = 0x09, kReturn = 0x0D, kShift = 0x10, kControl = 0x11,
                          kMenu = 0x12, kPause = 0x13, kCapital = 0x14, kKana = 0x15, kJunja = 0x17,
                          kFinal = 0x18, kKanji = 0x19, kEscape = 0x1B, kConvert = 0x1C,
                          kNonConvert = 0x1D, kSpace = 0x20, kPrior = 0x21, kNext = 0x22, kEnd = 0x23,
                          kHome = 0x24, kLeft = 0x25, kUp = 0x26, kRight = 0x27, kDown = 0x28,
                          kInsert = 0x2D, kDelete = 0x2E, kLWin = 0x5B, kRWin = 0x5C, kApps = 0x5D,
                          kNumpad0 = 0x60, kMultiply = 0x6A, kAdd = 0x6B, kSeparator = 0x6C,
                          kSubtract = 0x6D, kDecimal = 0x6E, kDivide = 0x6F, kF1 = 0x70, kF24 = 0x87,
                          kNumLock = 0x90, kScroll = 0x91, kLShift = 0xA0, kRShift = 0xA1,
                          kLControl = 0xA2, kRControl = 0xA3, kLMenu = 0xA4, kRMenu = 0xA5,
                          kProcessKey = 0xE5, kPacket = 0xE7,
                          // The Japanese layout's (kbd106) language keys.
                          kDbeAlphanumeric = 0xF0, kDbeKatakana = 0xF1, kDbeHiragana = 0xF2,
                          kOemAuto = 0xF3, kOemEnlw = 0xF4;
}  // namespace vk

struct Mods {
    bool shift = false;
    bool ctrl = false;
    bool alt = false;
    bool meta = false;  // the Windows key
    bool Plain() const { return !ctrl && !alt && !meta; }
};

// A key-down as the text service sees it.
struct KeyDown {
    unsigned vk = 0;
    unsigned scan = 0;      // lParam bits 16-23 (or MapVirtualKey's, for injected keys without one)
    bool extended = false;  // lParam bit 24
    bool repeat = false;    // lParam bit 30: an auto-repeat
    Mods mods;
    bool caps = false;      // Caps Lock is on (GetKeyState(VK_CAPITAL) & 1)
};

// The keys the host handles itself (bada-compatible §7.4, adapted for Windows: docs/spec.md §4).
// They never reach the engine as key events, except as "Lang1" (next entry).
enum class LangKey {
    kNone,
    kMuhenkan,  // 無変換: muhenkan mode
    kHenkan,    // 変換: henkan mode, profile "henkan"
    kKatakana,  // カタカナ/ひらがな: henkan mode, profile "katakana"
    kEisu,      // 英数 (an Apple JIS keyboard's, or the JIS layout's unshifted Caps Lock key): muhenkan
    kKana,      // かな (an Apple JIS keyboard's): the mac-kana profile, if bound (docs/spec.md §4.2)
    kCapsLock,  // the Caps Lock position (docs/spec.md §5)
};

// The language key a key-down is, if any. Scan codes first; virtual keys for injected input.
LangKey LangKeyOf(const KeyDown& key);

// The KeyboardEvent.code the engine gets for a key-down; "" for a modifier on its own (Shift, Ctrl,
// Alt, Win, Caps Lock, Num Lock...), which never goes to the engine: it belongs to the next key and
// must never commit (FKeyIME docs/issues/shift-splits-the-composition.md). A key without a name is
// "Unidentified": the engine commits what's staged and passes it on.
std::string CodeOf(const KeyDown& key);

// The character a plain JIS text field types for `code` (macOS ABC on a JIS keyboard, as
// bada-compatible's host and superkeyboard do), as UTF-8; "" when the key types none. With Caps Lock
// on, letters are capitals, ⇧ or not (bada-compatible v1.1 §7.3, as macOS types them).
std::string JisCharacter(const std::string& code, bool shift, bool caps);

// The scan code for `vk` when an injected key-down came without one (#9 in jis-moonshot's notes):
// MapVirtualKeyW on Windows, a stub in tests.
unsigned ScanOrMapped(unsigned vk, unsigned scan, unsigned (*scan_of)(unsigned vk));

// Whether a key-down test (wparam, lParam, message time, tick) is the same key-down as an earlier
// one, tested again (jis-moonshot #5): a first press can't come twice without a key-up, so the same
// lParam within a short while is the same press; auto-repeats share their lParam, so those must have
// the same message time too.
bool SameKeyDown(std::uintptr_t wparam, std::intptr_t lparam, std::int32_t time, std::uint64_t tick,
                 std::uintptr_t wparam2, std::intptr_t lparam2, std::int32_t time2, std::uint64_t tick2);

}  // namespace bw

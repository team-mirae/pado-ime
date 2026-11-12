// Core tests: JSON, Outputs, the key map, manifests, and the host logic typed through a plain text
// field into a real bada-compatible engine (shout, from team-mirae/bada-compatible). Portable: they
// run on Windows (tools/build.ps1) and on macOS / Linux (tools/test-core.sh).
//
//   bw_core_tests [path/to/shout library]   (default: $SHOUT_LIB; without one, engine tests skip)
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <filesystem>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#include "engine.h"
#include "host.h"
#include "json.h"
#include "keys.h"
#include "manifest.h"
#include "output.h"
#include "state_table.h"

namespace fs = std::filesystem;
using namespace bw;

namespace {

int failures = 0;
int checks = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        ++checks;                                                            \
        if (!(cond)) {                                                       \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);      \
            ++failures;                                                      \
        }                                                                    \
    } while (0)

#define CHECK_EQ(a, b)                                                                     \
    do {                                                                                   \
        ++checks;                                                                          \
        const auto& x_ = (a);                                                              \
        const auto& y_ = (b);                                                              \
        if (!(x_ == y_)) {                                                                 \
            std::ostringstream s_;                                                         \
            s_ << x_ << " != " << y_;                                                      \
            std::printf("FAIL %s:%d: %s == %s (%s)\n", __FILE__, __LINE__, #a, #b,         \
                        s_.str().c_str());                                                 \
            ++failures;                                                                    \
        }                                                                                  \
    } while (0)

// --- a text field (bada-compatible §7.1, §7.3), in code points

std::u32string U32(const std::string& s) {
    std::u32string out;
    for (size_t i = 0; i < s.size();) {
        const unsigned char c = static_cast<unsigned char>(s[i]);
        int n = c < 0x80 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4;
        char32_t cp = n == 1 ? c : n == 2 ? (c & 0x1F) : n == 3 ? (c & 0x0F) : (c & 0x07);
        for (int k = 1; k < n && i + k < s.size(); ++k) cp = (cp << 6) | (s[i + k] & 0x3F);
        out += cp;
        i += n;
    }
    return out;
}

std::string U8(const std::u32string& s) {
    std::string out;
    for (char32_t cp : s) {
        if (cp < 0x80) {
            out += static_cast<char>(cp);
        } else if (cp < 0x800) {
            out += static_cast<char>(0xC0 | (cp >> 6));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        } else if (cp < 0x10000) {
            out += static_cast<char>(0xE0 | (cp >> 12));
            out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        } else {
            out += static_cast<char>(0xF0 | (cp >> 18));
            out += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
            out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        }
    }
    return out;
}

class TextField final : public Field {
public:
    void Apply(const Output& out) override {
        if (!out.commit.empty()) {
            size_t s = sel_start_, e = sel_end_;
            if (marked_len_ && !out.replace_selection) {
                s = marked_start_;
                e = marked_start_ + marked_len_;
            }
            Replace(s, e, U32(out.commit));
            marked_len_ = 0;
        }
        if (!out.preedit.empty()) {
            size_t s = sel_start_, e = sel_end_;
            if (marked_len_) {
                s = marked_start_;
                e = marked_start_ + marked_len_;
            }
            const std::u32string p = U32(out.preedit);
            Replace(s, e, p);
            marked_start_ = s;
            marked_len_ = p.size();
        } else if (marked_len_) {
            Replace(marked_start_, marked_start_ + marked_len_, U"");
            marked_len_ = 0;
        }
        menu = out.menu;
    }
    std::string Selection() override { return U8(text_.substr(sel_start_, sel_end_ - sel_start_)); }
    void Type(const std::string& text) override {
        Replace(sel_start_, sel_end_, U32(text));
        ++typed;
    }

    // What the app does with a key the IME didn't eat (plain field, §7.3).
    void App(const std::string& code, const KeyDown& key) {
        if (!key.mods.Plain()) return;
        if (code == "Space") Replace(sel_start_, sel_end_, U" ");
        else if (code == "Enter") Replace(sel_start_, sel_end_, U"\n");
        else if (code == "Tab") Replace(sel_start_, sel_end_, U"\t");
        else if (code == "Backspace" && sel_start_ != sel_end_) Replace(sel_start_, sel_end_, U"");
        else if (code == "Backspace" && sel_start_ > 0) Replace(sel_start_ - 1, sel_start_, U"");
        else if (const std::string c = JisCharacter(code, key.mods.shift, key.caps); !c.empty()) {
            Replace(sel_start_, sel_end_, U32(c));  // Windows' layout, taken as JIS here
            ++app_typed;
        }
    }

    void Select(size_t s, size_t e) {
        sel_start_ = s;
        sel_end_ = e;
    }
    std::string text() const { return U8(text_); }
    std::string preedit() const { return U8(text_.substr(marked_start_, marked_len_)); }
    std::vector<std::string> menu;
    int typed = 0;
    int app_typed = 0;

private:
    void Replace(size_t s, size_t e, const std::u32string& with) {
        text_.replace(s, e - s, with);
        sel_start_ = sel_end_ = s + with.size();
    }
    std::u32string text_;
    size_t sel_start_ = 0, sel_end_ = 0, marked_start_ = 0, marked_len_ = 0;
};

// Key tokens: a KeyboardEvent.code (S-, C-, A- prefixes), or a language key by name.
struct Named {
    unsigned vk;
    unsigned scan;
    bool extended;
};
const std::map<std::string, Named>& Keys() {
    static const std::map<std::string, Named> keys = [] {
        std::map<std::string, Named> k;
        const char* q = "QWERTYUIOP";
        for (int i = 0; i < 10; ++i) k[std::string("Key") + q[i]] = {static_cast<unsigned>(q[i]), 0x10u + i, false};
        const char* a = "ASDFGHJKL";
        for (int i = 0; i < 9; ++i) k[std::string("Key") + a[i]] = {static_cast<unsigned>(a[i]), 0x1Eu + i, false};
        const char* z = "ZXCVBNM";
        for (int i = 0; i < 7; ++i) k[std::string("Key") + z[i]] = {static_cast<unsigned>(z[i]), 0x2Cu + i, false};
        for (int i = 0; i < 10; ++i) k["Digit" + std::to_string((i + 1) % 10)] = {0x31u + i, 0x02u + i, false};
        k["Digit0"] = {0x30, 0x0B, false};
        k["Period"] = {0xBE, 0x34, false};
        k["Comma"] = {0xBC, 0x33, false};
        k["BracketLeft"] = {0xC0, 0x1A, false};  // JIS @, VK_OEM_3 under kbd106
        k["Space"] = {0x20, 0x39, false};
        k["Enter"] = {0x0D, 0x1C, false};
        k["Tab"] = {0x09, 0x0F, false};
        k["Backspace"] = {0x08, 0x0E, false};
        k["Escape"] = {0x1B, 0x01, false};
        k["ArrowUp"] = {0x26, 0x48, true};
        k["ArrowDown"] = {0x28, 0x50, true};
        k["zenkaku"] = {vk::kOemAuto, 0x29, false};
        k["muhenkan"] = {vk::kNonConvert, 0x7B, false};
        k["henkan"] = {vk::kConvert, 0x79, false};
        k["katakana"] = {vk::kDbeHiragana, 0x70, false};
        k["eisu"] = {0, 0xF1, false};
        k["kana"] = {0, 0xF2, false};
        k["caps"] = {vk::kCapital, 0x3A, false};          // Caps Lock under a US layout, or ⇧ on JIS
        k["eisu-caps"] = {vk::kDbeAlphanumeric, 0x3A, false};  // the JIS layout's unshifted key
        k["Shift"] = {vk::kLShift, 0x2A, false};
        return k;
    }();
    return keys;
}

struct Typist {
    Host& host;
    TextField& field;
    bool caps = false;  // Caps Lock, as Windows holds it

    void Type(const std::string& tokens) {
        std::istringstream in(tokens);
        std::string token;
        while (in >> token) Press(token);
    }

    bool Press(std::string token) {
        KeyDown k;
        while (token.size() > 2 && token[1] == '-') {
            if (token[0] == 'S') k.mods.shift = true;
            if (token[0] == 'C') k.mods.ctrl = true;
            if (token[0] == 'A') k.mods.alt = true;
            if (token[0] == 'M') k.mods.meta = true;
            token = token.substr(2);
        }
        const auto it = Keys().find(token);
        if (it == Keys().end()) {
            std::printf("FAIL unknown test key %s\n", token.c_str());
            ++failures;
            return false;
        }
        k.vk = it->second.vk;
        k.scan = it->second.scan;
        k.extended = it->second.extended;
        // Windows toggles the lock on VK_CAPITAL before the IME sees the key.
        if (k.vk == vk::kCapital) caps = !caps;
        k.caps = caps;
        const bool eaten = host.Key(k, field);
        if (const auto wanted = host.TakeCapsWanted()) caps = *wanted;
        if (!eaten) field.App(CodeOf(k), k);
        return eaten;
    }
};

// --- tests

void TestJson() {
    auto v = json::Parse(R"({"a": [1, 2.5, true, null, "x\u00e9\ud83d\ude00\n"], "b": {"c": -3e2}})");
    CHECK(v.has_value());
    CHECK_EQ(v->Get("a")->array.size(), size_t{5});
    CHECK_EQ(*v->Get("a")->array[4].String(), std::string("x\xC3\xA9\xF0\x9F\x98\x80\n"));
    CHECK_EQ(v->Get("b")->GetNumber("c"), -300.0);
    CHECK_EQ(json::Write(*v), std::string(R"({"a":[1,2.5,true,null,"x)") + "\xC3\xA9\xF0\x9F\x98\x80" +
                                  R"(\n"],"b":{"c":-300}})");
    for (const char* bad : {"", "{", "[1,]", "{\"a\" 1}", "nul", "\"\\x\"", "{} x", "[\"\x01\"]"}) {
        CHECK(!json::Parse(bad).has_value());
    }
    std::string deep(100, '[');
    CHECK(!json::Parse(deep + std::string(100, ']')).has_value());
    CHECK_EQ(json::Quote("a\"b\\\x01"), std::string("\"a\\\"b\\\\\\u0001\""));
    const auto base = json::Parse(R"({"forms": ["upper"], "keep": 1})");
    const auto over = json::Parse(R"({"forms": ["lower"]})");
    CHECK_EQ(json::Write(json::MergeTop(&*base, &*over)), std::string(R"({"forms":["lower"],"keep":1})"));
}

void TestOutput() {
    const Output o = ParseOutput(R"({"commit":"A","replace_selection":true,"preedit":"B","menu":["B","b"],
        "selected":1,"zero":1,"consumed":true,"horizontal":true,"notes":["n",""],"japanese":[false,true],
        "need_selection":"shout","future":{"x":1}})");
    CHECK_EQ(o.commit, std::string("A"));
    CHECK(o.replace_selection && o.consumed && o.horizontal);
    CHECK_EQ(o.menu.size(), size_t{2});
    CHECK_EQ(o.selected, size_t{1});
    CHECK_EQ(o.zero, size_t{1});
    CHECK(o.japanese.size() == 2 && o.japanese[1]);
    CHECK(o.need_selection && *o.need_selection == "shout");
    const Output bad = ParseOutput("not json");
    CHECK(!bad.consumed && bad.commit.empty() && !bad.need_selection);
    CHECK(!ParseOutput(R"({"need_selection":null,"selected":-4})").need_selection);
    CHECK_EQ(ParseOutput(R"({"selected":-4})").selected, size_t{0});
}

void TestKeys() {
    auto code = [](unsigned v, unsigned scan, bool ext = false) {
        KeyDown k;
        k.vk = v;
        k.scan = scan;
        k.extended = ext;
        return CodeOf(k);
    };
    auto lang = [](unsigned v, unsigned scan, bool ext = false) {
        KeyDown k;
        k.vk = v;
        k.scan = scan;
        k.extended = ext;
        return LangKeyOf(k);
    };
    CHECK_EQ(code('A', 0x1E), std::string("KeyA"));
    CHECK_EQ(code(0xC0, 0x1A), std::string("BracketLeft"));  // whatever the layout's VK
    CHECK_EQ(code(0xE2, 0x73), std::string("IntlRo"));
    CHECK_EQ(code(0xDC, 0x7D), std::string("IntlYen"));
    CHECK_EQ(code(vk::kOemAuto, 0x29), std::string("Backquote"));  // 半角/全角
    CHECK_EQ(code(vk::kKanji, 0x29), std::string("Backquote"));    // ⌥半角/全角
    CHECK_EQ(code(vk::kOemEnlw, 0), std::string("Backquote"));
    CHECK_EQ(code(vk::kLShift, 0x2A), std::string());
    CHECK_EQ(code(vk::kCapital, 0x3A), std::string());
    CHECK_EQ(code(vk::kRMenu, 0x38, true), std::string());
    CHECK_EQ(code(vk::kPacket, 0x61), std::string("Unidentified"));
    CHECK_EQ(code(vk::kF1 + 6, 0x41), std::string("F7"));
    CHECK_EQ(code(vk::kF24, 0x76), std::string("F24"));
    CHECK_EQ(code(vk::kReturn, 0x1C, true), std::string("Enter"));
    CHECK_EQ(code(vk::kLeft, 0x4B, true), std::string("ArrowLeft"));
    CHECK_EQ(code(vk::kNumpad0 + 7, 0x47), std::string("Numpad7"));
    CHECK_EQ(code(vk::kDelete, 0x53, true), std::string("Delete"));
    CHECK_EQ(code(0xFF, 0x5E, true), std::string("Unidentified"));

    CHECK(lang(vk::kNonConvert, 0x7B) == LangKey::kMuhenkan);
    CHECK(lang(vk::kConvert, 0x79) == LangKey::kHenkan);
    CHECK(lang(vk::kConvert, 0) == LangKey::kHenkan);  // injected by virtual key
    CHECK(lang(vk::kDbeHiragana, 0x70) == LangKey::kKatakana);
    CHECK(lang(vk::kDbeKatakana, 0) == LangKey::kKatakana);
    CHECK(lang(vk::kKana, 0xF2) == LangKey::kKana);
    CHECK(lang(vk::kKanji, 0xF1) == LangKey::kEisu);
    CHECK(lang(vk::kCapital, 0x3A) == LangKey::kCapsLock);
    CHECK(lang(vk::kDbeAlphanumeric, 0x3A) == LangKey::kCapsLock);
    CHECK(lang('A', 0x1E) == LangKey::kNone);
    CHECK(lang(vk::kKanji, 0x29) == LangKey::kNone);  // ⌥半角/全角 is the engine's Backquote
    CHECK(lang(vk::kPacket, 0x79) == LangKey::kNone);

    CHECK_EQ(JisCharacter("KeyA", false, false), std::string("a"));
    CHECK_EQ(JisCharacter("KeyA", true, false), std::string("A"));
    CHECK_EQ(JisCharacter("KeyA", false, true), std::string("A"));
    CHECK_EQ(JisCharacter("KeyA", true, true), std::string("A"));  // v1.1 §7.3: ⇧ or not
    CHECK_EQ(JisCharacter("Digit2", true, false), std::string("\""));
    CHECK_EQ(JisCharacter("BracketLeft", false, true), std::string("@"));
    CHECK_EQ(JisCharacter("IntlRo", true, false), std::string("_"));
    CHECK_EQ(JisCharacter("IntlYen", false, false), std::string("\xC2\xA5"));
    CHECK_EQ(JisCharacter("Space", false, false), std::string());
    CHECK_EQ(JisCharacter("Key", false, false), std::string());

    CHECK_EQ(ScanOrMapped('G', 0, [](unsigned) { return 0x22u; }), 0x22u);
    CHECK_EQ(ScanOrMapped(vk::kPacket, 0, [](unsigned) { return 0x22u; }), 0u);
    CHECK_EQ(ScanOrMapped('G', 0x22, nullptr), 0x22u);

    const std::intptr_t first = 0x00220001, repeat = 0x40220001;
    CHECK(SameKeyDown(0x47, first, 100, 1000, 0x47, first, 100, 1000));
    CHECK(SameKeyDown(0x47, first, 100, 1000, 0x47, first, 90, 1200));
    CHECK(!SameKeyDown(0x47, first, 100, 1000, 0x47, first, 90, 1600));
    CHECK(!SameKeyDown(0x47, repeat, 100, 1000, 0x47, repeat, 130, 1030));
    CHECK(!SameKeyDown(0x47, first, 100, 1000, 0x48, first, 100, 1000));
}

const char* kWindows = R"("windows": {"clsid": "{11111111-2222-3333-4444-555555555555}",
                                      "profile": "{66666666-7777-8888-9999-AAAAAAAAAAAA}"})";

void TestManifest() {
    std::string error;
    auto m = ParseManifest(std::string(R"({"name": "x", "library": "{arch}/x.{ext}", "prefix": "x", )") +
                               kWindows + "}",
                           "/base", &error);
    CHECK(m.has_value());
    if (m) {
        CHECK_EQ(m->display_name, std::string("x"));
        CHECK_EQ(m->language, std::uint16_t{0x0411});
        CHECK_EQ(m->mode, std::string("henkan"));
        CHECK(!m->has_data);
        CHECK(m->settings_app.empty());  // no Settings… in the menu
        const std::string lib = LibraryPath(*m).generic_string();
#if defined(__APPLE__)
        CHECK(lib.find("/x.dylib") != std::string::npos);
#endif
        CHECK(lib.find("{") == std::string::npos);
    }
    {
        // windows.settings_app: the program the menu's Settings… starts.
        auto s = ParseManifest(R"({"name": "x", "library": "x", "prefix": "x", "windows": {
                                     "clsid": "{11111111-2222-3333-4444-555555555555}",
                                     "profile": "{66666666-7777-8888-9999-AAAAAAAAAAAA}",
                                     "settings_app": "x-settings.exe"}})",
                               "/base", &error);
        CHECK(s && s->settings_app == "x-settings.exe");
    }
    CHECK(!ParseManifest(R"({"name": "x", "library": "x", "prefix": "x"})", "/", &error));
    CHECK(error.find("windows") != std::string::npos);
    CHECK(!ParseManifest(R"({"name": "x", "library": "x", "prefix": "x", "windows": {"clsid": "{1}", "profile": "{2}"}})", "/", &error));
    CHECK(!ParseManifest(std::string(R"({"name": "x", "library": "x", "prefix": "x", "mode": "kana", )") + kWindows + "}", "/", &error));
    CHECK(IsGuid("{11111111-2222-3333-4444-555555555555}"));
    CHECK(!IsGuid("{11111111-2222-3333-4444-55555555555G}"));
    CHECK_EQ(LanguageId("ko-KR"), std::uint16_t{0x0412});
    CHECK_EQ(LanguageId("0x0409"), std::uint16_t{0x0409});
    CHECK_EQ(LanguageId("1041"), std::uint16_t{0x0411});
    CHECK_EQ(LanguageId("xx"), std::uint16_t{0});

    // Data: files, null, concatenations with text and a folder (superkeyboard's rules).
    const fs::path dir = fs::temp_directory_path() / "bw-core-tests";
    fs::remove_all(dir);
    fs::create_directories(dir / "brands");
    std::ofstream(dir / "a.txt", std::ios::binary) << "alpha";
    std::ofstream(dir / "brands" / "2.tsv", std::ios::binary) << "two\n";
    std::ofstream(dir / "brands" / "1.tsv", std::ios::binary) << "one";
    std::ofstream(dir / "brands" / "skip.txt", std::ios::binary) << "no";
    auto d = ParseManifest(std::string(R"({"name": "x", "library": "x", "prefix": "x",
        "data": ["a.txt", null, {"file": "a.txt"}, {"concat": ["a.txt", {"text": "#! b\n"}, {"dir": "brands", "suffix": ".tsv"}]}], )") +
                               kWindows + "}",
                           dir, &error);
    CHECK(d.has_value());
    std::vector<std::string> buffers;
    CHECK(d && LoadData(*d, &buffers, &error));
    CHECK_EQ(buffers.size(), size_t{4});
    if (buffers.size() == 4) {
        CHECK_EQ(buffers[0], std::string("alpha"));
        CHECK_EQ(buffers[1], std::string());
        CHECK_EQ(buffers[3], std::string("alpha\n#! b\none\ntwo\n"));
    }
    fs::remove_all(dir);
}

void TestPublishedState() {
    HostInfo info;
    info.ime = "shout";
    info.version = "0.1.0";
    info.app = "notepad.exe";
    const std::string engine = R"({"format":"bada-compatible-state/1","mode":"henkan","profile":null,
        "staged":"HELLO","preedit":"HELLO","menu":["HELLO","hello"],"selected":0,"zero":0,
        "horizontal":false,"need_selection":null,"host":{"name":"forged"}})";
    auto open = json::Parse(PublishedState(engine, info));
    CHECK(open.has_value());
    CHECK_EQ(open->GetString("staged"), std::string("HELLO"));
    CHECK_EQ(open->Get("host")->GetString("name"), std::string("bada-windows"));
    CHECK(!open->Get("host")->GetBool("secure", true));
    info.secure = true;
    auto secure = json::Parse(PublishedState(engine, info));
    CHECK(secure.has_value());
    CHECK_EQ(secure->GetString("staged", "x"), std::string());
    CHECK_EQ(secure->GetString("preedit", "x"), std::string());
    CHECK(secure->Get("menu")->array.empty());
    CHECK(secure->Get("host")->GetBool("secure"));
    CHECK(PublishedState(engine, info).find("HELLO") == std::string::npos);
    // An engine that answers nonsense still gives a state with the host's fields.
    auto junk = json::Parse(PublishedState("not json", info));
    CHECK(junk && junk->GetString("format") == "bada-compatible-state/1" && junk->Get("host"));
}

bool g_dead_thread = 0;
bool Alive(std::uint32_t thread_id, std::uint32_t) { return !(g_dead_thread && thread_id == 7); }

void TestStateTable() {
    auto table = std::make_unique<state::Table>();
    std::memset(table.get(), 0, sizeof(state::Table));
    state::Init(table.get());
    state::Snapshot snap;
    std::string staged;
    CHECK(state::Find(table.get(), 7, Alive, &snap) == state::Read::kNotActive);
    state::Slot* slot = state::Claim(table.get(), 7, 70, Alive);
    CHECK(slot != nullptr);
    CHECK(state::Claim(table.get(), 7, 70, Alive) == slot);
    // Claimed, nothing published yet: not active.
    CHECK(state::Find(table.get(), 7, Alive, &snap) == state::Read::kNotActive);
    state::Publish(table.get(), slot, 1, state::kFlagComposing, R"({"staged":"HELLO"})", 1);
    CHECK(state::Find(table.get(), 7, Alive, &snap) == state::Read::kOk);
    CHECK_EQ(snap.mode, 1);
    CHECK_EQ(table->last_thread, 7);
    CHECK(state::StagedText(state::Read::kOk, snap, &staged) == state::Staged::kOk);
    CHECK_EQ(staged, std::string("HELLO"));
    // A password field: the security error, whatever the text says.
    state::Publish(table.get(), slot, 1, state::kFlagSecure, R"({"staged":"leak"})", 2);
    CHECK(state::Find(table.get(), 7, Alive, &snap) == state::Read::kOk);
    CHECK(state::StagedText(state::Read::kOk, snap, &staged) == state::Staged::kSecure);
    CHECK(staged.empty());
    state::Publish(table.get(), slot, 0, state::kFlagNoStaged, R"({"staged":null})", 3);
    state::Find(table.get(), 7, Alive, &snap);
    CHECK(state::StagedText(state::Read::kOk, snap, &staged) == state::Staged::kUnknown);
    // Too long for the slot: no text, rather than half of it.
    state::Publish(table.get(), slot, 1, 0, std::string(state::kTextCapacity + 1, 'x'), 4);
    state::Find(table.get(), 7, Alive, &snap);
    CHECK(snap.json.empty());
    // A dead thread's slot is ignored, then reused when the table is full.
    g_dead_thread = true;
    CHECK(state::Find(table.get(), 7, Alive, &snap) == state::Read::kNotActive);
    for (std::uint32_t t = 100; t < 100 + state::kSlotCount - 1; ++t) CHECK(state::Claim(table.get(), t, 1, Alive));
    CHECK(state::Claim(table.get(), 999, 1, Alive) == slot);  // the dead one's
    CHECK(state::Claim(table.get(), 1000, 1, Alive) == nullptr);
    state::Release(table.get(), 999);
    CHECK(state::Claim(table.get(), 1000, 1, Alive) == slot);
    g_dead_thread = false;
}

std::unique_ptr<Module> LoadShout(const std::string& library) {
    std::string error;
    auto m = ParseManifest("{\"name\": \"shout\", \"library\": " + json::Quote(library) +
                               ", \"prefix\": \"shout\", " + kWindows + "}",
                           ".", &error);
    if (!m) {
        std::printf("FAIL shout manifest: %s\n", error.c_str());
        ++failures;
        return nullptr;
    }
    auto module = Module::Load(*m, &error);
    if (!module) {
        std::printf("FAIL loading %s: %s\n", library.c_str(), error.c_str());
        ++failures;
    }
    return module;
}

void TestShout(const Module& module) {
    CHECK_EQ(KeyEventJson("KeyA", Mods{true, false, false, false}),
             std::string(R"({"key":"KeyA","shift":true,"ctrl":false,"alt":false,"meta":false})"));
    {
        // superkeyboard's shout test: HELLO world, then muhenkan by 無変換.
        Engine engine(module);
        CHECK(engine.ok() && !engine.HasProfiles() && engine.HasState());
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kHenkan, field);
        t.Type("KeyH KeyE KeyL KeyL KeyO");
        CHECK_EQ(field.preedit(), std::string("HELLO"));
        CHECK_EQ(field.menu.size(), size_t{2});
        auto state = json::Parse(host.EngineState());
        CHECK(state && state->GetString("staged") == "HELLO" && state->GetString("mode") == "henkan");
        t.Type("Space KeyW KeyO KeyR KeyL KeyD Tab Enter");
        CHECK_EQ(field.text(), std::string("HELLO world\n"));
        t.Type("muhenkan KeyA KeyB KeyC Backspace");
        CHECK(host.mode() == Mode::kMuhenkan);
        CHECK(host.TakeStateChanged());
        CHECK_EQ(field.text(), std::string("HELLO world\nab"));
        CHECK_EQ(field.typed, 3);  // the host typed the JIS characters, the app the Backspace
        state = json::Parse(host.EngineState());
        CHECK(state && state->GetString("staged", "x").empty() && state->GetString("mode") == "muhenkan");
    }
    {
        // 変換 is the home key: from muhenkan it enters henkan; with a menu it moves in it (⇧: back).
        Engine engine(module);
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("henkan KeyH KeyI");
        CHECK(host.mode() == Mode::kHenkan);
        CHECK_EQ(field.preedit(), std::string("HI"));
        t.Type("henkan");
        CHECK_EQ(field.preedit(), std::string("hi"));
        t.Type("S-henkan");
        CHECK_EQ(field.preedit(), std::string("HI"));
        t.Type("C-henkan C-katakana");  // ⌃ Lang1 (v1.3, "fix what shows"): shout keeps what shows
        CHECK_EQ(field.preedit(), std::string("HI"));
        CHECK(host.mode() == Mode::kHenkan);
        t.Type("katakana");  // no profiles in shout: next entry too
        CHECK_EQ(field.preedit(), std::string("hi"));
        t.Type("eisu");  // 英数 commits, then muhenkan
        CHECK(host.mode() == Mode::kMuhenkan);
        CHECK_EQ(field.text(), std::string("hi"));
        t.Type("C-henkan");  // ⌃ + a profile key in muhenkan: nothing
        CHECK(host.mode() == Mode::kMuhenkan);
        t.Type("kana KeyX Period");  // かな: henkan (mac-kana); Period commits, then types
        CHECK_EQ(field.text(), std::string("hiX."));
        t.Type("C-muhenkan C-A-S-henkan");  // MEH + 変換: the other mode
        CHECK(host.mode() == Mode::kHenkan);
    }
    {
        // 半角/全角 in muhenkan shouts the selection (need_selection → P_host_selection).
        Engine engine(module);
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("KeyQ KeyU KeyI KeyE KeyT Space KeyP KeyL KeyE KeyA KeyS KeyE");
        field.Select(6, 12);
        t.Type("zenkaku");
        CHECK_EQ(field.text(), std::string("quiet PLEASE"));
    }
    {
        // Caps Lock: respect (default) leaves it to Windows; letters are capitals, ⇧ or not.
        Engine engine(module);
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("caps KeyA S-KeyB caps KeyC");
        CHECK_EQ(field.text(), std::string("ABc"));
        CHECK(host.mode() == Mode::kMuhenkan);
    }
    {
        // toggle: Caps Lock switches mode and stays off; ⇧ + Caps Lock is Caps Lock in muhenkan.
        Engine engine(module);
        HostSettings s;
        s.caps = CapsSetting::kToggle;
        Host host(engine, s, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("caps");
        CHECK(host.mode() == Mode::kHenkan);
        CHECK(!t.caps);
        t.Type("KeyH KeyI caps");
        CHECK(host.mode() == Mode::kMuhenkan);
        CHECK_EQ(field.text(), std::string("HI"));
        t.Type("S-caps KeyX");
        CHECK(t.caps);
        CHECK_EQ(field.text(), std::string("HIX"));
        t.Type("eisu-caps");  // the same key under the JIS layout (英数): toggles too
        CHECK(host.mode() == Mode::kHenkan);
        CHECK(!t.caps);  // leaving muhenkan forgets Caps Lock
        t.Type("muhenkan S-caps henkan");
        CHECK(!t.caps);
        CHECK(host.mode() == Mode::kHenkan);
    }
    {
        // respect, under the JIS layout: the unshifted Caps Lock key is 英数, muhenkan.
        Engine engine(module);
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kHenkan, field);
        t.Type("KeyA eisu-caps");
        CHECK(host.mode() == Mode::kMuhenkan);
        CHECK_EQ(field.text(), std::string("A"));
        t.Type("eisu-caps");
        CHECK(host.mode() == Mode::kMuhenkan);
    }
    {
        // transient: Caps Lock in muhenkan only; in henkan it does nothing and stays off.
        Engine engine(module);
        HostSettings s;
        s.caps = CapsSetting::kTransient;
        Host host(engine, s, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kHenkan, field);
        t.Type("caps");
        CHECK(!t.caps);
        CHECK(host.mode() == Mode::kHenkan);
        t.Type("muhenkan caps KeyA henkan");
        CHECK_EQ(field.text(), std::string("A"));
        CHECK(!t.caps);
        t.caps = true;  // another program turned it on in henkan
        t.Type("KeyB");
        CHECK(!t.caps);
    }
    {
        // UPPER_SNAKE: Space types _, ⌥Space a space, in muhenkan with Caps Lock on.
        Engine engine(module);
        HostSettings s;
        s.caps_rule = CapsRule::kUpperSnake;
        Host host(engine, s, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("caps KeyA Space KeyB A-Space KeyC caps Space");
        CHECK_EQ(field.text(), std::string("A_B C "));
    }
    {
        // Windows' layout types what the engine passes on (pass_on = windows).
        Engine engine(module);
        HostSettings s;
        s.pass_on = PassOn::kWindows;
        Host host(engine, s, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("KeyA S-Digit2");
        CHECK_EQ(field.text(), std::string("a\""));
        CHECK_EQ(field.typed, 0);
        CHECK_EQ(field.app_typed, 2);
        // A password field: nothing reaches the engine.
        HostSettings jis;
        host.set_settings(jis);
        KeyDown k;
        k.vk = 'K';
        k.scan = 0x25;
        CHECK(host.TypeOnly(k, field));
        CHECK_EQ(field.text(), std::string("a\"k"));
    }
    {
        // A modifier on its own never commits; a shortcut commits, then goes to the app.
        Engine engine(module);
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.SwitchMode(Mode::kHenkan, field);
        t.Type("KeyO KeyK Shift");
        CHECK_EQ(field.preedit(), std::string("OK"));
        CHECK(!t.Press("C-KeyA"));
        CHECK_EQ(field.text(), std::string("OK"));
        CHECK(!host.composing());
    }
    {
        // Engines on one Data, on two threads' worth of hosts: independent.
        Engine a(module), b(module);
        Host ha(a, HostSettings{}, Mode::kHenkan), hb(b, HostSettings{}, Mode::kHenkan);
        TextField fa, fb;
        Typist ta{ha, fa}, tb{hb, fb};
        ha.SwitchMode(Mode::kHenkan, fa);
        hb.SwitchMode(Mode::kMuhenkan, fb);
        ta.Type("KeyX");
        tb.Type("KeyX");
        CHECK_EQ(fa.preedit(), std::string("X"));
        CHECK_EQ(fb.text(), std::string("x"));
    }
}

}  // namespace

int main(int argc, char** argv) {
    TestJson();
    TestOutput();
    TestKeys();
    TestManifest();
    TestPublishedState();
    TestStateTable();
    std::string shout = argc > 1 ? argv[1] : "";
    if (shout.empty()) {
        if (const char* env = std::getenv("SHOUT_LIB")) shout = env;
    }
    if (shout.empty()) {
        std::printf("skipped the engine tests: no shout library (argument or SHOUT_LIB)\n");
    } else if (auto module = LoadShout(shout)) {
        TestShout(*module);
    }
    std::printf("%d checks, %d failed\n", checks, failures);
    return failures ? 1 : 0;
}

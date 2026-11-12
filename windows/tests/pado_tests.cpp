// pado's tests through bada-windows' core: the built IME (build/ime/pado/ime.json) loaded as the TIP
// loads it (the manifest, the 11 data buffers, en-ko's concatenation), then bada's layouts typed
// through the host logic into a plain text field. Reuses the host's core tests' field and typist:
// core_tests.cpp is compiled in with its main renamed. Built and run by windows/test.ps1.
//
//   pado_tests path\to\build\ime\pado\ime.json [path\to\dictionary-example.json]
#define main bw_core_tests_main
#include "../host/tests/core_tests.cpp"
#undef main

#include <chrono>

namespace {

void Settings(Engine& engine, const std::string& json) { engine.SetSettings(json); }

bool MenuHas(const TextField& field, const std::string& entry) {
    for (const std::string& m : field.menu) {
        if (m == entry) return true;
    }
    return false;
}

void TestPado(const Module& module, const std::string& dictionary) {
    Engine engine(module);
    CHECK(engine.ok());
    CHECK(engine.HasProfiles());  // bada_set_profile: henkan profiles
    CHECK(engine.HasState());     // bada_state: bada-compatible v1.1
    CHECK(engine.HasHostSelection());
    {
        // The catalog the settings window shows: every layout of spec §5.2.
        auto catalog = json::Parse(engine.Catalog());
        CHECK(catalog && catalog->Get("layouts") && catalog->Get("layouts")->array.size() >= 13);
    }
    {
        // Muhenkan: QWERTY, the JIS symbols typed by the host.
        Settings(engine, R"({"enabled":[13]})");
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kMuhenkan, kProfileHenkan, field);
        t.Type("KeyA KeyB Digit1 S-Digit2 Space");
        CHECK_EQ(field.text(), std::string("ab1\" "));
    }
    {
        // Korean 2-set (10): 変換, then 한글, Space commits and types the space.
        Settings(engine, R"({"enabled":[10]})");
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kMuhenkan, kProfileHenkan, field);
        t.Type("henkan KeyG KeyK KeyS KeyR KeyM KeyF");
        CHECK(host.mode() == Mode::kHenkan);
        CHECK_EQ(field.preedit(), std::string("한글"));
        t.Type("Space");
        CHECK_EQ(field.text(), std::string("한글 "));
        // 無変換 back to QWERTY.
        t.Type("muhenkan KeyX");
        CHECK(host.mode() == Mode::kMuhenkan);
        CHECK_EQ(field.text(), std::string("한글 x"));
    }
    {
        // QWERTY + bada roman (0, 15), README's example: g w i h a → 귀하 first.
        Settings(engine, R"({"enabled":[0,15]})");
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kHenkan, kProfileHenkan, field);
        t.Type("KeyG KeyW KeyI KeyH KeyA");
        CHECK_EQ(field.preedit(), std::string("귀하"));
        CHECK(MenuHas(field, "gwiha"));
        // 変換 (the profile in use) moves down the menu; Enter commits what shows, then is the app's.
        t.Type("henkan");
        CHECK(field.preedit() != "귀하");
        t.Type("S-henkan Enter");
        CHECK_EQ(field.text(), std::string("귀하\n"));
        // A common English word shows as English (bada 0.2.2: en-words.txt's counts after a tab
        // aren't part of the word; before, love showed 로베).
        t.Type("KeyL KeyO KeyV KeyE");
        CHECK_EQ(field.preedit(), std::string("love"));
        t.Type("Escape");
    }
    {
        // ⌃変換 (bada 0.3.1's ⌃かな, spec §6.3): fixes what shows, still staged; the keys after it
        // read on, and what was fixed doesn't change. With nothing staged, nothing.
        Settings(engine, R"({"enabled":[0,15]})");
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kHenkan, kProfileHenkan, field);
        t.Type("C-henkan");
        CHECK(field.text().empty() && field.preedit().empty() && host.mode() == Mode::kHenkan);
        t.Type("KeyG KeyW KeyI KeyH KeyA C-henkan");
        CHECK_EQ(field.preedit(), std::string("귀하"));
        CHECK(host.composing());
        t.Type("KeyG KeyA");
        CHECK_EQ(field.preedit().rfind("귀하", 0), 0u);
        CHECK(field.preedit() != "귀하");
        t.Type("Escape");
        CHECK_EQ(field.text().rfind("귀하", 0), 0u);
        // In muhenkan, ⌃変換 does nothing (and doesn't enter henkan).
        host.SwitchMode(Mode::kMuhenkan, field);
        t.Type("C-henkan");
        CHECK(host.mode() == Mode::kMuhenkan);
    }
    {
        // bada plain (16), the default henkan candidate since bada 0.3.0: it reads keys.
        Settings(engine, R"({})");
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kMuhenkan, kProfileHenkan, field);
        t.Type("henkan KeyJ KeyF");
        CHECK(host.mode() == Mode::kHenkan);
        CHECK(!field.preedit().empty() && field.preedit() != "jf");
        t.Type("Escape");
    }
    {
        // Japanese kana (20), README's example: 3 u q → あなた among the readings.
        Settings(engine, R"({"enabled":[20]})");
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kHenkan, kProfileHenkan, field);
        t.Type("Digit3 KeyU KeyQ");
        CHECK(field.preedit() == "あなた" || MenuHas(field, "あなた"));
        t.Type("Space");
        CHECK(!field.text().empty() && !host.composing());
    }
    {
        // Henkan profiles (spec §6.6): 変換 and カタカナ each their own candidates.
        Settings(engine, R"({"enabled":[10],"use_profiles":true,"profiles":{"katakana":{"enabled":[20]}}})");
        Host host(engine, HostSettings{}, Mode::kMuhenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kMuhenkan, kProfileHenkan, field);
        t.Type("katakana Digit3 KeyU KeyQ");
        CHECK(host.profile() == kProfileKatakana);
        CHECK(field.preedit() == "あなた" || MenuHas(field, "あなた"));
        // With a menu up, 変換 in another profile moves in it (host spec §4.2); with none, it switches.
        t.Type("Space henkan KeyG KeyK KeyS");
        CHECK(host.profile() == kProfileHenkan);
        CHECK_EQ(field.preedit(), std::string("한"));
        t.Type("Space");
        CHECK(field.text().size() > 0 && field.text().find("한 ") != std::string::npos);
    }
    {
        // 半角/全角 (Backquote) in henkan: full-width of what's staged (spec §6.3).
        Settings(engine, R"({"enabled":[0]})");
        Host host(engine, HostSettings{}, Mode::kHenkan);
        TextField field;
        Typist t{host, field};
        host.Start(Mode::kHenkan, kProfileHenkan, field);
        t.Type("KeyA KeyB zenkaku Space");
        CHECK(field.text().rfind("ａｂ", 0) == 0);
    }
    if (!dictionary.empty()) {
        // The user dictionary (spec §8), as the TIP loads it.
        std::string text;
        CHECK(ReadFile(PathFromUtf8(dictionary), &text));
        auto summary = json::Parse(module.LoadDictionary(text));
        CHECK(summary && summary->Get("ok") && summary->Get("ok")->boolean);
        CHECK(json::Parse(module.LoadDictionary("")) != std::nullopt);
    }
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::printf("usage: pado_tests ime.json [dictionary.json]\n");
        return 2;
    }
    std::string error;
    auto manifest = LoadManifest(PathFromUtf8(argv[1]), &error);
    CHECK(manifest.has_value());
    if (!manifest) {
        std::printf("FAIL manifest: %s\n", error.c_str());
        return 1;
    }
    CHECK_EQ(manifest->prefix, std::string("bada"));
    CHECK_EQ(manifest->data.size(), size_t{11});
    CHECK(!manifest->settings_app.empty());
    const auto start = std::chrono::steady_clock::now();
    auto module = Module::Load(*manifest, &error);
    const auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start).count();
    CHECK(module != nullptr);
    if (!module) {
        std::printf("FAIL loading the engine: %s\n", error.c_str());
        return 1;
    }
    std::printf("engine and data loaded in %lld ms\n", static_cast<long long>(ms));
    TestPado(*module, argc > 2 ? argv[2] : "");
    std::printf("%d checks, %d failed\n", checks, failures);
    return failures ? 1 : 0;
}

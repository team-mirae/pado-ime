// The engine manifest (docs/spec.md §2): superkeyboard's manifest (team-mirae/superkeyboard,
// docs/ime-contract.md §8), so the same fields describe the engine, plus a "windows" object with what
// only a Windows input method needs: its permanent GUIDs, its language and its indicator labels.
#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "json.h"

namespace bw {

// One buffer of P_data_new.
struct DataItem {
    enum class Kind { kNull, kFile, kConcat } kind = Kind::kNull;
    std::string file;  // kFile
    struct Part {
        enum class Kind { kFile, kText, kDir } kind = Kind::kFile;
        std::string value;   // a path, or the text
        std::string suffix;  // kDir
    };
    std::vector<Part> parts;  // kConcat
};

struct Manifest {
    std::filesystem::path root;  // relative paths start here
    std::string name;
    std::string description;
    std::string library;  // {ext} → dll / dylib / so, {arch} → x64 / x86 / arm64
    std::string prefix;
    std::vector<DataItem> data;  // empty: P_new(NULL)
    bool has_data = false;       // "data" present (possibly []): P_data_new is called
    json::Value settings;        // what every engine starts with (an object, or null)
    std::string mode = "henkan";
    std::string keyboard = "jis";

    // "windows"
    std::string display_name;  // the profile's name in Windows' language list; default: name
    std::string clsid;         // "{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}", permanent
    std::string profile;       // the language profile's GUID, permanent
    std::uint16_t language = 0x0411;  // LANGID; default ja-JP (docs/spec.md §2.2)
    std::string label_henkan = "\xE3\x81\x82";  // あ
    std::string label_muhenkan = "A";
    std::string icon;  // an .ico file (relative to root); default: the DLL's own icon
};

std::optional<Manifest> ParseManifest(std::string_view text, const std::filesystem::path& manifest_dir,
                                      std::string* error);
std::optional<Manifest> LoadManifest(const std::filesystem::path& file, std::string* error);

// The engine library's path, placeholders filled in for this build.
std::filesystem::path LibraryPath(const Manifest& manifest);

// The buffers for P_data_new, in order (superkeyboard's rules: a concatenated file part ends with a
// newline, a directory part takes its matching files in name order, each newline-terminated).
bool LoadData(const Manifest& manifest, std::vector<std::string>* buffers, std::string* error);

// "{8-4-4-4-12}" with hex digits.
bool IsGuid(std::string_view text);
// "ja-JP", "ko-KR", "en-US", … or a number ("0x0411", "1041"). 0: unknown.
std::uint16_t LanguageId(std::string_view tag);

std::filesystem::path PathFromUtf8(std::string_view utf8);
std::string Utf8Path(const std::filesystem::path& path);
bool ReadFile(const std::filesystem::path& file, std::string* bytes);

}  // namespace bw

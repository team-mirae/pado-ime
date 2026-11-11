// The IME this DLL is: its manifest (docs/spec.md §2), found next to the DLL (ime.json, or one
// folder up for the x64 / x86 layout), and the engine module every text service in the process
// shares, loaded on the thread pool at the first activation.
#pragma once

#include <windows.h>

#include <memory>
#include <string>

#include "engine.h"
#include "manifest.h"

namespace bw {

struct Product {
    Manifest manifest;
    std::wstring manifest_path;
    CLSID clsid = {};
    GUID profile = {};
    std::wstring display_name;  // UTF-16
    std::wstring key_name;      // the settings key and log name: the manifest's name
};

std::wstring Wide(const std::string& utf8);
std::string Utf8(const std::wstring& wide);
std::wstring ModuleDirectory();

// nullptr when the manifest is missing or invalid (ProductError says why).
const Product* GetProduct();
const std::wstring& ProductError();

// The shared engine module. Preload starts loading it in the background; SharedModule waits for it
// (nullptr if it failed: the text service then passes every key through). ReleaseSharedModule frees
// it when nothing uses the DLL (DllCanUnloadNow).
void PreloadModule();
const Module* SharedModule();
void ReleaseSharedModule();

// The user dictionary (P_dictionary_load): %APPDATA%\bada-windows\<name>\dictionary.json. Loaded
// with the module, and again by RefreshDictionary when the file changed (or went away) since: one
// file time compared, cheap enough for every focus change. Does nothing until the module is loaded.
std::wstring DictionaryPath();
void RefreshDictionary();

// The menu's Settings… (the manifest's windows.settings_app) and Open Dictionary Folder.
void OpenSettingsApp();
void OpenDictionaryFolder();

}  // namespace bw

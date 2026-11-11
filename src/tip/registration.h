// COM server and TSF registration (regsvr32, as admin; tools/install.ps1), from the manifest.
#pragma once

#include <windows.h>

namespace bw {

HRESULT RegisterServer(HINSTANCE module);
HRESULT UnregisterServer();
HRESULT RegisterProfiles(HINSTANCE module);
HRESULT UnregisterProfiles();
HRESULT RegisterCategories();
HRESULT UnregisterCategories();

}  // namespace bw

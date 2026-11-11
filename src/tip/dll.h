// The DLL's module handle and lock count (dllmain.cpp).
#pragma once

#include <windows.h>

namespace bw {

HINSTANCE DllInstance();
void DllAddRef();
void DllRelease();
// Runs `job` on the thread pool, holding the DLL loaded until it has returned.
bool RunInBackground(void (*job)());

}  // namespace bw

/* bada-windows public API, version 1 (docs/api.md).
 *
 * Exported by every bada-windows IME's DLL (x64 and x86). Find the DLL through the IME's CLSID (its
 * manifest's windows.clsid):
 *   HKLM\SOFTWARE\Classes\CLSID\{CLSID}\InprocServer32   (default value)
 * 32-bit callers get the x86 DLL from the same key through registry redirection.
 *
 * The functions only read the session's state table (bada-compatible v1.1 §12.1): loading the DLL
 * this way doesn't activate the IME, and callers may load and unload it per call (AutoHotkey's
 * DllCall does). Nothing a caller does changes the IME. Strings are UTF-16; `capacity` counts
 * wchar_t, the terminating NUL included. With buffer NULL or capacity 0, the functions return the
 * length needed (without the NUL) and write nothing. */
#ifndef BADA_WINDOWS_API_H
#define BADA_WINDOWS_API_H

#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

#define BADA_WINDOWS_API_VERSION 1

#define BADA_WINDOWS_MUHENKAN 0
#define BADA_WINDOWS_HENKAN 1
#define BADA_WINDOWS_NOT_ACTIVE (-1)    /* the IME isn't active on that thread (or it's unknown) */
#define BADA_WINDOWS_UNAVAILABLE (-2)   /* no state table: the IME hasn't run in this session */
#define BADA_WINDOWS_E_SECURE (-3)      /* security error: a password field; what's typed is never given out
                                           (GetLastError: ERROR_ACCESS_DENIED) */
#define BADA_WINDOWS_E_BUFFER (-4)      /* the buffer is too small */
#define BADA_WINDOWS_E_UNKNOWN (-5)     /* the engine doesn't report what's staged (no P_state) */
#define BADA_WINDOWS_E_BUSY (-6)        /* the state kept changing while being read: try again */

/* BADA_WINDOWS_API_VERSION. */
int WINAPI BadaWindowsApiVersion(void);

/* The mode on the thread that owns `window` (NULL: the focused window of the foreground thread):
 * BADA_WINDOWS_MUHENKAN, BADA_WINDOWS_HENKAN, or a negative value above. */
int WINAPI BadaWindowsQueryWindow(HWND window);
int WINAPI BadaWindowsQueryThread(DWORD thread_id);

/* The published state as JSON (bada-compatible-state/1 with a "host" object; docs/api.md). In a
 * password field it's published with staged, preedit and menu empty and host.secure true. Returns the
 * length written, or a negative value above. */
int WINAPI BadaWindowsGetState(HWND window, wchar_t* buffer, int capacity);

/* What's staged: the text P_flush would commit now. In a password field: BADA_WINDOWS_E_SECURE.
 * Returns the length written ("" is 0), or a negative value above. */
int WINAPI BadaWindowsGetStaged(HWND window, wchar_t* buffer, int capacity);

/* The registered window message posted to top-level windows (HWND_BROADCAST) when the mode, the
 * profile, secure input or whether something is staged changes: wParam the thread id, lParam the
 * mode (bits 0-7) and the slot flags (bits 8-15). Typing within a composition posts nothing: poll
 * GetState / GetStaged for the text. */
UINT WINAPI BadaWindowsStateMessage(void);

#ifdef __cplusplus
}
#endif

#endif

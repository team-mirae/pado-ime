// Follows themrsung/fkeyime's state table and API.
// The public API (include/bada_windows_api.h, docs/api.md): read-only queries of the state table.
#include "bada_windows_api.h"

#include <string>

#include "product.h"
#include "state_shared.h"

using namespace bw;

namespace {

DWORD FocusThread() {
    GUITHREADINFO info = {sizeof(info)};
    if (!GetGUIThreadInfo(0, &info)) return 0;  // 0: the foreground thread
    HWND window = info.hwndFocus ? info.hwndFocus : info.hwndActive;
    return window ? GetWindowThreadProcessId(window, nullptr) : 0;
}

DWORD ThreadOf(HWND window) { return window ? GetWindowThreadProcessId(window, nullptr) : FocusThread(); }

// Opens the table read-only for this call only: callers such as AutoHotkey load and unload the DLL
// every time, so nothing may stay mapped.
int Snapshot(DWORD thread_id, state::Read* read, state::Snapshot* out) {
    if (!thread_id) return BADA_WINDOWS_NOT_ACTIVE;
    HANDLE mapping = OpenFileMappingW(FILE_MAP_READ, FALSE, shared::MappingName().c_str());
    if (!mapping) return BADA_WINDOWS_UNAVAILABLE;
    int result = BADA_WINDOWS_UNAVAILABLE;
    if (const auto* table = static_cast<const state::Table*>(MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, sizeof(state::Table)))) {
        *read = state::Find(table, thread_id, shared::ThreadAlive, out);
        result = *read == state::Read::kOk ? 0 : *read == state::Read::kBusy ? BADA_WINDOWS_E_BUSY : BADA_WINDOWS_NOT_ACTIVE;
        UnmapViewOfFile(table);
    }
    CloseHandle(mapping);
    return result;
}

int Copy(const std::string& utf8, wchar_t* buffer, int capacity) {
    const std::wstring text = Wide(utf8);
    if (text.size() > 0x7FFFFFF0) return BADA_WINDOWS_E_BUFFER;
    const int length = static_cast<int>(text.size());
    if (!buffer || capacity <= 0) return length;
    if (capacity <= length) return BADA_WINDOWS_E_BUFFER;
    text.copy(buffer, text.size());
    buffer[length] = L'\0';
    return length;
}

}  // namespace

extern "C" int WINAPI BadaWindowsApiVersion(void) { return BADA_WINDOWS_API_VERSION; }

extern "C" int WINAPI BadaWindowsQueryThread(DWORD thread_id) {
    state::Read read = state::Read::kNotActive;
    state::Snapshot snapshot;
    const int result = Snapshot(thread_id, &read, &snapshot);
    return result < 0 ? result : snapshot.mode;
}

extern "C" int WINAPI BadaWindowsQueryWindow(HWND window) { return BadaWindowsQueryThread(ThreadOf(window)); }

extern "C" int WINAPI BadaWindowsGetState(HWND window, wchar_t* buffer, int capacity) {
    state::Read read = state::Read::kNotActive;
    state::Snapshot snapshot;
    const int result = Snapshot(ThreadOf(window), &read, &snapshot);
    return result < 0 ? result : Copy(snapshot.json, buffer, capacity);
}

extern "C" int WINAPI BadaWindowsGetStaged(HWND window, wchar_t* buffer, int capacity) {
    state::Read read = state::Read::kNotActive;
    state::Snapshot snapshot;
    const int result = Snapshot(ThreadOf(window), &read, &snapshot);
    if (result == BADA_WINDOWS_UNAVAILABLE) return result;
    std::string staged;
    switch (state::StagedText(read, snapshot, &staged)) {
        case state::Staged::kOk: return Copy(staged, buffer, capacity);
        case state::Staged::kSecure:
            // The security error: a password field's text is never given out, not even its length.
            if (buffer && capacity > 0) buffer[0] = L'\0';
            SetLastError(ERROR_ACCESS_DENIED);
            return BADA_WINDOWS_E_SECURE;
        case state::Staged::kUnknown: return BADA_WINDOWS_E_UNKNOWN;
        case state::Staged::kBusy: return BADA_WINDOWS_E_BUSY;
        case state::Staged::kNotActive: break;
    }
    return BADA_WINDOWS_NOT_ACTIVE;
}

extern "C" UINT WINAPI BadaWindowsStateMessage(void) { return shared::StateMessage(); }

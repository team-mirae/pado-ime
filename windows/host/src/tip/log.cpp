#include "log.h"

#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <iterator>
#include <string>

#include "product.h"
#include "settings.h"

namespace bw::log {

void Write(const wchar_t* format, ...) {
    if (!settings::ReadDword(L"DiagnosticLog", 0)) return;
    const Product* product = GetProduct();
    wchar_t folder[MAX_PATH];
    if (!ExpandEnvironmentStringsW(L"%ProgramData%\\bada-windows", folder, MAX_PATH)) return;
    CreateDirectoryW(folder, nullptr);
    const std::wstring path = std::wstring(folder) + L"\\" + (product ? product->key_name : L"bada-windows") + L".log";
    SYSTEMTIME t;
    GetLocalTime(&t);
    wchar_t line[1024];
    int n = swprintf_s(line, L"%02u:%02u:%02u.%03u %lu %s ", t.wHour, t.wMinute, t.wSecond, t.wMilliseconds,
                       GetCurrentProcessId(), settings::ExeName().c_str());
    if (n < 0) return;
    va_list args;
    va_start(args, format);
    // _TRUNCATE: an overlong line is cut, never the process ended (jis-moonshot's notes).
    int m = _vsnwprintf_s(line + n, std::size(line) - n - 1, _TRUNCATE, format, args);
    va_end(args);
    if (m < 0) m = static_cast<int>(wcsnlen(line + n, std::size(line) - n - 1));
    size_t length = static_cast<size_t>(n + m);
    line[length++] = L'\n';
    char utf8[3072];
    const int bytes = WideCharToMultiByte(CP_UTF8, 0, line, static_cast<int>(length), utf8, sizeof(utf8), nullptr, nullptr);
    if (bytes <= 0) return;
    // FILE_APPEND_DATA: every write lands at the end, so processes don't overwrite each other.
    HANDLE file = CreateFileW(path.c_str(), FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr,
                              OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return;
    DWORD written;
    WriteFile(file, utf8, static_cast<DWORD>(bytes), &written, nullptr);
    CloseHandle(file);
}

}  // namespace bw::log

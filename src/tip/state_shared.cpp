#include "state_shared.h"

#include <sddl.h>

#include <string>

#include "product.h"

namespace bw::shared {

std::wstring MappingName() {
    const Product* p = GetProduct();
    return L"Local\\bada-windows.state.v1." + (p ? Wide(p->manifest.clsid) : std::wstring(L"none"));
}

namespace {

// "D:P(A;;GA;;;SY)(A;;GA;;;<user>)S:(ML;;NWNR;;;ME)": SYSTEM and the user, medium integrity and up.
std::wstring Descriptor() {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return {};
    BYTE buffer[sizeof(TOKEN_USER) + SECURITY_MAX_SID_SIZE];
    DWORD size = 0;
    std::wstring sddl;
    if (GetTokenInformation(token, TokenUser, buffer, sizeof(buffer), &size)) {
        LPWSTR sid = nullptr;
        if (ConvertSidToStringSidW(reinterpret_cast<TOKEN_USER*>(buffer)->User.Sid, &sid)) {
            sddl = std::wstring(L"D:P(A;;GA;;;SY)(A;;GA;;;") + sid + L")S:(ML;;NWNR;;;ME)";
            LocalFree(sid);
        }
    }
    CloseHandle(token);
    return sddl;
}

}  // namespace

state::Table* Writable() {
    static state::Table* table = []() -> state::Table* {
        const std::wstring sddl = Descriptor();
        if (sddl.empty()) return nullptr;
        PSECURITY_DESCRIPTOR sd = nullptr;
        if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.c_str(), SDDL_REVISION_1, &sd, nullptr)) {
            return nullptr;
        }
        SECURITY_ATTRIBUTES sa = {sizeof(sa), sd, FALSE};
        HANDLE mapping = CreateFileMappingW(INVALID_HANDLE_VALUE, &sa, PAGE_READWRITE, 0, sizeof(state::Table),
                                            MappingName().c_str());
        LocalFree(sd);
        if (!mapping) return nullptr;
        auto* view = static_cast<state::Table*>(
            MapViewOfFile(mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(state::Table)));
        // The handle stays open for the life of the process, so the table outlives each text service.
        if (view) state::Init(view);
        return view;
    }();
    return table;
}

bool ThreadAlive(std::uint32_t thread_id, std::uint32_t process_id) {
    HANDLE thread = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, FALSE, thread_id);
    if (!thread) return GetLastError() == ERROR_ACCESS_DENIED;  // exists (an elevated process)
    DWORD code = 0;
    const bool alive = GetExitCodeThread(thread, &code) && code == STILL_ACTIVE &&
                       GetProcessIdOfThread(thread) == process_id;
    CloseHandle(thread);
    return alive;
}

UINT StateMessage() {
    static const UINT message = RegisterWindowMessageW(L"bada-windows.state");
    return message;
}

}  // namespace bw::shared

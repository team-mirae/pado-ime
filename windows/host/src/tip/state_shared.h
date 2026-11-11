// Follows themrsung/fkeyime's state table and API.
// The session's state table (src/core/state_table.h) in shared memory, and the notification message.
#pragma once

#include <windows.h>

#include <string>

#include "state_table.h"

namespace bw::shared {

// The IME's mapping name: Local\bada-windows.state.v1.{CLSID} (several bada-windows IMEs each have one).
std::wstring MappingName();

// The table, created on first use for writing (kept mapped for the life of the process). Readable
// and writable by the user's own processes at medium integrity and above, nobody else: it holds what
// they type (low-integrity and AppContainer processes, sandboxed browser parts included, can't read
// it; their own text services can't publish either, docs/api.md).
state::Table* Writable();

bool ThreadAlive(std::uint32_t thread_id, std::uint32_t process_id);

UINT StateMessage();

}  // namespace bw::shared

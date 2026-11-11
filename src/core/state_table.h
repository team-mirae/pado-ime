// Follows themrsung/fkeyime's state table and API.
// The state other programs read (docs/api.md): a table in session-wide shared memory with one slot
// per thread where the IME is active, holding its published state (bada-compatible v1.1 §12.1: the
// engine's P_state with the host's fields under "host"). Writers are the text services (one per
// thread, in every app); readers are the exported API (api.cpp), loaded by AutoHotkey and the like.
//
// FKeyIME's state table (docs/api.md there), grown a seqlock'd JSON text per slot. The layout is
// version 1 of an ABI: only ever append. Portable (no Windows headers): the slot logic is tested on
// any OS; the shared mapping itself is the shell's (src/tip/state_shared.cpp).
#pragma once

#include <cstdint>
#include <string>

namespace bw::state {

inline constexpr std::uint32_t kMagic = 0x54535742;  // "BWST"
inline constexpr std::uint32_t kVersion = 1;
inline constexpr std::uint32_t kSlotCount = 64;
inline constexpr std::uint32_t kTextCapacity = 8000;  // bytes of JSON per slot; longer: truncated menu

// Slot flags.
inline constexpr std::uint32_t kFlagSecure = 1;     // a password field: nothing typed is published
inline constexpr std::uint32_t kFlagOff = 2;        // the IME is off in this program
inline constexpr std::uint32_t kFlagComposing = 4;  // something is staged
inline constexpr std::uint32_t kFlagNoStaged = 8;   // the engine doesn't report what's staged

struct Slot {
    std::int32_t thread_id;   // 0: free
    std::uint32_t process_id;
    std::int32_t mode;        // 0 muhenkan, 1 henkan
    std::uint32_t flags;
    std::uint32_t sequence;   // odd while being written
    std::uint32_t length;     // bytes of `text`
    std::int64_t updated_ms;  // a monotonic clock, for readers that want "the latest"
    char text[kTextCapacity];  // UTF-8 JSON, not NUL-terminated
};

struct Table {
    std::uint32_t magic;
    std::uint32_t version;
    std::uint32_t slot_count;
    std::uint32_t slot_size;
    std::int32_t last_thread;  // the thread that published last (the one typing)
    std::uint32_t reserved[3];
    Slot slots[kSlotCount];
};

// Whether a thread still exists and belongs to the process. Injected so tests can fake it.
using AliveFn = bool (*)(std::uint32_t thread_id, std::uint32_t process_id);

// Fills in the header of a zeroed table. Idempotent.
void Init(Table* table);

// The slot of `thread_id`, claimed if it has none (free slots first, then those of dead threads).
// nullptr when every slot belongs to a live thread.
Slot* Claim(Table* table, std::uint32_t thread_id, std::uint32_t process_id, AliveFn alive);
// Writes the state into the thread's slot (seqlock: readers never see half of it).
void Publish(Table* table, Slot* slot, std::int32_t mode, std::uint32_t flags, const std::string& json,
             std::int64_t now_ms);
void Release(Table* table, std::uint32_t thread_id);

enum class Read { kOk, kNotActive, kBusy };
struct Snapshot {
    std::int32_t mode = -1;
    std::uint32_t flags = 0;
    std::string json;
};
// The state of `thread_id`. kNotActive when it has no slot, or its thread died.
Read Find(const Table* table, std::uint32_t thread_id, AliveFn alive, Snapshot* out);

// What's staged in a snapshot, for the API's "staged text" query. kSecure in a password field (the
// security error: what's typed there is never given out, not even as ""), kUnknown when the engine
// doesn't report it (no P_state).
enum class Staged { kOk, kNotActive, kBusy, kSecure, kUnknown };
Staged StagedText(Read read, const Snapshot& snapshot, std::string* out);

}  // namespace bw::state

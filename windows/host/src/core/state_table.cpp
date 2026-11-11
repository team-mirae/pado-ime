#include "state_table.h"

#include <atomic>
#include <cstring>

#include "json.h"

namespace bw::state {
namespace {

// Fields shared between processes are read and written atomically (lock-free 32-bit atomics are
// address-free, so they work across processes on one mapping).
template <typename T>
std::atomic_ref<T> At(T& field) {
    return std::atomic_ref<T>(field);
}
template <typename T>
T Load(const T& field) {
    return std::atomic_ref<T>(const_cast<T&>(field)).load(std::memory_order_acquire);
}

}  // namespace

void Init(Table* t) {
    if (Load(t->magic) == kMagic) return;
    t->version = kVersion;
    t->slot_count = kSlotCount;
    t->slot_size = sizeof(Slot);
    At(t->magic).store(kMagic, std::memory_order_release);
}

Slot* Claim(Table* t, std::uint32_t thread_id, std::uint32_t process_id, AliveFn alive) {
    const auto id = static_cast<std::int32_t>(thread_id);
    for (Slot& slot : t->slots) {
        if (Load(slot.thread_id) == id) {
            At(slot.process_id).store(process_id, std::memory_order_release);
            return &slot;
        }
    }
    // Two passes: free slots first, then slots whose thread died without releasing them.
    for (int pass = 0; pass < 2; ++pass) {
        for (Slot& slot : t->slots) {
            std::int32_t owner = Load(slot.thread_id);
            if (owner && (pass == 0 || alive(static_cast<std::uint32_t>(owner), Load(slot.process_id)))) {
                continue;
            }
            if (At(slot.thread_id).compare_exchange_strong(owner, id)) {
                At(slot.process_id).store(process_id, std::memory_order_release);
                At(slot.flags).store(0, std::memory_order_release);
                At(slot.mode).store(-1, std::memory_order_release);
                At(slot.length).store(0, std::memory_order_release);
                return &slot;
            }
        }
    }
    return nullptr;
}

void Publish(Table* t, Slot* slot, std::int32_t mode, std::uint32_t flags, const std::string& json,
             std::int64_t now_ms) {
    const std::uint32_t length =
        static_cast<std::uint32_t>(json.size() < kTextCapacity ? json.size() : 0);  // too long: none
    auto sequence = At(slot->sequence);
    std::uint32_t s = sequence.load(std::memory_order_relaxed);
    if (s & 1) ++s;  // a writer died mid-write: start clean
    sequence.store(s + 1, std::memory_order_relaxed);
    std::atomic_thread_fence(std::memory_order_release);
    At(slot->mode).store(mode, std::memory_order_relaxed);
    At(slot->flags).store(flags, std::memory_order_relaxed);
    At(slot->updated_ms).store(now_ms, std::memory_order_relaxed);
    std::memcpy(slot->text, json.data(), length);
    At(slot->length).store(length, std::memory_order_relaxed);
    sequence.store(s + 2, std::memory_order_release);
    At(t->last_thread).store(Load(slot->thread_id), std::memory_order_release);
}

void Release(Table* t, std::uint32_t thread_id) {
    const auto id = static_cast<std::int32_t>(thread_id);
    for (Slot& slot : t->slots) {
        std::int32_t expected = id;
        At(slot.thread_id).compare_exchange_strong(expected, 0);
    }
    std::int32_t expected = id;
    At(t->last_thread).compare_exchange_strong(expected, 0);
}

Read Find(const Table* t, std::uint32_t thread_id, AliveFn alive, Snapshot* out) {
    if (!thread_id || Load(t->magic) != kMagic) return Read::kNotActive;
    const auto id = static_cast<std::int32_t>(thread_id);
    for (const Slot& slot : t->slots) {
        if (Load(slot.thread_id) != id) continue;
        if (!alive(thread_id, Load(slot.process_id))) return Read::kNotActive;
        for (int attempt = 0; attempt < 64; ++attempt) {
            const std::uint32_t before = Load(slot.sequence);
            if (before & 1) continue;
            Snapshot s;
            s.mode = Load(slot.mode);
            s.flags = Load(slot.flags);
            std::uint32_t length = Load(slot.length);
            if (length > kTextCapacity) length = 0;
            s.json.assign(slot.text, length);
            std::atomic_thread_fence(std::memory_order_acquire);
            if (Load(slot.sequence) != before || Load(slot.thread_id) != id) continue;
            *out = std::move(s);
            return out->mode < 0 ? Read::kNotActive : Read::kOk;
        }
        return Read::kBusy;
    }
    return Read::kNotActive;
}

Staged StagedText(Read read, const Snapshot& snapshot, std::string* out) {
    out->clear();
    if (read == Read::kBusy) return Staged::kBusy;
    if (read != Read::kOk) return Staged::kNotActive;
    if (snapshot.flags & kFlagSecure) return Staged::kSecure;
    if (snapshot.flags & kFlagNoStaged) return Staged::kUnknown;
    const auto doc = json::Parse(snapshot.json);
    const json::Value* staged = doc ? doc->Get("staged") : nullptr;
    if (!staged || !staged->String()) return Staged::kUnknown;
    *out = *staged->String();
    return Staged::kOk;
}

}  // namespace bw::state

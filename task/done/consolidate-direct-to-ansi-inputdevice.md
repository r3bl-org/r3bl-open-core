# Refactoring Plan: Consolidate InputResource + PollerBridge → PollerThreadState

## Status: COMPLETE

All phases completed successfully. Quality checks passed.

## Current Architecture (Before)

```
input_device_impl.rs:
├── InputResource (struct)                    ← REMOVED
│   ├── thread_to_singleton_bridge: Arc<PollerBridge>
│   └── waker: Arc<mio::Waker>
├── global_input_resource (mod)
│   └── SINGLETON: Mutex<Option<InputResource>>
└── InputDeviceSubscriptionHandle (struct)
    ├── maybe_poller_rx: Option<PollerEventReceiver>
    └── mio_poller_thread_waker: Arc<mio::Waker>  ← CHANGED

poller_bridge.rs:                             ← RENAMED FILE
├── THREAD_GENERATION: AtomicU8
├── CHANNEL_CAPACITY: usize
├── PollerBridge (struct)                     ← RENAMED
│   ├── broadcast_tx: PollerEventSender
│   └── thread_liveness: Arc<ThreadLiveness>  ← REMOVED Arc
├── ThreadLiveness (struct)
└── ShutdownDecision (enum)

poller_thread.rs:
└── MioPollerThread (struct)
    └── state: Arc<PollerBridge>              ← RENAMED field
```

## Target Architecture (After)

```
poller_thread_state.rs (renamed from poller_bridge.rs):
├── THREAD_GENERATION: AtomicU8
├── CHANNEL_CAPACITY: usize
├── PollerThreadState (struct)                ← CONSOLIDATES InputResource + PollerBridge
│   ├── broadcast_tx: PollerEventSender
│   ├── thread_liveness: ThreadLiveness       ← inline (no Arc)
│   └── waker: mio::Waker                     ← moved from InputResource (no Arc)
├── ThreadLiveness (struct)                   ← unchanged
└── ShutdownDecision (enum)                   ← unchanged

input_device_impl.rs:
├── global_input_resource (mod)
│   └── SINGLETON: Mutex<Option<Arc<PollerThreadState>>>  ← simplified
└── InputDeviceSubscriptionHandle (struct)
    ├── maybe_poller_rx: Option<PollerEventReceiver>
    └── thread_state: Arc<PollerThreadState>  ← replaces mio_poller_thread_waker

poller_thread.rs:
└── MioPollerThread (struct)
    └── thread_state: Arc<PollerThreadState>  ← renamed from 'state'
```

## Why This Is Better

1. **Single source of truth**: All thread-related shared state in one struct
2. **Simpler Arc structure**: One Arc wrapping everything vs nested Arcs
3. **No `InputResource` wrapper**: Eliminated unnecessary indirection
4. **Better naming**: `PollerThreadState` describes what it IS, not what it connects
5. **Documentation consolidation**: Race condition docs, waker docs, lifecycle docs all in one place

## Documentation Strategy

### Key Insight: PollerBridge docs are already comprehensive

The `PollerBridge` struct docs (lines 37-196) are excellent and cover:
- Thread Lifecycle Overview (5-step sequence)
- The Inherent Race Condition (with timeline diagram)
- What Happens If We Exit Blindly (with zombie scenario diagram)
- Why Thread Reuse Is Safe (with safety table)
- Broadcast Channel Decoupling
- Related Tests (with test table)

**These stay exactly where they are** — just rename `PollerBridge` → `PollerThreadState`.

### What moves from InputResource

Only two sections need to move from `InputResource` to the new `waker` field:

1. **"# Waker Coupled To Poll"** (lines 50-64)
   - Explains Poll→Registry→Waker OS-level bond
   - ASCII diagram showing the chain
   - Why slow path replaces both together

2. **"# Why Waker Is Not Passed to the Thread"** (lines 66-79)
   - Thread only responds to wake events
   - Singleton is "distribution point" for waker clones

### What gets removed (no longer needed)

- `InputResource` intro (lines 34-48) — struct is gone
- `thread_to_singleton_bridge` field docs (lines 100-112) — field is gone
- `waker` field docs (lines 114-123) — replaced with moved sections above

### Documentation Merge Result

```rust
pub struct PollerThreadState {
    /// Broadcasts parsed input events to async subscribers.
    pub broadcast_tx: PollerEventSender,

    /// Thread liveness and incarnation tracking.
    pub thread_liveness: ThreadLiveness,

    /// Waker to signal thread shutdown.
    ///
    /// # Waker Coupled To Poll
    /// [MOVED FROM InputResource - lines 50-64]
    ///
    /// # Why Waker Is Not Passed to the Thread
    /// [MOVED FROM InputResource - lines 66-79]
    pub waker: mio::Waker,
}
```

### Module-Level Docs to Update

- `input_device_impl.rs`: Update container/payload to reference `Arc<PollerThreadState>` instead of `InputResource`
- `poller_thread_state.rs`: Minor update to module docs (rename mentions)

---

## Implementation Phases

### Phase 1: Rename and restructure poller_bridge.rs → poller_thread_state.rs [COMPLETE]

**File:** `mio_poller/poller_bridge.rs` → `mio_poller/poller_thread_state.rs`

**Changes:**

1. [x] Rename file: `poller_bridge.rs` → `poller_thread_state.rs`
2. [x] Rename struct: `PollerBridge` → `PollerThreadState`
3. [x] Add field: `waker: mio::Waker`
4. [x] Change field: `thread_liveness: Arc<ThreadLiveness>` → `thread_liveness: ThreadLiveness`
5. [x] Update `new()` signature: `fn new(waker: mio::Waker) -> Self`
6. [x] Merge documentation from `InputResource`:
   - "Waker Coupled To Poll" → `waker` field docs
   - "Why Waker Is Not Passed to the Thread" → struct-level docs
7. [x] Update all internal doc links
8. [x] Update module-level docs

### Phase 2: Update mio_poller/mod.rs [COMPLETE]

**Changes:**

1. [x] `pub use poller_bridge::*` → `pub use poller_thread_state::*`
2. [x] Update any `mod poller_bridge` → `mod poller_thread_state`

### Phase 3: Update input_device_impl.rs - Remove InputResource [COMPLETE]

**Changes:**

1. [x] Remove `InputResource` struct entirely
2. [x] Update `SINGLETON` type:
   ```rust
   // Before
   pub static SINGLETON: Mutex<Option<InputResource>> = Mutex::new(None);
   // After
   pub static SINGLETON: Mutex<Option<Arc<PollerThreadState>>> = Mutex::new(None);
   ```
3. [x] Update module-level docs (lines 6-26):
   - Change "payload" from `InputResource` to `Arc<PollerThreadState>`
   - Update module contents list
4. [x] Move `InputResource` docs to `PollerThreadState` or `SINGLETON`

### Phase 4: Update allocate() function [COMPLETE]

**Changes:**

```rust
// Before (slow path)
let bridge = Arc::new(PollerBridge::new());
MioPollerThread::new(new_poll, Arc::clone(&bridge));
guard.replace(InputResource {
    thread_to_singleton_bridge: bridge,
    waker: Arc::new(new_waker),
});

// After (slow path)
let thread_state = Arc::new(PollerThreadState::new(new_waker));
MioPollerThread::new(new_poll, Arc::clone(&thread_state));
guard.replace(thread_state);
```

```rust
// Before (creating handle)
InputDeviceSubscriptionHandle {
    maybe_poller_rx: Some(input_resource_state.thread_to_singleton_bridge.broadcast_tx.subscribe()),
    mio_poller_thread_waker: Arc::clone(&input_resource_state.waker),
}

// After (creating handle)
InputDeviceSubscriptionHandle {
    maybe_poller_rx: Some(thread_state.broadcast_tx.subscribe()),
    thread_state: Arc::clone(&thread_state),
}
```

1. [x] Update slow path: create `PollerThreadState` with waker
2. [x] Update `guard.replace()` call
3. [x] Update fast path check: `thread_state.thread_liveness.is_running()` (no `.thread_to_singleton_bridge`)
4. [x] Update `InputDeviceSubscriptionHandle` creation
5. [x] Update all doc references in function

### Phase 5: Update InputDeviceSubscriptionHandle [COMPLETE]

**Changes:**

1. [x] Change field:
   ```rust
   // Before
   pub mio_poller_thread_waker: Arc<mio::Waker>,
   // After
   pub thread_state: Arc<PollerThreadState>,
   ```
2. [x] Update `Drop` impl:
   ```rust
   // Before
   let wake_result = self.mio_poller_thread_waker.wake();
   // After
   let wake_result = self.thread_state.waker.wake();
   ```
3. [x] Update field docs

### Phase 6: Update MioPollerThread [COMPLETE]

**File:** `mio_poller/poller_thread.rs`

**Changes:**

1. [x] Rename field: `state` → `thread_state`
2. [x] Update all usages of `self.state` → `self.thread_state`
3. [x] Update `new()` parameter name
4. [x] Update docs

### Phase 7: Update other consumers [COMPLETE]

**Files to check:**

1. [x] `handler_receiver_drop.rs` - uses `PollerBridge`
2. [x] `handler_stdin.rs` - may reference types
3. [x] `handler_signals.rs` - may reference types
4. [x] `input_device_public_api.rs` - imports

### Phase 8: Fix all doc links [COMPLETE]

**After structural changes, fix broken links:**

1. [x] `InputResource` → `PollerThreadState`
2. [x] `PollerBridge` → `PollerThreadState`
3. [x] `thread_to_singleton_bridge` → `thread_state`
4. [x] `mio_poller_thread_waker` → `thread_state.waker`

### Phase 9: Quality checks [COMPLETE]

1. [x] `cargo check` - ✅ Passed
2. [x] `cargo build` - ✅ Passed
3. [x] `cargo rustdoc-fmt` - ✅ Passed (1 file modified)
4. [x] `cargo doc --no-deps` - ✅ Passed (0 warnings)
5. [x] `cargo clippy --all-targets` - ✅ Passed (0 warnings)
6. [x] `cargo test --no-run` - ✅ Passed
7. [x] `cargo test --all-targets` - ✅ Passed (2,770+ tests)
8. [x] `cargo test --doc` - ✅ Passed (312 doctests)

---

## Risk Assessment

### Low Risk
- Renaming is mechanical
- Arc restructuring is straightforward

### Medium Risk
- Documentation consolidation - easy to miss links
- Multiple files to update atomically

### Mitigation
- Incremental commits after each phase
- Run quality checks frequently
- Use grep to find all references before each phase

---

## Final Summary

| Phase | Files Changed | Status |
|-------|---------------|--------|
| 1 | 1 (poller_thread_state.rs) | ✅ COMPLETE |
| 2 | 1 (mio_poller/mod.rs) | ✅ COMPLETE |
| 3 | 1 (input_device_impl.rs) | ✅ COMPLETE |
| 4 | 1 (input_device_impl.rs) | ✅ COMPLETE |
| 5 | 1 (input_device_impl.rs) | ✅ COMPLETE |
| 6 | 1 (poller_thread.rs) | ✅ COMPLETE |
| 7 | 4 (handler files) | ✅ COMPLETE |
| 8 | 1 (input_device_public_api.rs) | ✅ COMPLETE |
| 9 | 0 (verification) | ✅ COMPLETE |

**Total: 8 files changed**

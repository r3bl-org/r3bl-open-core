<!-- cspell:words EBUSY panicker ONLCR SIGABRT errno -->

# Task: Fix RRT with Sane Types (Typestate Pattern)

## Overview

The `RRT` framework currently manages thread liveness and event broadcast through
decoupled sources of truth (`SharedWakerSlot` and the `tokio` channel's atomic
`receiver_count`). This decoupling creates transient states where one source says "alive"
while the other says "dead", resulting in several race conditions (The Exit Decision Race,
The Fast-Path Race, and The Zombie Waker Bug).

This task implements a robust **Typestate Pattern** for RRT. By combining a 5-variant
`ThreadState` enum (`Stopped`, `Starting`, `Running`, `Stopping`, `Restarting`) behind a
single `Mutex` + `Condvar` monitor, we make these race conditions structurally impossible
to represent.

## Implementation plan

### Phase 1: Implement `ThreadState` and `ThreadLifecycleMonitor` [COMPLETE]

- [x] Create `ThreadState<W: RRTWorker>` enum with variants: `Stopped`, `Starting`,
      `Running(WakerHandle<W::Waker>)`, `Stopping(StopReason)`, `Restarting`.
- [x] Create `ThreadLifecycleMonitor<W: RRTWorker>` struct holding `Mutex<ThreadState<W>>`
      and `Condvar`.
- [x] Create `WakerHandle<W: RRTWaker>` wrapper that **deliberately does not implement
      `Clone` or `Copy`**, ensuring the waker can never escape the `Running` state as a
      stale copy.
- [x] Update `RRT<W>` to use `Arc<ThreadLifecycleMonitor<W>>` for its `shared_state`
      field.

### Phase 2: Refactor `run_worker_loop()` and recovery paths [COMPLETE]

- [x] Update `run_worker_loop()` to use the monitor for all state transitions.
- [x] **Exit Decision Serialization:** Implement the "is anyone still here?" check by
      locking `shared_state.state` at the top of the loop. If `receiver_count == 0`,
      transition to `Stopping` and exit. This serializes the exit decision with new
      subscribers.
- [x] **RAII Termination Guard:** Implement a `TerminationGuard` that, on `Drop`, sets the
      state to `Stopped`, preventing `EBUSY` when a new thread immediately spawns.
- [x] **Restart Logic:** Update `Continuation::Restart` to:
  1. Lock state and transition from `Running` to `Restarting`.
  2. **Drop the lock and drop the old `worker`** (via `maybe_worker.take()`) to release OS
     handles.
  3. **Sleep for the backoff delay** (see [`RestartPolicy`]). This is crucial to prevent
     "resource busy" loops and give OS handles (like sockets or ports) time to fully
     release.
  4. Call `W::create_and_register_os_sources()`.
  5. Lock state, transition to `Running(WakerHandle::new(waker))`, and call
     `shared_state.condvar.notify_all()`.
- [x] **Panic Handling:** The framework catches panics from
      `block_until_ready_then_dispatch()` using `catch_unwind`. Since `sender` is still
      passed/global, `Shutdown(Panic)` can still be sent exactly as it is today. The RAII
      guard will catch the current state and reliably set `Stopped` + notify.

### Phase 3: Update `subscribe()` and public API [COMPLETE]

- [x] Rewrite `RRT::try_subscribe()` to lock `shared_state` and match explicitly on the
      states in a `loop`.
- [x] **Fast Path Serialization:** In the `Running` arm, explicitly call
      `sender.subscribe()` to increment the receiver count **while still holding the
      `shared_state` lock**, then release the lock. This serializes the subscriber's "I'm
      here!" increment with the framework's "is anyone still here?" check, structurally
      eliminating the exit decision race.
- [x] **Safe OS Allocation & Spawn:** In the `Stopped` arm, transition to `Starting`, drop
      the lock, allocate OS resources. If successful:
  1. **Lock the state again.**
  2. **Call `sender.subscribe()`** to create the receiver and increment the count.
  3. **Increment the `thread_generation` counter** (using `AtomicU8Ext::increment()`).
  4. **Set state to `Running(WakerHandle::new(waker))`.**
  5. **Spawn the thread while still holding the lock.**
  6. **Call `notify_all()` and then release the lock.** This sequence ensures that the
     "State is Running" and "Thread exists" conditions are perfectly atomic. If OS
     allocation fails, revert state to `Stopped` and notify all. **If OS allocation
     succeeds but thread spawn fails**, drop the worker/waker and the receiver (freeing
     resources), revert state to `Stopped`, notify all, and return the `ThreadSpawn`
     error. (Any subscribers who joined via the fast path in the tiny window between
     setting `Running` and the spawn failure will see `Stopped` on their next check and
     safely no-op).
- [x] Handle transitional states (`Starting`, `Stopping`, `Restarting`) in `subscribe()`
      by blocking on the `Condvar` (`shared_state.condvar.wait()`) until the state
      changes.
- [x] Update `subscribe_to_existing()`: change signature to return
      `Option<SubscriberGuard<W>>`. It must be **Condvar-aware** (Option A): 1. Lock the
      state. 2. If state is `Starting`, `Stopping`, or `Restarting`, block on the
      `Condvar` and loop. 3. If state is `Running`, subscribe to the global `sender` and
      return `Some`. 4. If state is `Stopped`, return `None` (preserving the "don't spawn"
      contract). **Rustdoc Requirement:** Explicitly document that this is for
      **Observers** who only want to join if a thread is already alive. Note that it will
      block on transient states but return `None` immediately if the thread is truly dead.
- [x] **Input Device (`input_device_public_api.rs`):** Update
      `DirectToAnsiInputDevice::subscribe()`: replace
      `global_input_resource::SINGLETON.subscribe_to_existing().expect(...)` with
      `global_input_resource::SINGLETON.try_subscribe().expect(...)`. This ensures that
      even if the thread has crashed (`Stopped`), the primary input device can relaunch
      it. If it is in a transient state, it will safely block. **Rustdoc Requirement:**
      Explicitly document that this is for **Primary Stakeholders**. Explain the recovery
      guarantee: it uses the slow-path to relaunch the input thread if it has crashed,
      ensuring the device remains functional.
- [x] Delete `is_thread_running()`.
- [x] `get_receiver_count()`: unchanged, continues to use `self.sender.receiver_count()`.

### Phase 4: Update guards, worker implementations, and tests [COMPLETE]

- [x] Update `SubscriberGuard` and `WakeOnDrop` to hold the
      `Arc<ThreadLifecycleMonitor<W>>`.
- [x] **Field Drop-Order Invariant:** Ensure that in the `SubscriberGuard` struct
      definition, the **Receiver** field is declared **above** the **`WakeOnDrop`** guard
      field. Rust drops fields in the order they are declared, and we need the receiver to
      drop first to decrement the count before the final wake check.
- [x] Make `WakeOnDrop::drop()` invoke the waker only if the state is `Running` (this is
      safe to do while locking `shared_state.state`).
- [x] Strip out manual `receiver_count == 0` checks from the receiver-drop wake path
      (e.g., `handle_receiver_drop_waker_with_sender`).
- [x] Ensure the dispatch logic handles the `ReceiverDropWaker` mio token by returning
      `Continuation::Continue`. Also update send-failure handlers (e.g.,
      `sender.send().is_err()` in `handler_signals.rs` and `handler_stdin.rs`) to return
      `Continuation::Continue` instead of `Continuation::Stop`, letting the framework
      catch `receiver_count == 0` on the next loop iteration.
- [x] **Test Removals:** Delete the following tests from
      `process_isolated_tests/group_b_run_worker_loop.rs` — they directly test
      `TerminationGuard` clearing the waker to `None`, which no longer exists. Their
      equivalent behavior is covered by the new RAII guard unit tests below:
  - `test_guard_clears_waker_on_stop()`
  - `test_guard_clears_waker_on_exhaustion()`
  - `test_guard_clears_waker_on_panic()`
- [x] **Test Migrations:** Update all integration, process-isolated, and unit tests
      (including `rrt_subscriber_guard::drop_order_tests` and `mod::unit_tests`). The
      tests do not need to reproduce the race conditions since they are structurally
      impossible now. Instead:
  1. Align the tests with the new `ThreadState` access patterns (constructing
     `Arc<ThreadLifecycleMonitor<W>>` instead of `SharedWakerSlot`).
  2. Rewrite test assertions: replace checks like `is_none()` on `SharedWakerSlot` with
     explicit matches on `ThreadState::Stopped` or `ThreadState::Running`.
  3. Replace `std::thread::sleep` delays (used to wait for threads to exit) with efficient
     `Condvar` waits for the state to reach `Stopped` or `Running`.
- [x] **New State Machine Unit Tests:** Add deterministic (non-flaky, no concurrency
      stress) unit tests for the new behavior introduced by the 5-state machine:
  1. **State transitions:** Test each transition path: `Stopped->Starting->Running`
     (normal subscribe), `Running->Stopping->Stopped` (framework-initiated,
     receiver_count==0), `Running->Stopping->Stopped` (worker-initiated Stop/EOF),
     `Running->Restarting->Running` (successful restart), `Restarting->Stopped` (restart
     budget exhausted, via RAII guard), `Running->Stopped` (panic, via RAII guard),
     `Restarting->Stopped` (panic during restart, via RAII guard), `Starting->Stopped` (OS
     allocation failure), `Starting->Stopped` (thread spawn failure after successful
     allocation).
  2. **RAII guard:** Test the guard's `Drop` in isolation — verify it transitions
     `Stopping->Stopped`, `Running->Stopped`, and `Restarting->Stopped`, and calls
     `notify_all()` in all cases.
  3. **Condvar notifications:** Verify that a thread blocked on `condvar.wait()` wakes
     when the state transitions through `Starting->Running`, `Starting->Stopped`
     (failure), `Stopping->Stopped`, and `Restarting->Running`. A missed `notify_all()`
     causes a permanent hang, so these catch liveness bugs.
  4. **WakeOnDrop state check:** Verify `WakeOnDrop::drop()` invokes the waker when state
     is `Running`, and no-ops for `Stopped`, `Starting`, `Stopping`, and `Restarting`.
  5. **`subscribe_to_existing()` rejection:** Verify it returns `None` for `Stopped`,
     `Starting`, `Stopping`, and `Restarting`, and `Some` only for `Running`.
  6. **Error recovery:** Verify that after OS allocation failure in the `Stopped` branch,
     the state reverts to `Stopped`, waiters are notified, and a subsequent `subscribe()`
     call can succeed.
- [x] Run `./check.fish --test` to ensure all tests pass and no race conditions remain.

### Phase 5: Land Shadow Sources [COMPLETE]

- [x] **Final Doc Landing:** Once Phases 1-4 are implemented and verified via tests,
      replace the contents of the following `.rs` files with the verified content from
      their respective shadow source files:
  1. `tui/src/core/resilient_reactor_thread/mod.rs` (from
     `task/fix-rrt-with-sane-types/mod.md`)
  2. `tui/src/core/resilient_reactor_thread/rrt.rs` (from
     `task/fix-rrt-with-sane-types/rrt.md`)
  3. `tui/src/core/resilient_reactor_thread/rrt_worker.rs` (from
     `task/fix-rrt-with-sane-types/rrt_worker.md`)
  4. `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs` (from
     `task/fix-rrt-with-sane-types/rrt_subscriber_guard.md`)
  5. `tui/src/core/resilient_reactor_thread/rrt_types.rs` (from
     `task/fix-rrt-with-sane-types/rrt_types.md`)
- [x] Run `cargo rustdoc-fmt` on all modified files to ensure documentation formatting is
      consistent.
- [x] **Cleanup:** Delete the `task/fix-rrt-with-sane-types/` directory and its shadow
      files. (Completed by user during implementation).
- [x] **Code Cleanup:** Sweep all RRT-related files (especially `rrt.rs`) for internal
      migration notes, `TODO`s, and `NOTE` blocks that were used to track the refactor
      (e.g., `// NOTE: is_thread_running() DELETED.`). Remove them to leave the final code
      clean.
- [x] Run `./check.fish --full` to verify the final state.

### Phase 6: Architectural Refinement (Handle-driven & Fallible API) [COMPLETE]

- [x] **SubscriberGuard (Self-Replicating Handle):** Update `rrt_subscriber_guard.rs`: 1.
      Add `sender: BroadcastSender<W::Event>` field (prefer private / `pub(crate)`, not
      `pub`, unless external access is required). 2. Preserve the **Drop Order
      Invariant**: `receiver` stays declared above `wake_on_drop`. 3. **Implement
      `try_subscribe(&self)`** returning `Result<Self, SubscribeError>` (self-replication
      path). 4. ~~Implement `subscribe_to_existing(&self)`~~ (Deleted in Phase 11 - zero
      call sites). 5. Update `SubscriberGuard::new()` to accept the `sender` and
      `receiver` (allocated under lock).
- [x] **Single Subscribe Engine (RRT Layer):** Update `rrt.rs`: 1. Keep one private
      state-machine subscribe helper (single source of truth for lock/condvar logic). 2.
      Change `RRT::try_subscribe()` to return
      `Result<SubscriberGuard<W>, SubscribeError>`. 3. Make both `RRT::try_subscribe()`
      and `SubscriberGuard::try_subscribe()` delegate to that same helper.
- [x] **Fallible API Migration (Input Device Layer):** Update
      `input_device_public_api.rs`: 1. **`DirectToAnsiInputDevice::new()`**: Change
      signature to return `Result<Self, SubscribeError>`. 2.
      **`DirectToAnsiInputDevice::subscribe()`**: Change signature to return
      `Result<InputSubscriberGuard, SubscribeError>`. 3. **Document the "Total Honesty"
      Policy**: Explicitly state that these methods are fallible to reflect OS resource
      constraints (catastrophes), even if callers choose `.expect()`. 4. If external
      breakage risk is high, stage with `try_new()` / `try_subscribe()` before
      hard-switching signatures.
- [x] **Handle Type Promotion + Explicit Validation:** Ensure all integration tests and
      examples treat `SubscriberGuard` (or `InputSubscriberGuard`) as the primary
      subscription capability, and add explicit checks for: 1. `guard.try_subscribe()`
      peer-replication behavior. 2. `guard.subscribe_to_existing()` observer behavior. 3.
      Fallible-path propagation from RRT layer to input-device API.
- [x] Run `./check.fish --check`, `./check.fish --test`, and `./check.fish --clippy`, then
      `./check.fish --full` to verify the final refined state.

### Phase 8: Module Refactoring and Final Documentation [COMPLETE]

- [x] **Engine Extraction:** Create `rrt_engine.rs` and move `run_worker_loop()` and
      `advance_backoff_delay()` from `rrt.rs`.
- [x] **Type Migration:** Move `RetryLoopExhaustion` from `rrt.rs` to `rrt_types.rs`.
      (Actually verified it is already in `rrt_types.rs`, so I will just verify its
      documentation).
- [x] **Distributed State Machine Documentation:**
  - [x] Add "The Distributed State Machine" section to `mod.rs` mapping files to roles
        (Driver, Engine, Controller, States, Signal).
  - [x] Add "Static vs. Flexible Usage" section to `mod.rs` explaining the `LazyLock`
        behavior.
- [x] **Entry Point Refactoring:**
  - [x] Clean up `rrt.rs` to contain only the `RRT` struct and public API.
  - [x] Update `RRT` struct documentation to link to the new architectural sections in
        `mod.rs`.
- [x] **Consistency and Validation:**
  - [x] Standardize breadcrumbs across all module files.
  - [x] Group re-exports in `mod.rs` into "Public API" and "Internal Implementation".
  - [x] Run `./check.fish --check` and verify all intra-doc links.

### Phase 10: Observability and Logging [COMPLETE]

- [x] **Add Debug Flag**: Add `DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD` constant to
      `tui/src/tui/mod.rs`.
- [x] **Implement Lifecycle Logging**: Add `tracing::debug!` or `tracing::info!` for key
      RRT transitions (spawn, exit, restarts) in `rrt.rs` and `rrt_engine.rs`.
- [x] **Implement Error Logging**: Add `tracing::error!` for critical failures (OS
      resource allocation, thread spawn, budget exhaustion).
- [x] **Drop-path poison logging**: In `TerminationGuard::drop()` and
      `WakeOnDrop::drop()`, emit `tracing::error!` on `MutexPoisoned` without
      re-panicking. Drop impls honor the "poison = process terminates" policy by logging,
      not by panicking again (the originating panicker already owned termination; a
      drop-path re-panic risks double-panic-during-unwind aborts).
- [x] **Conditional Execution**: Ensure all logging uses the `.then(|| { ... })` pattern
      with the debug flag.
- [x] **Check Code Quality**: Run `./check.fish --full` to ensure everything is correct.

### Phase 11: Cleanup: API surface reduction [COMPLETE]

- [x] **Delete Dead Code**: Remove `RRT::subscribe_to_existing()` and
      `SubscriberGuard::subscribe_to_existing()` as they have zero call sites.
- [x] **Simplify API**: Prefer `try_subscribe()` for all use cases, reducing the potential
      for `Option`-based error conflation and `panic!` on poisoned mutexes.
- [x] **Update Documentation**: Remove all internal and external references to the deleted
      methods.

### Phase 12: Monitor API Refinement (By-Value MutexGuard) [COMPLETE]

- [x] **Update `update_state` Signature:** Change `update_state` in `rrt_monitor.rs` to
      take the `MutexGuard` by value and return it by value
      (`pub fn update_state<'this>(..., guard: MutexGuard<'this, ...>) -> MutexGuard<'this, ...>`).
- [x] **Enforce Lock Chain-of-Custody:** This by-value pattern ensures consistency with
      the Monitor's `wait` and `wait_until` methods, and structurally prevents logic
      errors by creating a linear "chain of custody" for the lock. The caller must
      explicitly re-assign or drop the returned guard.
- [x] **Update Call Sites:** Refactor all calls in `rrt.rs`, `rrt_engine.rs`, and
      `rrt_termination_guard.rs` to follow the
      `state_guard = monitor.update_state(state_guard, ...)` pattern.
- [x] **Verify Consistency:** Ensure the use of the `'this` lifetime naming convention
      across the `ThreadLifecycleMonitor` API for better readability and alignment with
      existing patterns.
- [x] **Final Validation:** Run `./check.fish --test` to ensure the new API is working
      correctly across all state machine transitions.

### Phase 13: Symmetrical Monitor API (Friction-as-a-Feature) [COMPLETE]

- [x] **Enhance `Monitor` Primitives:**
  - [x] Add `set_state(guard, new_state) -> MutexGuard`: Direct value replacement.
  - [x] Add `update_state(guard, FnOnce(&mut State)) -> MutexGuard`: In-place mutation via
        closure.
  - [x] Add `read_state(guard, FnOnce(&State) -> R) -> (R, MutexGuard)`: Read-only access
        via closure.
  - [x] **Planned Implementation (`Monitor<State>`):**
  - [x] **Rationale:** Using `FnOnce` for single-call methods provides maximum
        flexibility, while `FnMut` is preserved for `wait_until` due to potential spurious
        wakeups.
- [x] **Refine `ThreadLifecycleMonitor` API:**
  - [x] Rename existing `update_state` to `set_state` (more accurate semantics).
  - [x] Update `set_state` to delegate to `monitor.set_state` while preserving RRT
        transition logging.
  - [x] Add closure-based `update_state` that delegates to `monitor.update_state`.
  - [x] Add closure-based `read_state` that delegates to `monitor.read_state`.
  - [x] **Planned Implementation (`ThreadLifecycleMonitor<W>`):**
  - [x] **Lock Chain-of-Custody:** All these methods follow the "take by value, return by
        value" pattern to structurally prevent stale guard usage and deadlocks.
- [x] **Detailed Migration Plan:**
  - [x] **`update_state` (replacement) → `set_state`**:
    - [x] `rrt.rs`: Lines 58, 72, 95, 139.
    - [x] `rrt_engine.rs`: Lines 130, 157, 172, 282.
    - [x] `rrt_termination_guard.rs`: Line 42.
    - [x] `rrt_monitor.rs`: Doc examples in `update_state` (lines 274, 284).
  - [x] **`lock` + direct read → `read_state` (closure-based)**:
    - [x] `rrt_monitor.rs`: `wake_if_running()` — Move the
          `if let ThreadState::Running(...)` check into a `read_state` closure.
    - [x] `rrt_engine.rs`: Line 121 — Move the `if let ThreadState::Running(...)` and
          `receiver_count == 0` logic into a `read_state` closure to determine if shutdown
          is needed.
    - [x] `rrt.rs`: Line 45 — Refactor the `match *state_guard` in `try_subscribe` to use
          `read_state` to determine the next action (subscribe-fast-path vs. block-wait
          vs. spawn-slow-path).
  - [x] **Closure-based `update_state` usage**:
    - [x] Identify any nested mutations that can be cleaned up (currently most RRT state
          transitions replace the entire enum variant, so `set_state` is the primary tool,
          but `update_state` is available for future-proofing).
- [x] **Verification:**
  - [x] Run `./check.fish --test` to ensure all state machine transitions remain robust.
  - [x] Verify that the added "friction" in the API leads to cleaner, more auditable code
        in the RRT engine.

### Phase 14: Poison-Safe Terminal Cleanup (Drop-Safety Focus) [COMPLETE]

#### Overview

In a TUI environment, the highest risk is the "Double Panic Abort." This happens when an
original panic triggers a `drop()` implementation that then panics again (usually via
`lock().unwrap()` on a poisoned mutex). This aborts the process and bricks the terminal
(leaving it in raw mode).

**The Focus**: We don't need to eliminate `lock().unwrap()` everywhere. In active paths
(like `RRT::try_subscribe`), a panic on poisoning is acceptable as the "engine" is broken.
However, we must ensure that the **Cleanup Path** (specifically `Drop` and anything it
calls) is resilient to poisoning so it can always attempt terminal restoration.

#### Phase 14.1: Architectural Formalization

- [x] Update `.agent/skills/concurrency-safety/patterns.md` to document the "Drop-Safety"
      mandate.
  - Normal Paths: `lock().unwrap()` (fail fast).
  - Cleanup/Drop Paths: `lock().unwrap_or_else(|e| e.into_inner())` (never abort).
- [x] Add a section to `tui/src/core/terminal_io/output_device.rs` explaining the
      importance of poison-safe locking for cleanup.

#### Phase 14.2: Resilient Restoration Implementation

- [x] `tui/src/core/common/monitor.rs` and
      `tui/src/core/resilient_reactor_thread/rrt_monitor.rs`:
  - Simplify existing methods to use `unwrap()`.
  - Introduce `lock_raw()`.
- [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`: Refactor
      `disable_raw_mode()`.
- [x] `tui/src/core/terminal_io/output_device.rs`: Refactor `lock()`.
- [x] `tui/src/readline_async/readline_async_impl/readline.rs`: Refactor `Drop` impl.
- [x] `tui/src/core/resilient_reactor_thread/rrt_monitor.rs`: Refactor
      `wake_if_running()`. _(Note: Simplified to return `()` and handle poisoning/logging
      internally to ensure infallible cleanup)_.

#### Phase 14.3: Validation

##### 1. Unit Test for `SAVED_TERMIOS` Recovery

- [x] **Scenario**: Intentionally poison the `SAVED_TERMIOS` mutex by panicking in a
      thread that holds the lock.
- [x] **Verification**: Call `disable_raw_mode()` and verify:
  - It does not return an error (it uses `into_inner()`).
  - It correctly resets `SAVED_TERMIOS` to `None`.
  - Future calls to `enable_raw_mode()` succeed without seeing a poisoned lock. (Verified
    in `raw_mode_unix.rs`).

- [x] **Unit Test for generic `Monitor` Poisoning Recovery** (added to `monitor.rs`).

##### 2. `OutputDevice` Poison Resilience Test

- [x] **Scenario**: Use a custom `OutputDevice`. Poison the internal resource mutex.
- [x] **Verification**: Call `lock()` and verify it returns a valid `MutexGuard`
      (containing the dirty state) instead of panicking. (Verified in `output_device.rs`).

##### 3. "Double Panic Abort" Prevention (Integration Test)

- [x] **Scenario**: Create a test that:
  1. Enables raw mode.
  2. Acquires the `safe_line_state` lock in `Readline`.
  3. Triggers a panic (using `std::panic::catch_unwind` for the outer scope).
- [x] **Verification**: Ensure the `drop(readline)` completes and calls
      `disable_raw_mode()`. We can verify this by checking if the mock terminal received
      the exit sequences or by checking a mock raw mode provider. (Verified in
      `double_panic_prevention_test.rs`).

##### 4. Diagnostic Verification

- [x] **Scenario**: Simulate a panic that poisons an RRT lock.
- [x] **Verification**: Ensure that the `miette` diagnostic report (if one is being
      generated) is printed to the screen and is not swallowed by a secondary panic during
      the output device's final flush. (Inherent in poison-safe restoration verified
      above).

##### 5. Comprehensive Quality Check

- [x] Run `./check.fish --full` to ensure no regressions in existing RRT, Readline, or
      Terminal tests.

#### Phase 14.4: High-Level Architectural Documentation

- [x] Consolidate resilience patterns in `tui/src/lib.rs` (crate root) under the heading
      "Terminal Restoration: Panic, Drop, and Mutex Poison-Safety".
  - [x] Explicitly define **Normal Paths (Fail-Fast)** vs. **Cleanup Paths
        (Poison-Safe)**.
  - [x] Provide compilable `no_run` examples for both locking strategies.
  - [x] Expanded the "Double Panic Abort" scenario with a concrete **Thread A** narrative.
- [x] Add cross-reference links from all key poison-safe components back to the crate root
      section:
  - `SAVED_TERMIOS` (static)
  - `OutputDevice` (struct and `lock()`)
  - `Readline` (struct and `drop()`)
  - `Monitor` (struct and `lock_raw()`)
  - `ThreadLifecycleMonitor` (struct and `lock_raw()`)

#### Phase 14.5: Documentation & Terminology Standardization

- [x] Standardized all `# Poison Safety` headings across the codebase for searchability.
- [x] Standardized `# Panics` sections to consistently link back to the central Terminal
      Restoration documentation.
- [x] Implemented a consistent **Fail-Fast** doc pattern using dual headings (`# Panics` +
      `# Poison     Safety`) for all normal paths.
- [x] Performed a surgical audit to remove accidental "air gaps" (blank lines) within
      `///` blocks and ensure co-location between doc comments, attributes, and function
      definitions.
- [x] Verified that **Cleanup Paths** (e.g., `OutputDevice::lock()`, `Drop` impls) do not
      erroneously claim to panic on poisoning.
- [x] Standardized core terminology to use **bold** consistently:
  - **Double Panic Abort**
  - **Resilience over Integrity**
  - **brick the user's terminal**
- [x] Restored missing technical prose in `rrt_subscriber_guard.rs` linking `drop()` to
      `mio::Poll` wake-ups.
- [x] Verified code quality using `./check.fish --full`.

##### Final Verification Results

| Component              | File                       | implementation | # Panics | # Poison Safety | Gaps Removed | Co-located |
| :--------------------- | :------------------------- | :------------- | :------: | :-------------: | :----------: | :--------: |
| `OutputDevice::lock()` | `output_device.rs`         | Poison-Safe    |   N/A    |       ✅        |      ✅      |     ✅     |
| `RawModeGuard::drop()` | `raw_mode_core.rs`         | Poison-Safe    |   N/A    |       ✅        |      ✅      |     ✅     |
| `Readline::drop()`     | `readline.rs`              | Poison-Safe    |   N/A    |       ✅        |      ✅      |     ✅     |
| `TerminationGuard`     | `rrt_termination_guard.rs` | Poison-Safe    |   N/A    |       ✅        |      ✅      |     ✅     |
| `SubscriberGuard`      | `rrt_subscriber_guard.rs`  | Poison-Safe    |   N/A    |       ✅        |      ✅      |     ✅     |
| `Monitor::lock()`      | `monitor.rs`               | Fail-Fast      |    ✅    |       ✅        |      ✅      |     ✅     |
| `Monitor::wait()`      | `monitor.rs`               | Fail-Fast      |    ✅    |       ✅        |      ✅      |     ✅     |
| `rrt_monitor::lock()`  | `rrt_monitor.rs`           | Fail-Fast      |    ✅    |       ✅        |      ✅      |     ✅     |
| `run_worker_loop()`    | `rrt_engine.rs`            | Fail-Fast      |    ✅    |       ✅        |      ✅      |     ✅     |
| `Spinner` methods      | `spinner.rs`               | Fail-Fast      |    ✅    |       ✅        |      ✅      |     ✅     |

Table Legend & Key

- Columns
  - **Component**: The specific function, method, or struct being documented.
  - **File**: The source file containing the component.
  - **Implementation**:
  - **Poison-Safe**: Uses `into_inner()` or `lock_raw()` to recover from poisoning without
    panicking. Crucial for cleanup paths.
  - **Fail-Fast**: Uses `.lock().unwrap()` or similar. Intentionally panics on poisoning
    to prevent operating on corrupted state.
  - **# Panics**:
  - **✅**: The doc comment contains a `# Panics` section explaining when and why it
    panics.
  - **N/A**: This component is poison-safe and does not panic on poisoning, so a
    `# Panics` section is omitted to avoid contradiction.
  - **Removed**: An erroneous `# Panics` section was surgically removed because the
    implementation is actually poison-safe.
  - **# Poison Safety**:
  - **✅**: The doc comment contains the standardized `# Poison Safety` heading, linking
    to the central documentation in the crate root.
  - **Gaps Removed**:
  - **✅**: All internal blank lines ("air gaps") within the `///` doc comment blocks have
    been removed for clean rendering.
  - **Co-located**:
  - **✅**: The doc comment block is immediately adjacent to the function signature or
    attribute (no extra blank lines).
- Rows
  - Each row represents a critical point in the terminal restoration architecture where
    mutex poisoning must be handled deliberately.
- Cell Values
  - **✅**: Verified and standardized according to the architectural mandates.
  - **N/A**: Not applicable for this specific implementation type.
  - **Removed**: Corrected a documentation error during the standardization audit.

### Phase 15: Migrate Terminal-Dependent Tests to PTYs [COMPLETE]

#### Overview

Several tests in `tui` use mock I/O devices but still mutate **global terminal state**
(via `disable_raw_mode()` which operates on the `SAVED_TERMIOS` static). This is not a
problem with the mocks themselves - the mock `OutputDevice` and `InputDevice` work
correctly. The problem is that `Readline::drop()` unconditionally calls
`crate::disable_raw_mode()`, which modifies the **test runner process's own terminal
settings** (termios `ONLCR` flag). This causes the "staircase effect" in
`./check.fish --test` output: newlines move the cursor down but not back to column 0.

PTY isolation solves this by giving each test its own pseudoterminal. The test's
`disable_raw_mode()` operates on the child process's TTY, leaving the test runner's
terminal untouched.

#### Phase 15.1: Migrate `double_panic_prevention_test.rs` [COMPLETE]

- [x] Refactor
      `tui/src/core/resilient_reactor_thread/integration_tests/double_panic_prevention_test.rs`:
  - Use `generate_pty_test!`.
  - The `controlled` process runs the double-panic scenario:
    1. Poison the `safe_line_state` mutex in a background thread.
    2. In the main thread, wrap a panic-triggering block in `std::panic::catch_unwind`.
    3. The panic triggers `Readline::drop()`, which attempts to lock the poisoned mutex.
    4. If `drop()` is resilient, `catch_unwind` returns, and the process calls
       `std::process::exit(0)`.
  - The `controller` verifies the child **exited with code 0**:
    - **Exit Code 0**: Success. The poison-safe `drop()` prevented a double-panic.
    - **Exit Code 134 (SIGABRT)**: Failure. A double-panic occurred during unwind, causing
      an immediate abort.
- [x] Add a rustdoc module comment explaining why this test uses PTY isolation:
  - It is **not** because of mock I/O devices (those work fine).
  - It is because `Readline::drop()` calls `disable_raw_mode()`, which mutates the global
    `SAVED_TERMIOS` static and alters the terminal environment of whatever process runs
    it.
  - Without PTY isolation, this corrupts the `cargo test` runner's terminal output.

#### Phase 15.2: Migrate `Readline` Unit Tests to PTY [COMPLETE]

- [x] Refactor unit tests in `tui/src/readline_async/readline_async_impl/readline.rs` that
      call `Readline::try_new()` into a new PTY integration test:
      `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`.
- [x] Tests to migrate:
  1. `test_readline_internal_process_event_and_terminal_output`
  2. `test_editor_state_empty_buffer`
  3. `test_editor_state_with_content`
  4. `test_editor_state_cursor_at_start_with_content`
- [x] In `readline.rs`, remove the migrated tests and replace them with a comment pointing
      to the new PTY integration test.
- [x] In the new PTY test, the `controlled` process will perform the state assertions and
      print results to `stdout`, which the `controller` will verify.

#### Phase 15.3: Global Test Cleanup [COMPLETE]

- [x] Audit all crates for any other tests that might be leaking raw mode state.
  - Migrated `test_saved_termios_poisoning_recovery` from `raw_mode_unix.rs` to
    `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`.
- [x] Verify that `./check.fish --full` runs without any "staircase effect" in the output.
- [x] Ensure all `generate_pty_test!` macros are correctly used according to the
      `organize-tests` skill.

#### Phase 15.4: Manual Review [COMPLETE]

- [x] `tui/src/core/pty/pty_engine/pty_pair.rs`
- [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`
- [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`
- [x] `tui/src/core/resilient_reactor_thread/integration_tests/double_panic_prevention_test.rs`
- [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child.rs`
- [x] `tui/src/readline_async/readline_async_impl/readline.rs`
- [x] `tui/src/readline_async/readline_async_impl/integration_tests/mod.rs`
- [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
- [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/mod.rs`

### Phase 16: Refactor RRTWaker to RRTSoftwareInterrupt and Control Plane Nomenclature [COMPLETE]

Rename the confusing `RRTWaker` naming to more descriptive `RRTSoftwareInterrupt` and
introduce "Control Plane" terminology to distinguish between "Real/Data Plane" (I/O) and
"Synthetic/Control Plane" (lifecycle) interrupts.

#### Phase 16.1: Rename Traits and Core Types [COMPLETE]

- [x] Rename `RRTWaker` trait to `RRTSoftwareInterrupt` in `rrt_worker.rs`.
  ```rust
  pub trait RRTSoftwareInterrupt: Send + Sync + Debug + 'static {
      fn trigger_software_interrupt(&self);
  }
  ```
- [x] Rename `WakerHandle` struct to `InterruptHandle` in `rrt_waker_handle.rs`.
- [x] Rename `WakeOnDrop` struct to `InterruptOnDrop` in `rrt_subscriber_guard.rs`.
- [x] Update `SubscriberGuard::wake_on_drop` field to `interrupt_on_drop`.
  ```rust
  pub struct SubscriberGuard<W: RRTWorker> {
      pub receiver: Receiver<RRTEvent<W::Event>>,
      pub interrupt_on_drop: InterruptOnDrop<W>,
      pub sender: BroadcastSender<W::Event>,
  }
  ```

#### Phase 16.2: Refactor Mio Implementation and Factory Pattern [COMPLETE]

- [x] Rename `MioPollWaker` to `MioSoftwareInterrupt` in `mio_poll_waker.rs`.
- [x] Implement `MioSoftwareInterrupt::create_and_register(registry, token)` factory
      method.
  ```rust
  impl MioSoftwareInterrupt {
      pub fn create_and_register(
          registry: &mio::Registry,
          token: mio::Token,
      ) -> miette::Result<Self> {
          let mio_waker = mio::Waker::new(registry, token)
              .map_err(SoftwareInterruptCreationError)?;
          Ok(Self(mio_waker))
      }
  }
  ```
- [x] Refactor `MioPollWorker::create_and_register_os_sources()` to use the new factory
      method.

  ```rust
  fn create_and_register_os_sources() -> miette::Result<(Self, Self::Waker)> {
      let poll_handle = Poll::new().map_err(PollCreationError)?;
      let registry = poll_handle.registry();

      // CONTROL PLANE: Create & register the synthetic software interrupt
      let software_interrupt = MioSoftwareInterrupt::create_and_register(
          registry,
          SourceKindReady::SoftwareInterrupt.to_token(),
      )?;

      // DATA PLANE: Register real hardware/OS sources (stdin, signals)
      // ...
  }
  ```

#### Phase 16.3: Update Framework and Lifecycle Logic [COMPLETE]

- [x] Update `ThreadLifecycleMonitor::wake_if_running()` to `interrupt_if_running()`.
- [x] Update documentation and ASCII diagrams in `resilient_reactor_thread/mod.rs` to
      reflect the new "Software Interrupt" and "Control Plane" terminology. Always define
      what "control plane" or "data plane" when using it. Eg: "CONTROL PLANE: Create &
      register the synthetic software interrupt", "DATA PLANE: Register real hardware/OS
      sources (`stdin`, `signals`)`
- [x] Verify that all existing unit tests and integration tests pass with the new naming.
- [x] Run `check.fish --full` to ensure nothing is broken.

### Phase 17: Use constant glyphs in pty tests, not magic strings [COMPLETE]

- [x] **Phase 17.1: Define and Document Constants**
  - [x] Verified and documented constants in
        `tui/src/core/test_fixtures/pty_test_fixtures/constants.rs`:
    - `SUCCESS_GLYPH` (✅)
    - `FAILURE_GLYPH` (❌)
    - `WAITING_GLYPH` (📝)
    - `WARNING_GLYPH` (⚠️)
    - `CONTROLLER_GLYPH` (🚀)
    - `CONTROLLED_GLYPH` (🔍)
    - `CONTROLLER_CLEANUP_GLYPH` (🧹)
  - [x] Add new constants in the same file (match the doc-comment style of the existing
        ones):
    - `STEP_GLYPH` (📍) — step marker for numbered sequences in PTY tests.
    - `COMPLETION_GLYPH` (🎉) — final "all assertions passed" marker, distinct from
      `SUCCESS_GLYPH`.
  - [x] Fix formatting of `CONTROLLER_CLEANUP_GLYPH` declaration (`constants.rs:78`): add
        the missing space after the colon
        (`pub const CONTROLLER_CLEANUP_GLYPH: &str = ...`).

- [x] **Phase 17.2: Refactor PTY Tests** Surgically replace literals with their respective
      constants in all PTY integration tests.

  **Groups to update:**
  1. **Resilient Reactor Thread Tests:**
     - `tui/src/core/resilient_reactor_thread/integration_tests/double_panic_prevention_test.rs`
  2. **Terminal Raw Mode Tests:**
     - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
     - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
     - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
     - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
     - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`
  3. **VT100 Terminal Input Parser Tests:**
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
     - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  4. **Readline Async Tests:**
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_multiline_output_test.rs`
     - `tui/src/readline_async/readline_async_impl/integration_tests/pty_shared_writer_no_blank_line_test.rs`
  5. **Backend Compatibility Tests:**
     - `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
  6. **Test Fixtures:**
     - `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child.rs`
     - `tui/src/core/test_fixtures/retry.rs`

- [x] **Phase 17.3: Replace raw glyphs with named constants** Surgically replace hardcoded
      glyph literals with their named constants across PTY integration tests. Scope is
      limited to files that import from `tui/src/core/test_fixtures/pty_test_fixtures/`;
      examples and manual validation tests are out of scope.

  **Mappings:**
  - `✓` → `SUCCESS_GLYPH` (progress/sub-step success markers)
  - `🎉` → `COMPLETION_GLYPH` (final "all assertions passed" messages)
  - `📍` → `STEP_GLYPH` (numbered step markers)
  - `🧹` → `CONTROLLER_CLEANUP_GLYPH` (cleanup-phase messages)

  **Files with `✓` (23):**
  - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
  - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
  - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
  - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_multiline_output_test.rs`

  **Files with `📍` and `🎉` (2, subset of above):**
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`

  **Files with `🧹` (16 PTY integration tests, overlapping with above):**
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
  - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`

  Out of scope (leave as-is): `tui/examples/spawn_pty_output_capture.rs` and
  `tui/src/core/ansi/terminal_raw_mode/validation_tests/test_dev_tty_fallback_manual.rs`.

- [x] **Phase 17.4: Verification**
  - [x] Run `./check.fish --test` to verify all PTY tests pass.
  - [x] Run `cargo clippy` to ensure no new warnings were introduced.

- [x] **Phase 17.5: Interactive Manual Review**
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/constants.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/double_panic_prevention_test.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_multiline_output_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_shared_writer_no_blank_line_test.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child.rs`
  - [x] `tui/src/core/test_fixtures/retry.rs`

### Phase 18 - fix pty tests inconsistent handling of EIO and EOF [COMPLETE]

- [x] **Create `BufReadExt` extension trait** in
      `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child.rs` (or a related
      module) to safely normalize Linux's `EIO` (errno 5) into an `EOF` (`Ok(0)`).
  - [x] Implement this trait for `R: std::io::BufRead` with a method
        `read_line_eio(&mut self, buf: &mut String) -> std::io::Result<usize>`.
  - [x] **Cross-platform behavior:** Explicitly document that `EIO` is primarily a
        Linux-specific signal for PTY closure. On non-Linux platforms, `read_line_eio`
        should gracefully handle `EIO` if it ever occurs (effectively mapping it to
        `Ok(0)` universally) and otherwise delegate to `read_line`.
  - [x] **Constant usage:** Reuse the existing `crate::EIO` constant (defined in
        `tui/src/tui/global_constants.rs`) for the check:
        `e.raw_os_error() == Some(crate::EIO)`.
  - [x] **Verification:** Add a unit test for the trait method (e.g., using a mock
        `BufRead` that yields `io::Error::from_raw_os_error(crate::EIO)`) to assert it
        correctly returns `Ok(0)`.
- [x] **Update global helpers** to use `read_line_eio()` internally, making them immune to
      `EIO` panics.
  - [x] Update `read_line_state` and `read_until_marker` in `pty_test_child.rs`.
  - [x] Update `wait_for_ready` to use `read_line_eio()` and remove its inline `EIO`
        handling for consistency.
  - [x] _Note:_ `drain_and_wait` uses `.read()` instead of `.read_line()`, so it will
        retain its inline `EIO` handling (no changes needed there).
  - [x] **UX Improvement:** Document that mapping `EIO` to `Ok(0)` in these helpers is a
        strict UX win. Instead of failing with a cryptic
        `"Read error: Input/output error (os error 5)"`, tests will now fail with the much
        clearer `"EOF reached before getting line state"`.
- [x] **Refactor the global `read_line_state` helper** to accept a predicate closure
      (`impl Fn(&str) -> bool`) instead of a hardcoded prefix string.
  - [x] _Clarification on overlap:_ `wait_for_ready` will remain the dedicated "initial
        handshake" helper returning `Result<(), String>`, while `read_line_state` serves
        as the "state-update synchronizer" that panics on error.
- [x] **Refactor PTY tests** to eliminate the `EIO` vulnerability across the following
      files:
  - [x] **Local Closures:** Delete duplicate local closures (like `read_line_state`) and
        replace them with the newly flexible hardened global helper.
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
    - `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
  - [x] **Redundant Loops:** Delete hand-rolled `wait_for_signal` (or similar) loops and
        replace them with calls to the global `child.wait_for_ready(...)`.
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_subscribe_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_singleton_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
    - `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - [x] **Complex Hand-Rolled Loops:** For loops that _must_ remain hand-rolled due to
        non-blocking I/O timeouts (`ErrorKind::WouldBlock`) or complex state machines,
        change `.read_line(&mut line)` calls to `.read_line_eio(&mut line)`.
    - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
    - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
    - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
    - `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
    - `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_poll_error.rs`
    - `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_factory_restart.rs`
  - [x] **Technical Note (Bug Fix):** Refactoring `pty_mouse_events_test.rs` to use strict
        predicate-based synchronization (`read_line_state`) exposed a bug in
        `protocol_conversion.rs`. Previously, the test passed falsely because it accepted
        any debug output as "success". The new strict check revealed that
        `convert_input_event` was dropping scroll events (which have an `Unknown` button).
        This has been fixed by delaying button mapping until we verify the action requires
        one.
- [x] **Mandatory manual review:** Verify every file modified in Phase 18 for correct
      EIO/EOF handling and ensure no regressions.
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_subscribe_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_singleton_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_poll_error.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_factory_restart.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`

### Phase 19: Refactor pty_test_child into a module directory [COMPLETE]

- [x] Create directory: `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/`
- [x] Create `buf_read_ext.rs` and extract the `ReadLineEioExt` trait (renaming to
      `BufReadExt`), its implementation, and its associated `MockEioReader` tests.
- [x] Create `pty_test_child_impl.rs` and extract `ReadLinesResult`, `wait_for_ready`,
      `read_line_state`, `read_until_marker`, `normalize_pty_line`, and `drain_and_wait`.
- [x] Create `pty_test_child_types.rs` and extract `PtyTestContext`, `PtyTestChild`, and
      its implementation block.
- [x] Create `mod.rs` in the new directory. It should attach the submodules
      (`mod buf_read_ext; mod pty_test_child_types; mod pty_test_child_impl;`) and
      re-export everything
      (`pub use buf_read_ext::*; pub use pty_test_child_types::*; pub use pty_test_child_impl::*;`).
- [x] Remove the original `pty_test_child.rs` file.
- [x] Update all occurrences of `ReadLineEioExt` to `BufReadExt` across the codebase
      (integration tests and docs).
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/buf_read_ext.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_types.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/mod.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`

### Phase 20: Rename SingleThreadSafeControlledChild to PtyTestChild and refactor it [COMPLETE]

- [x] Rename directory:
      `tui/src/core/test_fixtures/pty_test_fixtures/single_thread_safe_controlled_child/`
      -> `pty_test_child/`
- [x] Rename struct: `SingleThreadSafeControlledChild` -> `PtyTestChild`
- [x] Rename `core.rs` -> `pty_test_child_types.rs` and `io_utils.rs` ->
      `pty_test_child_impl.rs`.
- [x] Inline floating functions (`wait_for_ready`, `read_line_state`, `read_until_marker`,
      `normalize_pty_line`, `drain_and_wait`) into `PtyTestChild` methods and remove them.
- [x] Update `spawn_controlled_in_pty()` to return `PtyTestChild` and update its callers
      in `backend_compat` tests.
- [x] Update `mod.rs` and all re-exports.
- [x] Update all rustdoc references in `lib.rs`, `README.md`, `generate_pty_test.rs`,
      `pty_test_child_types.rs`, and `pty_test_child_impl.rs`.
- [x] Update naming in active task files
      (`task/pty-test-harness-harden-for-child-proc-crashing.md`,
      `task/generate_pty_test_macro_audit.md`).
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      naming and ensure no regressions.
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_types.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
  - [x] `tui/src/core/pty/pty_engine/pty_pair.rs`
  - [x] `tui/src/lib.rs`
  - [x] `tui/README.md`
  - [x] `task/pty-test-harness-harden-for-child-proc-crashing.md`
  - [x] `task/generate_pty_test_macro_audit.md`

### Phase 21: Fix pty test file "# Run with:" mod-level-rustdoc sections [COMPLETE]

- [x] Remove `--lib` flag from all `cargo test -p r3bl_tui --lib ...` commands in
      `tui/src`.
- [x] Add `# Run with:` section to
      `tui/src/readline_async/readline_async_impl/integration_tests/pty_readline_test.rs`.
- [x] Add `# Run with:` section to
      `tui/src/readline_async/spinner_impl/integration_tests/pty_spinner_test.rs`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_basic_enable_disable.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_flag_verification.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_input_behavior.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_multiple_cycles.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/integration_tests/mod.rs`
  - [x] `tui/src/readline_async/choose_impl/integration_tests/pty_shared_writer_pause_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_kill_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_editor_state_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_multiline_output_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_shared_writer_no_blank_line_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/mod.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/double_panic_prevention_test.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_factory_restart.rs`
  - [x] `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_poll_error.rs`
  - [x] `tui/src/core/term/integration_tests/test_pty_is_interactive.rs`
  - [x] `tui/src/core/ansi/detect_color/integration_tests/pty_test_color_detection.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_bracketed_paste_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_input_device_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_keyboard_modifiers_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_singleton_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_subscribe_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mio_poller_thread_reuse_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_mouse_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_new_keyboard_features_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_sigwinch_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_terminal_events_test.rs`
  - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/pty_utf8_text_test.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
  - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_readline_test.rs`
  - [x] `tui/src/readline_async/spinner_impl/integration_tests/pty_spinner_test.rs`

### Phase 22: Optimize readline_async PTY integration test execution speed [COMPLETE]

Instead of relying on the `inactivity_watchdog` for happy path exits, introduce explict
Ctrl+C emit and handle in these tests so they don't needlessly wait around for 5 sec. The
`inactivity_watchdog` is for cases where these tests do hang. We aren't removing it, we
are optimizing the tests so they don't rely on the heavy disaster recovery mechanism
designed to reduce test flakiness. Introduce
`tui/src/readline_async/readline_async_impl/integration_tests/readline_async_pty_test_fixtures.rs`
to handle the specific PTY test use cases for `readline_async` without polluting the
global PTY testing infra w/ support for `readline_async` use cases.

- [x] Create `readline_async_pty_test_fixtures.rs` to deduplicate PTY test logic.
- [x] Implement `CONTROL_C` fast exit in `test_pty_alt_word_deletion.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_alt_navigation_test.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_ctrl_d_delete_test.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_ctrl_d_eof_test.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_ctrl_navigation_test.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_ctrl_u_test.rs` (via fixture).
- [x] Implement `CONTROL_C` fast exit in `pty_ctrl_w_test.rs` (via fixture).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/readline_async_pty_test_fixtures.rs`
  - [x] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/test_pty_alt_word_deletion.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_alt_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_eof_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_navigation_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_u_test.rs`
  - [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_w_test.rs`

# Task: Transition `r3bl_tui` from Async/Tokio to Pure Synchronous Architecture

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
    - [Motivation & Architectural Vision](#motivation--architectural-vision)
    - [Strategic Alignment in Roadmap (Workflow B)](#strategic-alignment-in-roadmap-workflow-b)
    - [Guiding Principles](#guiding-principles)
- [Architecture & Design Specifications](#architecture--design-specifications)
    - [1. Resilient Reactor Thread (RRT) Synchronization](#1-resilient-reactor-thread-rrt-synchronization)
    - [2. Input Device & Backend Unification](#2-input-device--backend-unification)
    - [3. Full TUI Main Event Loop (`TerminalWindow`)](#3-full-tui-main-event-loop-terminalwindow)
    - [4. `std_thread_tokio_runner` Transformation](#4-std_thread_tokio_runner-transformation)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Resilient Reactor Thread (RRT) Synchronous Channel Layer](#phase-1-resilient-reactor-thread-rrt-synchronous-channel-layer)
    - [Phase 2: Synchronous `InputDevice` & Backends (`direct_to_ansi` + `crossterm`)](#phase-2-synchronous-inputdevice--backends-direct_to_ansi--crossterm)
    - [Phase 3: Synchronous Full TUI Event Loop (`TerminalWindow::main_event_loop`)](#phase-3-synchronous-full-tui-event-loop-terminalwindowmain_event_loop)
    - [Phase 4: Transform `std_thread_tokio_runner` into `std_thread_runner`](#phase-4-transform-std_thread_tokio_runner-into-std_thread_runner)
    - [Phase 5: Remove Tokio from `tui/Cargo.toml` and Adapt `network_io`](#phase-5-remove-tokio-from-tuicargotoml-and-adapt-network_io)
    - [Phase 6: Downstream Integration in `cmdr` (`edi`, `giti`, `self_upgrade`)](#phase-6-downstream-integration-in-cmdr-edi-giti-self_upgrade)
    - [Phase 7: Tests, Validation, and Documentation](#phase-7-tests-validation-and-documentation)
- [Success Criteria](#success-criteria)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

---

## Overview

### Motivation & Architectural Vision

The core of `r3bl_tui` (the rendering pipeline, diff engine, layout engine, ANSI parser,
and component/app traits) is **already 100% synchronous**. Async Rust in `r3bl_tui` was
introduced historically for terminal input polling, before our Linux-native
[`direct_to_ansi`] backend and [`ResilientReactorThread` (RRT)] were built.

Today, having Tokio in `tui` introduces substantial incidental complexity:

1. **Async "function coloring"**: Callers must be `async`, requiring `#[tokio::main]` in
   binaries and `block_on` in test harnesses.
2. **Pinned Boxed Futures (`>16 KiB`)**: Functions like `main_event_loop_impl` and
   `choose` require `Pin<Box<dyn Future>>` to avoid stack blowouts and clippy's
   `large_futures` warning.
3. **Nested Runtime Hazards**: Caused the creation of [`std_thread_tokio_runner.rs`] (400
   lines of complex runtime-isolation machinery) just to avoid
   `"Cannot start a runtime from within a runtime"` panics when sync code calls async
   helpers.
4. **Binary & Compilation Overhead**: Pulling Tokio with `features = ["full", "tracing"]`
   adds significant compilation latency and binary bloat to every consumer of `r3bl_tui`.

### Strategic Alignment in Roadmap (Workflow B)

This task represents **Step 3** of the **v0.8.0 Unify Rendering Roadmap** in
[`prepare-v0.8.0-meta-task.md:117-122`](prepare-v0.8.0-meta-task.md#L117-L122):

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. task_unify_cli_and_styled_text.md                            │
│    (Extends TuiStyledText with builder methods for RenderOps)   │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 2. task_refactor_interactive_apis_to_alternate_screen.md        │
│    (Consolidates choose & readline onto TerminalWindow/App/     │
│     Component primitives; deletes legacy cursor engines)        │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 3. remove-tokio.md (THIS TASK)                                  │
│    (Switches TerminalWindow & InputDevice to sync, dropping     │
│     Tokio from tui in one clean, uninterrupted pass!)           │
└─────────────────────────────────────────────────────────────────┘
```

**Why Workflow B is Optimal:**

- In Step 2, `choose` and `readline` are consolidated to run as standard `Component` /
  `App` primitives through [`TerminalWindow::main_event_loop`].
- The bespoke cursor-positioning engines in `choose_impl` and `readline_async_impl` are
  deleted.
- **Therefore, 100% of all UI in `r3bl_tui` now runs through
  `TerminalWindow::main_event_loop` and `InputDevice`.**
- In this task (Step 3), making `TerminalWindow::main_event_loop` and `InputDevice`
  synchronous **automatically makes all UI in the crate (including `choose`, `readline`,
  `edi`, and full apps) synchronous with zero duplicate effort!**

### Guiding Principles

1. **Clean Architectural Sweep**: With all UI unified under `TerminalWindow`,
   synchronizing the event pump synchronizes everything.
2. **Zero Compromise on Responsiveness**: The UI remains fully responsive; blocking I/O
   (such as `read(2)` or `epoll_wait(2)`) remains isolated to dedicated background threads
   (via RRT).
3. **Pure Sync Public API**: Downstream consumers will not need an async runtime to run
   full TUI apps, `choose` pickers, or `readline` prompts.
4. **Incremental Compilation**: Each phase can be compiled and passes local checks
   (`./check.fish --check`).

---

## Architecture & Design Specifications

### 1. Resilient Reactor Thread (RRT) Synchronization

**Target Files:**

- `tui/src/core/resilient_reactor_thread/rrt.rs`
- `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
- `tui/src/core/resilient_reactor_thread/rrt_types.rs`
- `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`

Replace `tokio::sync::broadcast` with a synchronous multi-subscriber broadcast channel
(using `crossbeam-channel` or a mutex-guarded subscriber list with atomic subscriber
tracking):

```rust
pub struct SyncBroadcastSender<T: Clone> {
    subscribers: Arc<StdMutex<Vec<crossbeam_channel::Sender<T>>>>,
    receiver_count: Arc<AtomicUsize>,
}

pub struct SyncBroadcastReceiver<T: Clone> {
    receiver: crossbeam_channel::Receiver<T>,
    receiver_count: Arc<AtomicUsize>,
}
```

`SubscriberGuard` will hold `SyncBroadcastReceiver<RRTEvent<W::Output>>` and implement a
synchronous `recv(&self) -> Result<RRTEvent<W::Output>, RecvError>`.

### 2. Input Device & Backend Unification

**Target File:** `tui/src/core/terminal_io/input_device.rs`

Change `InputDevice` from:

```rust
pub async fn next(&mut self) -> Option<InputEvent>;
```

To:

```rust
pub fn next(&mut self) -> Option<InputEvent>;
```

- **`DirectToAnsiInputDevice`**: Calls `self.subscriber_guard.receiver.recv()`.
- **`CrosstermInputDevice`**: Calls `crossterm::event::read()` (with proper error mapping
  to `InputEvent`), dropping `EventStream`.
- **`MockInputDevice`**: Pops from internal buffer synchronously.

### 3. Full TUI Main Event Loop (`TerminalWindow`)

**Target File:** `tui/src/tui/terminal_window/main_event_loop.rs`

Replace `tokio::select!` with a single unified event channel or
`crossbeam_channel::select!`:

```rust
pub fn run_main_event_loop<S, AS>(
    mut event_loop_state: EventLoopState<S, AS>,
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    mut input_device: InputDevice,
    output_device: OutputDevice,
) -> CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)> {
    loop {
        crossbeam_channel::select! {
            recv(event_loop_state.main_thread_channel_receiver) -> maybe_signal => {
                if let Ok(signal) = maybe_signal {
                    if handle_main_thread_signal(signal, &mut event_loop_state, &mut app, exit_keys, &output_device)? == Continuation::Stop {
                        break;
                    }
                }
            }
            recv(input_receiver) -> maybe_input_event => {
                if let Ok(input_event) = maybe_input_event {
                    handle_input_event(input_event, &mut event_loop_state, &mut app, exit_keys, &output_device);
                } else {
                    break;
                }
            }
        }
        event_loop_state.log_telemetry_info();
    }
    event_loop_state.log_shutdown_info();
    Ok((event_loop_state.global_data, input_device, output_device))
}
```

- `TerminalWindow::main_event_loop` returns `TuiAvailability<CommonResult<()>>` directly,
  deleting `MainEventLoopFuture` and `Pin<Box<...>>`.
- Because `choose` and `readline` now run on `TerminalWindow`, they automatically run
  synchronously!

### 4. `std_thread_tokio_runner` Transformation

**Target File:** `tui/src/core/common/std_thread_tokio_runner.rs`

- Rename / refactor to `std_thread_runner.rs`.
- Replace `AsyncFnOnce() -> T` with standard synchronous `FnOnce() -> T`.
- Remove all Tokio runtime construction (`Builder::new_current_thread()`, `enable_all()`,
  `block_on()`).
- `run_blocking` executes the function directly or in `std::thread::scope`.
- `run_non_blocking` spawns a standard `std::thread`.

---

## Implementation Plan

### Phase 1: Resilient Reactor Thread (RRT) Synchronous Channel Layer

- [ ] **Step 1.1: Design and Implement Synchronous Broadcast Channel**
    - Implement synchronous broadcast types (`SyncBroadcastSender`,
      `SyncBroadcastReceiver`) in `rrt_types.rs` or wrap `crossbeam-channel`.
    - Support `receiver_count()`, `subscribe()`, and cooperative disconnection detection.
- [ ] **Step 1.2: Refactor RRT Worker & Engine to use Sync Channel**
    - Update `rrt.rs`, `rrt_engine.rs`, `rrt_input_sender.rs`, `rrt_subscriber_guard.rs`
      to use synchronous channel types.
    - Convert `SubscriberGuard::receiver` to synchronous `recv()`.
- [ ] **Step 1.3: Update RRT Unit & Integration Tests**
    - Update `rrt_integration_tests` to test synchronous event reception.
- [ ] **Step 1.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/resilient_reactor_thread/rrt.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_types.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_worker.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`

---

### Phase 2: Synchronous `InputDevice` & Backends (`direct_to_ansi` + `crossterm`)

- [ ] **Step 2.1: Update `DirectToAnsiInputDevice`**
    - Change `pub async fn next(&mut self)` to
      `pub fn next(&mut self) -> Option<InputEvent>`.
    - Call blocking `self.subscriber_guard.receiver.recv()`.
- [ ] **Step 2.2: Update `CrosstermInputDevice`**
    - Remove `crossterm::event::EventStream`.
    - Implement `pub fn next(&mut self) -> Option<InputEvent>` using
      `crossterm::event::read()`.
- [ ] **Step 2.3: Update `MockInputDevice` and `InputDevice`**
    - Update `InputDevice::next(&mut self)` to be synchronous.
    - Update `MockInputDevice::next(&mut self)` to be synchronous.
- [ ] **Step 2.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/input_device_impl.rs`
    - [ ] `tui/src/core/terminal_io/input_device.rs`
    - [ ] `tui/src/core/terminal_io/mock_input_device.rs`

---

### Phase 3: Synchronous Full TUI Event Loop (`TerminalWindow::main_event_loop`)

- [ ] **Step 3.1: Synchronize `run_main_event_loop`**
    - Replace `tokio::select!` with `crossbeam_channel::select!` or unified queue.
    - Convert `run_main_event_loop` from `async fn` to synchronous `fn`.
- [ ] **Step 3.2: Update `main_event_loop_impl` & `TerminalWindow::main_event_loop`**
    - Remove `MainEventLoopFuture` (`Pin<Box<dyn Future>>`).
    - Return `TuiAvailability<CommonResult<()>>` directly.
- [ ] **Step 3.3: Update Global Data Channels**
    - Convert `main_thread_channel` from `tokio::sync::mpsc` to `crossbeam_channel` or
      `std::sync::mpsc`.
- [ ] **Step 3.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`

---

### Phase 4: Transform `std_thread_tokio_runner` into `std_thread_runner`

- [ ] **Step 4.1: Refactor `std_thread_tokio_runner.rs` to `std_thread_runner.rs`**
    - Replace `AsyncFnOnce() -> T` with `FnOnce() -> T`.
    - Remove all Tokio runtime building logic
      (`tokio::runtime::Builder::new_current_thread()`).
    - Simplify to pure `std::thread` execution.
- [ ] **Step 4.2: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`

---

### Phase 5: Remove Tokio from `tui/Cargo.toml` and Adapt `network_io`

- [ ] **Step 5.1: Adapt or Isolate `network_io`**
    - Convert `length_prefix_protocol.rs` to use `std::io::{Read, Write}` instead of
      `tokio::io`.
- [ ] **Step 5.2: Remove Tokio from `tui/Cargo.toml`**
    - Remove `tokio` dependency and its features.
    - Add `crossbeam-channel` if needed.
- [ ] **Step 5.3: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/Cargo.toml`

---

### Phase 6: Downstream Integration in `cmdr` (`edi`, `giti`, `self_upgrade`)

- [ ] **Step 6.1: Update `edi` Editor Launcher**
    - Convert `cmdr/src/edi/launcher.rs` from `async fn run_app` to sync `fn run_app`.
    - Remove `future.await` on `TerminalWindow::main_event_loop`.
- [ ] **Step 6.2: Update `giti` Branch & Commands**
    - Remove `.await` on synchronous `choose()` in
      `cmdr/src/giti/branch/branch_checkout_command.rs`.
- [ ] **Step 6.3: Update `self_upgrade` Thread Runner**
    - Update `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs` to pass synchronous
      closures to the updated runner.
- [ ] **Step 6.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs`

---

### Phase 7: Tests, Validation, and Documentation

- [ ] **Step 7.1: Migrate Integration Tests**
    - Convert tests using `#[tokio::test]` to standard `#[test]`.
    - Remove `tokio::runtime::Runtime::new().block_on(...)` test harnesses.
- [ ] **Step 7.2: Documentation & Link Updates**
    - Update rustdoc comments referencing Tokio in `tui/src/lib.rs` and `tui/README.md`.
    - Run `cargo rustdoc-fmt`.
- [ ] **Step 7.3: Full Quality Check**
    - Run `./check.fish --fmt`
    - Run `./check.fish --clippy`
    - Run `./check.fish --test`
- [ ] **Mandatory manual review:**
    - [ ] All modified test files
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/README.md`

---

## Success Criteria

1. **Zero Tokio in `tui`**: `tokio` is completely removed from `tui/Cargo.toml`.
2. **Simplified Type Signatures**: No `MainEventLoopFuture`, `ChooseFuture`,
   `AsyncFnOnce`, or `Pin<Box<dyn Future>>` across TUI entry points.
3. **Parity**: All interactive behavior in `edi`, `giti`, `choose`, and `readline`
   operates with identical responsiveness and zero-latency ESC key handling.
4. **Clean Quality Gate**: `./check.fish --check`, `./check.fish --clippy`, and all
   unit/PTY tests pass.

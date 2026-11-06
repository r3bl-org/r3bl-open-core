# Task: Transition `r3bl_tui` from Async/Tokio to Pure Synchronous Architecture

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
    - [Motivation & Architectural Vision](#motivation--architectural-vision)
    - [Strategic Alignment in Roadmap (Workflow B)](#strategic-alignment-in-roadmap-workflow-b)
    - [Guiding Principles](#guiding-principles)
- [Architecture & Design Specifications](#architecture--design-specifications)
    - [1. Resilient Reactor Thread (RRT) Synchronization (`std::sync::mpsc` + `Waker`)](#1-resilient-reactor-thread-rrt-synchronization-stdsyncmpsc--waker)
    - [2. Input Device & Backend Unification](#2-input-device--backend-unification)
    - [3. Full TUI Main Event Loop (`TerminalWindow`)](#3-full-tui-main-event-loop-terminalwindow)
    - [4. `std_thread_tokio_runner` Transformation & Pure-`std` `Future` Adapter](#4-std_thread_tokio_runner-transformation--pure-std-future-adapter)
    - [5. Remaining `tui` Modules (Delete `AsyncPtySession`, Synchronize `Spinner`, `core::script`, `network_io`)](#5-remaining-tui-modules-delete-asyncptysession-synchronize-spinner-corescript-network_io)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Resilient Reactor Thread (RRT) Synchronous Channel Layer](#phase-1-resilient-reactor-thread-rrt-synchronous-channel-layer)
    - [Phase 2: Synchronous `InputDevice` & Backends (`direct_to_ansi` + `crossterm`)](#phase-2-synchronous-inputdevice--backends-direct_to_ansi--crossterm)
    - [Phase 3: Synchronous Full TUI Event Loop (`TerminalWindow::main_event_loop`)](#phase-3-synchronous-full-tui-event-loop-terminalwindowmain_event_loop)
    - [Phase 4: Transform `std_thread_tokio_runner` into `std_thread_runner` + `Future` Adapter](#phase-4-transform-std_thread_tokio_runner-into-std_thread_runner--future-adapter)
    - [Phase 5: Delete `AsyncPtySession`, Synchronize `Spinner`, `core::script`, `network_io` & Drop Tokio from `tui/Cargo.toml`](#phase-5-delete-asyncptysession-synchronize-spinner-corescript-network_io--drop-tokio-from-tuicargotoml)
    - [Phase 6: Downstream Integration in `cmdr` & `build-infra` (`edi`, `giti`, `self_upgrade`, `cargo-rustdoc-fmt`)](#phase-6-downstream-integration-in-cmdr--build-infra-edi-giti-self_upgrade-cargo-rustdoc-fmt)
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
3. **100% Pure `std` Primitives (Zero External Async/Channel Crates)**: Replace `tokio`
   entirely using standard library primitives (`std::sync::mpsc`, `std::thread`,
   `std::future::Future`, and `std::task::Waker`). Do **not** add `crossbeam-channel` or
   any optional `tokio` feature flag. (Since Rust 1.67+, `std::sync::mpsc` is already
   powered by `crossbeam-channel` under the hood in `libstd`, and since Rust 1.72+,
   `std::sync::mpsc::Sender<T>` is `Clone + Send + Sync`.)
4. **Runtime-Agnostic Async Compatibility (`std::future::Future` Adapters)**: Downstream
   customers using *any* async runtime (`tokio`, `smol`, `async-std`, `futures`) can still
   `.await` TUI entry points (`TerminalWindow`, `choose`, `readline`), `.await`
   `InputDevice` events, and send [`TerminalWindowMainThreadSignal`]s from spawned async
   tasks—all via pure-`std` `Future` and `Waker` adapters with zero runtime lock-in.
5. **Incremental Compilation**: Each phase can be compiled and passes local checks
   (`./check.fish --check`).

---

## Architecture & Design Specifications

### 1. Resilient Reactor Thread (RRT) Synchronization (`std::sync::mpsc` + `Waker`)

**Target Files:**

- `tui/src/core/resilient_reactor_thread/rrt.rs`
- `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
- `tui/src/core/resilient_reactor_thread/rrt_types.rs`
- `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`

Replace `tokio::sync::broadcast` with a 100% pure-`std` multi-subscriber broadcast channel
built on `std::sync::mpsc` with optional `std::task::Waker` support for async consumers:

```rust
pub struct SubscriberSlot<T> {
    sender: std::sync::mpsc::Sender<T>,
    waker: Arc<StdMutex<Option<std::task::Waker>>>,
}

pub struct SyncBroadcastSender<T: Clone> {
    subscribers: Arc<StdMutex<Vec<SubscriberSlot<T>>>>,
    receiver_count: Arc<AtomicUsize>,
}

pub struct SyncBroadcastReceiver<T: Clone> {
    receiver: std::sync::mpsc::Receiver<T>,
    waker: Arc<StdMutex<Option<std::task::Waker>>>,
    receiver_count: Arc<AtomicUsize>,
}
```

- **Synchronous Reception (`recv`)**: `SubscriberGuard` holds
  `SyncBroadcastReceiver<RRTEvent<W::Output>>` and implements blocking
  `recv(&self) -> Result<RRTEvent<W::Output>, std::sync::mpsc::RecvError>`.
- **Runtime-Agnostic Async Reception (`recv_async`)**: Also exposes
  `recv_async(&mut self) -> impl std::future::Future<Output = Result<RRTEvent<W::Output>, std::sync::mpsc::RecvError>>`
  that checks `self.receiver.try_recv()` and registers `cx.waker().clone()` in
  `self.waker` when `Pending`. When `SyncBroadcastSender::send` broadcasts an event, it
  sends on each subscriber's `std::sync::mpsc::Sender` (pruning disconnected slots) and
  wakes any registered `Waker`.
- **Drop Order Preservation (RFC 1857)**: Dropping `SyncBroadcastReceiver` atomically
  decrements `receiver_count` before `SubscriberGuard::interrupt_on_drop` fires.

### 2. Input Device & Backend Unification

**Target File:** `tui/src/core/terminal_io/input_device.rs`

Change `InputDevice` primary API from `async fn next` to synchronous `fn next`, plus a
pure-`std` `next_async` adapter for customers writing custom async `select!` loops:

```rust
impl InputDevice {
    /// Blocking synchronous read of the next terminal input event.
    pub fn next(&mut self) -> Option<InputEvent>;

    /// Runtime-agnostic `std::future::Future` adapter for async callers (`tokio`, `smol`, etc.).
    pub fn next_async(&mut self) -> impl std::future::Future<Output = Option<InputEvent>> + '_;
}
```

- **`DirectToAnsiInputDevice`**:
    - `next(&mut self)` calls `self.subscriber_guard.receiver.recv()`.
    - `next_async(&mut self)` awaits `self.subscriber_guard.receiver.recv_async()`.
- **`CrosstermInputDevice`**:
    - `next(&mut self)` calls `crossterm::event::read()` (with proper error mapping to
      `InputEvent`), dropping `crossterm::event::EventStream` and `futures_util`.
- **`MockInputDevice`**: Pops from internal buffer synchronously (and sleeps via
  `std::thread::sleep` if a synthetic delay is configured).

### 3. Full TUI Main Event Loop (`TerminalWindow`)

**Target Files:**

- `tui/src/tui/terminal_window/main_event_loop.rs`
- `tui/src/tui/terminal_window/terminal_window_api.rs`
- `tui/src/tui/terminal_window/shared_global_data.rs`

Replace `tokio::select!` and `tokio::sync::mpsc` with a **single unified
`std::sync::mpsc::channel`** (no `crossbeam-channel` needed):

1. **Unified Event Channel (`std::sync::mpsc`)**:
   - `GlobalData::main_thread_channel_sender` becomes
     `std::sync::mpsc::Sender<TerminalWindowMainThreadSignal<AS>>`.
   - Add `TerminalWindowMainThreadSignal::Input(InputEvent)` (or an internal
     `EventLoopMessage<AS>` enum with `Signal(TerminalWindowMainThreadSignal<AS>)` and
     `Input(InputEvent)`) fed into the same `std::sync::mpsc` queue.
   - Because `std::sync::mpsc::Sender::send` is non-blocking and `Sender` is
     `Clone + Send + Sync`, **both** standard OS threads (`std::thread::spawn`) **and**
     downstream async tasks (`tokio::spawn`, `smol::spawn`) inside
     `App::app_start_background_services` can call
     `main_thread_channel_sender.send(TerminalWindowMainThreadSignal::ApplyAppSignal(...))`
     directly without `.await`!
   - `request_exit_by_sending_signal` simplifies to a synchronous 1-liner:
     `channel_sender.send(TerminalWindowMainThreadSignal::Exit).ok();` (deleting
     `tokio::spawn`).

2. **Synchronous `run_main_event_loop`**:

```rust
pub fn run_main_event_loop<S, AS>(
    mut event_loop_state: EventLoopState<S, AS>,
    mut app: BoxedSafeApp<S, AS>,
    exit_keys: &[InputEvent],
    mut input_device: InputDevice,
    output_device: OutputDevice,
) -> CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)> {
    while let Ok(signal_or_input) = event_loop_state.main_thread_channel_receiver.recv() {
        match signal_or_input {
            TerminalWindowMainThreadSignal::Input(input_event) => {
                handle_input_event(
                    input_event,
                    &mut event_loop_state,
                    &mut app,
                    exit_keys,
                    &output_device,
                );
            }
            signal => {
                if handle_main_thread_signal(
                    signal,
                    &mut event_loop_state,
                    &mut app,
                    exit_keys,
                    &output_device,
                )? == Continuation::Stop
                {
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

- `TerminalWindow::main_event_loop` runs synchronously and returns
  `TuiAvailability<CommonResult<(GlobalData<S, AS>, InputDevice, OutputDevice)>>`
  directly, deleting `MainEventLoopFuture` and `Pin<Box<dyn Future>>`.
- `TerminalWindow::main_event_loop_async` (and `choose_async` / `readline_async`) wraps
  the synchronous call using `StdThreadRunner::run_async` (below) so async customers can
  `.await` it on any executor without blocking their async worker threads.

### 4. `std_thread_tokio_runner` Transformation & Pure-`std` `Future` Adapter

**Target File:** `tui/src/core/common/std_thread_tokio_runner.rs` (rename to
`std_thread_runner.rs`)

Transform `StdThreadTokioRunner` into `StdThreadRunner` with **three** execution modes
using 100% `std`:

1. **`StdThreadRunner::run_blocking(work_fn: F) -> ThreadRunnerOutcome<T>`**
   - Spawns a named scoped OS thread (`std::thread::scope`) and joins immediately.
2. **`StdThreadRunner::run_non_blocking(work_fn: F) -> std::io::Result<StdThreadJoinHandle<T>>`**
   - Spawns a named unscoped OS thread (`std::thread::Builder::spawn`) and returns a
     handle that can be `.join()`ed synchronously later.
3. **`StdThreadRunner::run_async(work_fn: F) -> ThreadJoinFuture<T>` (Pure-`std` Async Adapter)**
   - Spawns a named unscoped OS thread that executes synchronous `work_fn()`, stores the
     resulting `ThreadRunnerOutcome<T>` into a shared state, and wakes a
     `std::task::Waker`:

```rust
pub struct ThreadJoinFuture<T> {
    shared_state: Arc<StdMutex<ThreadFutureState<T>>>,
}

struct ThreadFutureState<T> {
    outcome: Option<ThreadRunnerOutcome<T>>,
    waker: Option<std::task::Waker>,
}

impl<T> std::future::Future for ThreadJoinFuture<T> {
    type Output = ThreadRunnerOutcome<T>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut guard = self.shared_state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(outcome) = guard.outcome.take() {
            std::task::Poll::Ready(outcome)
        } else {
            guard.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
```

- Also implement `IntoFuture` on `StdThreadJoinHandle<T>` (or provide `.into_future()`) so
  callers can either `.join()` synchronously **or** `.await` it in any async runtime
  (`tokio`, `smol`, `async-std`, etc.)!

### 5. Remaining `tui` Modules (Delete `AsyncPtySession`, Synchronize `Spinner`, `core::script`, `network_io`)

1. **Delete `AsyncPtySession` (`tui/src/core/pty/pty_session/`)**:
   - [`PtySession`](tui/src/core/pty/pty_session/session.rs) is already 100% synchronous
     (`std::sync::mpsc::SyncSender` / `Receiver` + `std::thread::JoinHandle`), and
     [`PTYMux`](tui/src/core/pty/pty_mux/process_manager.rs) already uses `PtySession`
     exclusively.
   - The only caller of `PtySessionBuilder::start_async()` in the workspace was
     `cmdr/src/self_upgrade/upgrade/build.rs`, which switches to synchronous
     `PtySessionBuilder::start()`.
   - Delete `AsyncPtySession` from `session.rs`, `PtySessionBuilder::start_async()` from
     `builder.rs`, and `AsyncPtyOrchestratorHandle` / `AsyncOutputEventReceiverHalf` /
     `AsyncInputEventSenderHalf` from `type_aliases.rs`. (External async callers can use
     `StdThreadRunner::run_async` if needed.)
2. **`Spinner` (`tui/src/readline_async/spinner.rs`)**:
   - Replace `tokio::time::interval` and `tokio::sync::broadcast` with a dedicated
     `std::thread` loop that waits on a `std::sync::mpsc::Receiver<()>` via
     `rx.recv_timeout(tick_interval)`:
     - On `RecvTimeoutError::Timeout`: renders the next spinner tick frame.
     - On `Ok(())` or `RecvTimeoutError::Disconnected`: prints the final message, restores
       terminal state, and exits cleanly.
   - `Spinner::await_shutdown()` joins the spinner thread synchronously (and provides
     `await_shutdown_async()` returning `ThreadJoinFuture<()>` for async consumers).
3. **`core::script` (`tui/src/core/script/`)**:
   - Convert `command_impl.rs`, `git.rs`, `env_source.rs`, `download.rs`, `crates_api.rs`,
     `github_api.rs`, and `http_client.rs` from `tokio::process::Command` / `tokio::fs` /
     async `reqwest` to `std::process::Command`, `std::fs`, and a pure-sync HTTP client
     (e.g., `ureq` with `rustls` and `json`, removing transitive `tokio` from `reqwest`).
4. **`network_io` (`tui/src/network_io/`)**:
   - Convert `length_prefix_protocol.rs` and `compress_block.rs` from
     `tokio::io::{AsyncRead, AsyncWrite}` to `std::io::{Read, Write}`.

---

## Implementation Plan

### Phase 1: Resilient Reactor Thread (RRT) Synchronous Channel Layer

- [ ] **Step 1.1: Design and Implement `std::sync::mpsc` Broadcast Channel with `Waker`**
    - Implement `SyncBroadcastSender` and `SyncBroadcastReceiver` in `rrt_types.rs` using
      `std::sync::mpsc::channel()`, `Arc<AtomicUsize>` for `receiver_count()`, and
      `Arc<StdMutex<Option<std::task::Waker>>>` for async notification.
    - Support `receiver_count()`, `subscribe()`, blocking `recv()`, and runtime-agnostic
      `recv_async()`.
- [ ] **Step 1.2: Refactor RRT Worker & Engine to use Sync Channel**
    - Update `rrt.rs`, `rrt_engine.rs`, `rrt_input_sender.rs`, `rrt_subscriber_guard.rs`
      to use `SyncBroadcastSender` / `SyncBroadcastReceiver`.
    - Preserve RFC 1857 drop order in `SubscriberGuard`.
- [ ] **Step 1.3: Update RRT Unit & Integration Tests**
    - Update `rrt_integration_tests` to test both synchronous `recv()` and `recv_async()`.
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
    - Change `pub fn next(&mut self) -> Option<InputEvent>` to call blocking
      `self.subscriber_guard.receiver.recv()`.
    - Add `pub fn next_async(&mut self)` backed by
      `self.subscriber_guard.receiver.recv_async()`.
- [ ] **Step 2.2: Update `CrosstermInputDevice`**
    - Remove `crossterm::event::EventStream`.
    - Implement `pub fn next(&mut self) -> Option<InputEvent>` using
      `crossterm::event::read()`.
- [ ] **Step 2.3: Update `MockInputDevice` and `InputDevice`**
    - Update `InputDevice::next(&mut self)` to be synchronous and provide
      `InputDevice::next_async(&mut self)` via pure-`std` `Future`.
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

- [ ] **Step 3.1: Synchronize `run_main_event_loop` with Unified `std::sync::mpsc`**
    - Replace `tokio::select!` with a unified `std::sync::mpsc::Receiver` loop that
      processes both `TerminalWindowMainThreadSignal` and `InputEvent`.
    - Convert `run_main_event_loop` from `async fn` to synchronous `fn`.
    - Replace `tokio::spawn` in `request_exit_by_sending_signal` and `send_signal!` with
      direct non-blocking `std::sync::mpsc::Sender::send`.
- [ ] **Step 3.2: Update `main_event_loop_impl` & `TerminalWindow::main_event_loop`**
    - Remove `MainEventLoopFuture` (`Pin<Box<dyn Future>>`).
    - Return `TuiAvailability<CommonResult<...>>` synchronously from
      `TerminalWindow::main_event_loop`.
    - Provide `TerminalWindow::main_event_loop_async` (and async adapter helpers for
      `choose` / `readline`) using `StdThreadRunner::run_async` for async callers.
- [ ] **Step 3.3: Update Global Data Channels**
    - Convert `main_thread_channel` from `tokio::sync::mpsc` to `std::sync::mpsc`.
- [ ] **Step 3.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`

---

### Phase 4: Transform `std_thread_tokio_runner` into `std_thread_runner` + `Future` Adapter

- [ ] **Step 4.1: Refactor `std_thread_tokio_runner.rs` to `std_thread_runner.rs`**
    - Replace `AsyncFnOnce() -> T` with `FnOnce() -> T`.
    - Remove all Tokio runtime building logic
      (`tokio::runtime::Builder::new_current_thread()`).
    - Simplify `run_blocking` and `run_non_blocking` to pure `std::thread` execution.
    - Implement `ThreadJoinFuture<T>` (`std::future::Future` + `std::task::Waker`) and
      `StdThreadRunner::run_async` so any async runtime (`tokio`, `smol`, `async-std`) can
      `.await` background or TUI threads without blocking executor threads.
- [ ] **Step 4.2: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`

---

### Phase 5: Delete `AsyncPtySession`, Synchronize `Spinner`, `core::script`, `network_io` & Drop Tokio from `tui/Cargo.toml`

- [ ] **Step 5.1: Delete `AsyncPtySession` and `PtySessionBuilder::start_async()`**
    - Remove `AsyncPtySession`, `PtySessionBuilder::start_async()`, and the `Async*`
      type aliases from `tui/src/core/pty/pty_session/type_aliases.rs`, `session.rs`, and
      `builder.rs`.
    - Update `tui/src/core/pty/mod.rs` and `tui/src/core/pty/pty_session/mod.rs` rustdoc
      to remove `AsyncPtySession` references.
- [ ] **Step 5.2: Convert `Spinner` to `std::thread` + `recv_timeout`**
    - Replace `tokio::time::interval` and `tokio::sync::broadcast` in
      `tui/src/readline_async/spinner.rs` with `std::thread` and
      `std::sync::mpsc::Receiver::recv_timeout(interval)`.
- [ ] **Step 5.3: Synchronize `core::script` and `network_io`**
    - Convert `tui/src/core/script/` (`git.rs`, `env_source.rs`, `download.rs`,
      `crates_api.rs`, `github_api.rs`, `http_client.rs`, `package_manager.rs`,
      `permissions.rs`) to use `std::process::Command`, `std::fs`, and a pure-sync HTTP
      client (`ureq` with `rustls` and `json`).
    - Convert `length_prefix_protocol.rs` to use `std::io::{Read, Write}` instead of
      `tokio::io`.
- [ ] **Step 5.4: Remove Tokio & Async Crates from `tui/Cargo.toml`**
    - Remove `tokio`, `futures-util`, `futures-core`, `async-stream`, and `reqwest`, and
      drop the `"event-stream"` feature from `crossterm`.
    - Verify no external channel crate (`crossbeam-channel`) is added.
- [ ] **Step 5.5: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/core/pty/pty_session/type_aliases.rs`
    - [ ] `tui/src/core/pty/pty_session/session.rs`
    - [ ] `tui/src/core/pty/pty_session/builder.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/core/script/mod.rs` (and submodules)
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/Cargo.toml`

---

### Phase 6: Downstream Integration in `cmdr` & `build-infra` (`edi`, `giti`, `self_upgrade`, `cargo-rustdoc-fmt`)

- [ ] **Step 6.1: Update `build-infra` (`cargo-rustdoc-fmt`) & Drop Tokio**
    - Remove `#[tokio::main]` and `.await` calls on `try_is_git_repo()` /
      `try_get_changed_files_by_ext()` in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
    - Remove `tokio` from `build-infra/Cargo.toml` and run
      `cargo install --path build-infra --force`.
- [ ] **Step 6.2: Update `edi` Editor Launcher**
    - Convert `cmdr/src/edi/launcher.rs` and `cmdr/src/bin/edi.rs` from `async fn` to sync
      `fn`.
    - Remove `future.await` on `TerminalWindow::main_event_loop`.
- [ ] **Step 6.3: Update `giti` Branch & Commands**
    - Convert `cmdr/src/bin/giti.rs` and `cmdr/src/giti/` commands from `async fn` to
      synchronous `fn`, removing `.await` on `choose()` and git script helpers.
- [ ] **Step 6.4: Update `self_upgrade` & `analytics_client` and Drop Tokio from `cmdr/Cargo.toml`**
    - Update `cmdr/src/self_upgrade/upgrade/build.rs` to use synchronous
      `PtySessionBuilder::start()` instead of `start_async()`.
    - Update `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs` and
      `cmdr/src/analytics_client/` to use synchronous `StdThreadRunner` and pure-sync HTTP
      calls.
    - Remove `tokio`, `futures-util`, and `reqwest` from `cmdr/Cargo.toml`.
- [ ] **Step 6.5: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/Cargo.toml`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs`
    - [ ] `cmdr/Cargo.toml`

---

### Phase 7: Tests, Validation, and Documentation

- [ ] **Step 7.1: Migrate Integration Tests & Add Async Adapter Tests**
    - Convert tests using `#[tokio::test]` to standard `#[test]`.
    - Add unit/integration tests verifying `ThreadJoinFuture` and `recv_async()` work with
      a minimal `std::task::Waker` / async executor harness.
- [ ] **Step 7.2: Documentation & Link Updates**
    - Update rustdoc comments referencing Tokio in `tui/src/lib.rs`, `tui/README.md`, and
      `cmdr/README.md`, and document how async customers (`tokio`, `smol`, `async-std`)
      use the pure-`std` `Future` adapters and `main_thread_channel_sender`.
    - Run `cargo rustdoc-fmt`.
- [ ] **Step 7.3: Full Quality Check**
    - Run `./check.fish --fmt`
    - Run `./check.fish --clippy`
    - Run `./check.fish --test`
- [ ] **Mandatory manual review:**
    - [ ] All modified test files
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/README.md`
    - [ ] `cmdr/README.md`

---

## Success Criteria

1. **Zero Tokio Across All Workspace Crates (`tui`, `cmdr`, `build-infra`, `rust-analyzer-mcp-server`)**:
   `tokio`, `futures-util`, `futures-core`, and `async-stream` are completely removed from
   `Cargo.toml` files, and all channels/futures use 100% pure `std` (`std::sync::mpsc`,
   `std::thread`, `std::future::Future`, `std::task::Waker`).
2. **Runtime-Agnostic Async Support**: Customers using `tokio`, `smol`, `async-std`, or
   any `std::future::Future` executor can `.await` TUI entry points (`ThreadJoinFuture`),
   `.await` input events (`next_async`), and send `TerminalWindowMainThreadSignal`s from
   async tasks without blocking executor worker threads.
3. **Simplified Type Signatures**: No `MainEventLoopFuture`, `ChooseFuture`,
   `AsyncPtySession`, `AsyncFnOnce`, or `Pin<Box<dyn Future>>` across primary TUI entry
   points.
4. **Parity**: All interactive behavior in `edi`, `giti`, `choose`, `readline`, and
   `Spinner` operates with identical responsiveness and zero-latency ESC key handling.
5. **Clean Quality Gate**: `./check.fish --check`, `./check.fish --clippy`, and all
   unit/PTY tests pass.

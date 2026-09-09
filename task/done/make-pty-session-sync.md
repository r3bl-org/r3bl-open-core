# Task: Make PTY Session Synchronous with std::sync and Provide AsyncPtySession Adapter

## Overview

Refactor the PTY session layer to be completely synchronous using the Rust standard
library (`std::sync::mpsc::sync_channel` and `std::thread`), while providing an opt-in
`AsyncPtySession` adapter via `PtySessionBuilder::start_async()`.

### Motivation and Context

As explored in my article [To async or not to async: Rust MCP server][1], forcing
asynchronous runtimes onto operations that are fundamentally synchronous and blocking
often adds unnecessary runtime coupling, executor overhead, and cognitive complexity.

[1]: https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/

In the PTY subsystem:

1. Low-level PTY controllers (especially Windows ConPTY via `portable_pty`) are blocking
   `std::io::Read` and `std::io::Write` streams.
2. The previous reader and writer tasks (`reader_task.rs` and `writer_task.rs`, now
   `threads/reader.rs` and `threads/writer.rs`) already ran inside
   `tokio::task::spawn_blocking`, calling `blocking_send()` and `blocking_recv()`.
3. The only production consumer in the crate, `ProcessManager` in `pty_mux`, is already
   100% synchronous (polling via `try_recv()` and `try_send()`).
4. Making `PtySession` purely synchronous removes the hard Tokio runtime requirement from
   `PtySessionBuilder::start()`, allowing `PtySession` to be used in synchronous binaries,
   scripts, and tests without spinning up a Tokio reactor.
5. For callers that require integration with `tokio::select!` (such as interactive TUI
   loops and terminal emulators), `PtySessionBuilder::start_async()` produces an
   `AsyncPtySession` backed by Tokio channels and bridge tasks.

## Implementation plan

### Phase 1: Convert Core PTY Session to std Synchronous Primitives and Migrate Tasks to Threads

Replace Tokio channels and tasks in `pty_session` with `std::sync::mpsc::sync_channel` and
`std::thread`.

- [x] Update `tui/src/core/pty/pty_session/pty_session_types.rs` to define sync type
      aliases: `InputEventSenderHalf = std::sync::mpsc::SyncSender<PtyInputEvent>`,
      `OutputEventReceiverHalf = std::sync::mpsc::Receiver<PtyOutputEvent>`, and
      `PtyOrchestratorHandle = std::thread::JoinHandle<miette::Result<PtyControlledChildExitStatus>>`.
- [x] Migrate `tasks/` directory to `threads/`: -
      `tui/src/core/pty/pty_session/tasks/reader_task.rs` ->
      `tui/src/core/pty/pty_session/threads/reader.rs` using `SyncSender<PtyOutputEvent>`
      and spawning an OS thread named `pty-reader`. -
      `tui/src/core/pty/pty_session/tasks/writer_task.rs` ->
      `tui/src/core/pty/pty_session/threads/writer.rs` using `Receiver<PtyInputEvent>` and
      `SyncSender<PtyOutputEvent>`, spawned via
      `std::thread::Builder::new().name("pty-writer".into()).spawn(...)`. -
      `tui/src/core/pty/pty_session/tasks/orchestrator.rs` ->
      `tui/src/core/pty/pty_session/threads/orchestrator.rs` running on a dedicated OS
      thread named `pty-orchestrator`, calling `controlled_child.wait()` synchronously,
      joining reader and writer thread handles, and emitting exit status. -
      `tui/src/core/pty/pty_session/tasks/mod.rs` ->
      `tui/src/core/pty/pty_session/threads/mod.rs`.
- [x] Update `tui/src/core/pty/pty_session/pty_session_struct.rs` (formerly
      `pty_session_builder.rs`) `start()` implementation to instantiate
      `std::sync::mpsc::sync_channel(DefaultSize::PtyChannelBufferSize.into())` and return
      synchronous `PtySession`.
- [x] Update `tui/src/core/pty/pty_mux/process_manager.rs` to ensure full compilation and
      compatibility with standard library `SyncSender::try_send` and `Receiver::try_recv`.
- [x] Verify core compilation with `./check.fish --check`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/pty/pty_session/pty_session_types.rs`
    - [x] `tui/src/core/pty/pty_session/threads/reader.rs`
    - [x] `tui/src/core/pty/pty_session/threads/writer.rs`
    - [x] `tui/src/core/pty/pty_session/threads/orchestrator.rs`
    - [x] `tui/src/core/pty/pty_session/threads/mod.rs`
    - [x] `tui/src/core/pty/pty_session/pty_session_struct.rs`
    - [x] `tui/src/core/pty/pty_mux/process_manager.rs`

### Phase 2: Introduce AsyncPtySession and start_async()

Implement the opt-in async adapter layer for applications using Tokio event loops.

- [x] Define `AsyncPtySession` in `tui/src/core/pty/pty_session/pty_session_struct.rs`
      and async type aliases in `pty_session_types.rs` with Tokio
      `Sender<PtyInputEvent>`, Tokio `Receiver<PtyOutputEvent>`, and
      `tokio::task::JoinHandle`.
- [x] Implement `PtySessionBuilder::start_async(self) -> miette::Result<AsyncPtySession>`
      in `pty_session_struct.rs` that spawns bridge tasks between sync channels/threads
      and Tokio channels.
- [x] Expose and re-export `AsyncPtySession` in `tui/src/core/pty/pty_session/mod.rs` and
      `tui/src/core/pty/mod.rs`.
- [x] Update rustdoc documentation and architectural diagrams in
      `tui/src/core/pty/mod.rs`, `tui/src/core/pty/pty_session/mod.rs`, and
      `tui/src/core/pty/pty_engine/pty_pair.rs` describing both synchronous and
      asynchronous usage modes.
- [x] Verify compilation and documentation with `./check.fish --check` and
      `./check.fish --quick-doc`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/pty/mod.rs`
    - [x] `tui/src/core/pty/pty_engine/pty_pair.rs`
    - [x] `tui/src/core/pty/pty_session/mod.rs`
    - [x] `tui/src/core/pty/pty_session/session.rs`
    - [x] `tui/src/core/pty/pty_session/builder.rs`
    - [x] `tui/src/core/pty/pty_session/config.rs`
    - [x] `tui/src/core/pty/pty_session/type_aliases.rs`
    - [x] `tui/src/core/pty/pty_session/events/mod.rs`
    - [x] `tui/src/core/pty/pty_session/events/input.rs`
    - [x] `tui/src/core/pty/pty_session/events/output.rs`
    - [x] `tui/src/core/pty/pty_session/events/key_press_generator.rs`

### Phase 3: Migrate E2E Tests to Sync and Validate Async Adapter

Convert existing E2E tests to pure synchronous tests, add new async adapter tests, and
update examples.

- [x] Convert `tui/src/core/pty/e2e_tests/session_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`, using `orchestrator_task_handle.join()`.
- [x] Convert `tui/src/core/pty/e2e_tests/error_handling_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`.
- [x] Convert `tui/src/core/pty/e2e_tests/osc_capture_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`.
- [x] Convert `tui/src/core/pty/e2e_tests/resize_test.rs` from `#[tokio::test]` to
      synchronous `#[test]` using `rx_output_event.recv_timeout()`.
- [x] Add new asynchronous E2E test `tui/src/core/pty/e2e_tests/async_session_test.rs`
      verifying `PtySessionBuilder::start_async()` within `#[tokio::test]` and
      `tokio::select!`.
- [x] Update examples in `tui/examples/` (`pty_simple_example.rs`,
      `pty_rw_echo_example.rs`, `spawn_pty_interactive.rs`, `spawn_pty_output_capture.rs`)
      and `cmdr/src/analytics_client/upgrade_check.rs` to use `start_async()`.
- [x] Run test suite with `./check.fish --test` and linting with `./check.fish --clippy`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/pty/e2e_tests/session_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/error_handling_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/resize_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/async_session_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/mod.rs`
    - [x] `tui/examples/pty_simple_example.rs`
    - [x] `tui/examples/pty_rw_echo_example.rs`
    - [x] `tui/examples/spawn_pty_interactive.rs`
    - [x] `tui/examples/spawn_pty_output_capture.rs`
    - [x] `cmdr/src/analytics_client/upgrade_check.rs`

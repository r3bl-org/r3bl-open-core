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
2. The current `reader_task` and `writer_task` already run inside
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

### Phase 1: Convert Core PTY Session and Tasks to std Synchronous Primitives

Replace Tokio channels and tasks in `pty_session` with `std::sync::mpsc::sync_channel` and
`std::thread`.

- [ ] Update `tui/src/core/pty/pty_session/pty_session_types.rs` to define sync type
      aliases: `InputEventSenderHalf = std::sync::mpsc::SyncSender<PtyInputEvent>`,
      `OutputEventReceiverHalf = std::sync::mpsc::Receiver<PtyOutputEvent>`, and
      `PtyOrchestratorHandle = std::thread::JoinHandle<miette::Result<PtyControlledChildExitStatus>>`.
- [ ] Refactor `tui/src/core/pty/pty_session/tasks/reader_task.rs` to use
      `SyncSender<PtyOutputEvent>` and spawn an OS thread via
      `std::thread::Builder::new().name("pty-reader".into()).spawn(...)`.
- [ ] Refactor `tui/src/core/pty/pty_session/tasks/writer_task.rs` to use
      `Receiver<PtyInputEvent>` and `SyncSender<PtyOutputEvent>`, spawned via
      `std::thread::Builder::new().name("pty-writer".into()).spawn(...)`.
- [ ] Refactor `tui/src/core/pty/pty_session/tasks/orchestrator.rs` to run on a dedicated
      OS thread via
      `std::thread::Builder::new().name("pty-orchestrator".into()).spawn(...)`, calling
      `controlled_child.wait()` synchronously, joining reader and writer thread handles,
      and emitting exit status.
- [ ] Update `tui/src/core/pty/pty_session/pty_session_builder.rs` `start()`
      implementation to instantiate
      `std::sync::mpsc::sync_channel(DefaultSize::PtyChannelBufferSize.into())` and return
      synchronous `PtySession`.
- [ ] Update `tui/src/core/pty/pty_mux/process_manager.rs` to ensure full compilation and
      compatibility with standard library `SyncSender::try_send` and `Receiver::try_recv`.
- [ ] Verify core compilation with `./check.fish --check`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/pty/pty_session/pty_session_types.rs`
    - [ ] `tui/src/core/pty/pty_session/tasks/reader_task.rs`
    - [ ] `tui/src/core/pty/pty_session/tasks/writer_task.rs`
    - [ ] `tui/src/core/pty/pty_session/tasks/orchestrator.rs`
    - [ ] `tui/src/core/pty/pty_session/pty_session_builder.rs`
    - [ ] `tui/src/core/pty/pty_mux/process_manager.rs`

### Phase 2: Introduce AsyncPtySession and start_async()

Implement the opt-in async adapter layer for applications using Tokio event loops.

- [ ] Create `tui/src/core/pty/pty_session/async_pty_session.rs` defining
      `AsyncPtySession` with Tokio `Sender<PtyInputEvent>`, Tokio
      `Receiver<PtyOutputEvent>`, and `tokio::task::JoinHandle`.
- [ ] Implement `PtySessionBuilder::start_async(self) -> miette::Result<AsyncPtySession>`
      in `pty_session_builder.rs` that spawns bridge tasks between sync channels/threads
      and Tokio channels.
- [ ] Expose and re-export `AsyncPtySession` in `tui/src/core/pty/pty_session/mod.rs` and
      `tui/src/core/pty/mod.rs`.
- [ ] Update rustdoc documentation and architectural diagrams in `tui/src/core/pty/mod.rs`
      and `tui/src/core/pty/pty_session/mod.rs` describing both synchronous and
      asynchronous usage modes.
- [ ] Verify compilation and documentation with `./check.fish --check` and
      `./check.fish --quick-doc`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/pty/pty_session/async_pty_session.rs`
    - [ ] `tui/src/core/pty/pty_session/pty_session_builder.rs`
    - [ ] `tui/src/core/pty/pty_session/mod.rs`
    - [ ] `tui/src/core/pty/mod.rs`

### Phase 3: Migrate E2E Tests to Sync and Validate Async Adapter

Convert existing E2E tests to pure synchronous tests, add new async adapter tests, and
update examples.

- [ ] Convert `tui/src/core/pty/e2e_tests/session_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`, using `orchestrator_task_handle.join()`.
- [ ] Convert `tui/src/core/pty/e2e_tests/error_handling_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`.
- [ ] Convert `tui/src/core/pty/e2e_tests/osc_capture_test.rs` from `#[tokio::test]` to
      synchronous `#[test]`.
- [ ] Convert `tui/src/core/pty/e2e_tests/resize_test.rs` from `#[tokio::test]` to
      synchronous `#[test]` using `rx_output_event.recv_timeout()`.
- [ ] Add new asynchronous E2E test `tui/src/core/pty/e2e_tests/async_session_test.rs`
      verifying `PtySessionBuilder::start_async()` within `#[tokio::test]` and
      `tokio::select!`.
- [ ] Update examples in `tui/examples/` (`pty_simple_example.rs`,
      `pty_rw_echo_example.rs`, `spawn_pty_interactive.rs`, `spawn_pty_output_capture.rs`)
      to use `start_async()`.
- [ ] Run test suite with `./check.fish --test` and linting with `./check.fish --clippy`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/pty/e2e_tests/session_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/error_handling_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/resize_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/async_session_test.rs`
    - [ ] `tui/examples/pty_simple_example.rs`
    - [ ] `tui/examples/pty_rw_echo_example.rs`
    - [ ] `tui/examples/spawn_pty_interactive.rs`
    - [ ] `tui/examples/spawn_pty_output_capture.rs`

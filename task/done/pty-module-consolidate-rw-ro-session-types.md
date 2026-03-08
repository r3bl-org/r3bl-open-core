# Task Overview

Unify `PtyReadOnlySession` and `PtyReadWriteSession` into a single `PtySession` type. This simplifies the public API and ensures all sessions support control events like `Resize`.

# Implementation Plan

## Step 1: Core Types [COMPLETE]

- [x] Rename `PtyReadWriteSession` to `PtySession` in `tui/src/core/pty/pty_session/pty_session_core.rs`.
- [x] Delete `PtyReadOnlySession` from `tui/src/core/pty/pty_session/pty_session_core.rs`.
- [x] Standardize field names for readability:
    - [x] `input_event_ch_tx_half` -> `tx_input_event`
    - [x] `output_evt_ch_rx_half` -> `rx_output_event`
    - [x] `orchestrator_handle` -> `orchestrator_task_handle`
    - [x] `child_process_terminate_handle` -> `child_process_termination_handle`
- [x] Clean up type aliases in `pty_session_core.rs`:
    - [x] Restore `InputEventSenderHalf` and `OutputEventReceiverHalf`.

## Step 2: Task Consolidation [COMPLETE]

- [x] Rename `tui/src/core/pty/pty_session/tasks/orchestrator_read_write.rs` to `orchestrator.rs`.
- [x] Delete `tui/src/core/pty/pty_session/tasks/orchestrator_read_only.rs`.
- [x] Update `tui/src/core/pty/pty_session/tasks/mod.rs`.
- [x] Fix potential deadlock in `orchestrator.rs` by sending `PtyInputEvent::Close` to the writer task when the child process exits.

## Step 3: Implementation and Builder Simplification [COMPLETE]

- [x] Rename `tui/src/core/pty/pty_session/pty_session_impl_read_write.rs` to `pty_session_impl.rs`.
- [x] Delete `tui/src/core/pty/pty_session/pty_session_impl_read_only.rs`.
- [x] Update `PtySessionBuilder`:
    - [x] Rename `start_read_write_session()` to `start_session()`.
    - [x] Delete `start_read_only_session()`.

## Step 4: Documentation Overhaul [COMPLETE]

- [x] Update `tui/src/core/pty/mod.rs`:
    - [x] Remove "Session Modes" distinction (Task Duo vs Task Trio).
    - [x] Present "Task Trio" as the standard architecture for all sessions.
    - [x] Add code links to `ProcessManager` methods showing keyboard input, resize requests, and output/OSC handling.
- [x] Update `tui/src/core/pty/pty_session/mod.rs`:
    - [x] Update usage examples to use new `PtySession` and `start_session()` API.
- [x] Update re-exports in all `mod.rs` files to export only `PtySession`.

## Step 5: Global Migration and Test Updates [COMPLETE]

- [x] Global search and replace: `PtyReadWriteSession`/`PtyReadOnlySession` -> `PtySession`.
- [x] Update `pty_mux/process_manager.rs` to use new field names and unified API.
- [x] Update E2E tests:
    - [x] Delete `read_only_session_test.rs`.
    - [x] Rename `read_write_session_test.rs` to `session_test.rs` and update contents.
    - [x] Update `resize_test.rs`, `osc_capture_test.rs`, `error_handling_test.rs`.
- [x] Update and rename examples:
    - [x] `spawn_pty_read_only.rs` -> `spawn_pty_output_capture.rs`.
    - [x] `spawn_pty_read_write.rs` -> `spawn_pty_interactive.rs`.
    - [x] `pty_simple_example.rs`, `pty_rw_echo_example.rs`.
- [x] Update `analytics_client/upgrade_check.rs`.
- [x] Clean up `cross_platform_commands.rs`:
    - [x] Remove unused `echo` helper.

## Step 6: Verification [COMPLETE]

- [x] Run `cargo test -p r3bl_tui --lib core::pty` (All 19 tests passed).
- [x] Run `./check.fish --full` (Verified workspace health).

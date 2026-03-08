# Unified PTY Test Fixtures Plan

## Status: COMPLETED (Superseded by PtyTestContext)

This document outlined the original plan to unify PTY integration tests. This plan has been **fully executed and surpassed** by the implementation of the `PtyTestContext` pattern and the `generate_pty_test!` macro automation.

## 1. Accomplished So Far

- [x] **Consolidated logic**: Moved `wait_for_ready` and `read_line_state` logic into `tui/src/core/test_fixtures/pty_test_fixtures/single_thread_safe_controlled_child.rs`.
- [x] **Created shared constants**: Added `CONTROLLED_READY` and `LINE_PREFIX` to `tui/src/core/test_fixtures/pty_test_fixtures/constants.rs`.
- [x] **Consolidated API**: Provided both standalone functions and `SingleThreadSafeControlledChild` methods for synchronization.
- [x] **Proof of concept**: Refactored `pty_ctrl_d_delete_test.rs` to use the new "flawless" pattern. It has been stress-tested (20x) and is stable.

## 2. Evolution: The `PtyTestContext` Pattern

The "Flawless" pattern described below was further evolved into the **`PtyTestContext`** pattern. The `generate_pty_test!` macro now automatically handles:
1. Reader cloning.
2. Platform-specific handshakes (ConPTY DSR on Windows).
3. Resource bundling into a single `PtyTestContext` object.

This eliminates the need for manual `Chain` and `Cursor` usage in individual tests.

## 3. Files Updated

All files listed below have been refactored to use the automated `PtyTestContext` pattern:

### Readline Async Integration Tests
- [x] `pty_alt_kill_test.rs`
- [x] `pty_alt_navigation_test.rs`
- [x] `pty_ctrl_d_eof_test.rs`
- [x] `pty_ctrl_navigation_test.rs`
- [x] `pty_ctrl_u_test.rs`
- [x] `pty_ctrl_w_test.rs`

### Terminal Raw Mode Integration Tests
- [x] `test_basic_enable_disable.rs`
- [x] `test_flag_verification.rs`
- [x] `test_input_behavior.rs`
- [x] `test_multiple_cycles.rs`

### VT100 Parser Integration Tests
- [x] `pty_bracketed_paste_test.rs`
- [x] `pty_input_device_test.rs`
- [x] `pty_keyboard_modifiers_test.rs`
- [x] `pty_mio_poller_singleton_test.rs`
- [x] `pty_mio_poller_subscribe_test.rs`
- [x] `pty_mio_poller_thread_lifecycle_test.rs`
- [x] `pty_mio_poller_thread_reuse_test.rs`
- [x] `pty_mouse_events_test.rs`
- [x] `pty_new_keyboard_features_test.rs`
- [x] `pty_sigwinch_test.rs`
- [x] `pty_terminal_events_test.rs`
- [x] `pty_utf8_text_test.rs`

### Backend Compatibility & Reactor Tests
- [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
- [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
- [x] `tui/src/core/resilient_reactor_thread/tests/rrt_restart_pty_tests.rs`
- [x] `tui/src/core/resilient_reactor_thread/tests/rrt_restart_tests.rs`

## 4. Final Validation
Stress-tested with 20 iterations of all PTY tests:
`for i in {1..20}; do cargo test -p r3bl_tui test_pty -- --nocapture || exit 1; done`
All iterations passed.

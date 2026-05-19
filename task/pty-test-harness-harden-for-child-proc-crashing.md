# Task: Harden PTY test harness to detect child process crashes/failures

## Overview

Currently, the PTY test harness (`PtyTestChild::drain_and_wait()`) ignores the exit status of the `controlled` process. It only logs the status to `stderr`, which is only visible when running tests with `--nocapture`. This means if a `controlled` process crashes (segfault) or panics, the test might still pass if the `controller` doesn't happen to encounter a failure string in the PTY output.

Additionally, many existing PTY tests use manual `std::process::exit()` calls, which is a code smell, bypasses standard harness cleanup, and creates "ghost signals" that the harness doesn't officially validate.

## Goals

1.  **Deterministic Failure Detection**: Make the exit status of the `controlled` process a primary validation channel in `PtyTestChild::drain_and_wait()`.
2.  **Clean Failure Reporting**: Use Rust's `panic!()` mechanism in `controlled` functions instead of manual `exit(1)`.
3.  **Harness Authority**: Ensure the harness is responsible for reaping the process and validating its integrity.
4.  **Remove Redundancy**: Remove `std::process::exit(0)` from `controlled` functions as the macro already handles this.

## Proposed Changes

### 1. Update the Harness (`PtyTestChild::drain_and_wait()`)
Modify `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`:
- Change the `match self.child.wait()` block in `drain_and_wait` to assert that the status is successful.
- From: `eprintln!("{GLYPH_SUCCESS} drain_and_wait: child exited: {status:?}");`
- To: `assert!(status.success(), "Controlled process failed with: {status:?}");`

### 2. Refactor Smelly PTY Tests
Remove manual `std::process::exit()` and `stdout().flush()` calls. Replace failures with `panic!()`.
- `tui/src/core/term/term_integration_tests/test_pty_is_interactive.rs`
- `tui/src/core/ansi/detect_color/color_detection_integration_tests/pty_test_color_detection.rs`
- `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
- `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_input_behavior.rs`
- `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_multiple_cycles.rs`
- `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
- `tui/src/core/resilient_reactor_thread/integration_tests/pty_test_production_poll_error.rs`
- `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs` (Refactor `exit` helper)
- `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`

### 3. Update Documentation & Examples
Update files that show the old `exit(0)` pattern in examples or docs:
- `tui/src/lib.rs` (Doc comments for `generate_pty_test!`)
- `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs` (Doc comments)
- `.agent/skills/organize-tests/pty-conventions.md`
- `.agent/skills/organize-tests/examples.md`

## Implementation Plan

- [ ] **Phase 1: Harness Update**
    - Update `PtyTestChild::drain_and_wait()` to assert `status.success()`.
    - Run existing PTY tests: `./check.fish --test -p r3bl_tui`.
    - Note: Some tests might fail immediately if they were "silently" failing before.

- [ ] **Phase 2: Refactor Smelly Tests**
    - Apply surgical refactors to all identified files in Section 2.
    - Remove `std::process::exit(0)` and `std::process::exit(1)`.
    - Replace `exit(1)` or logic-error exits with `panic!()`.
    - Remove unnecessary `flush()` calls before exit (the harness/macro handles this).

- [ ] **Phase 3: Verify and Formalize**
    - Verify all tests pass with the new hardened harness.
    - Update `pty-conventions.md`, `examples.md`, and inline doc comments.
    - Run `/docs` to verify formatting and links.

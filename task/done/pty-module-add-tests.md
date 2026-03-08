# Task: Add Tests for PTY Module

## Overview
Following the PTY module reorganization, this task provides comprehensive test coverage for the new PTY subsystem. We use a tiered approach: colocated unit tests for logic and cross-platform E2E integration tests for async orchestration, using real processes instead of mocks.

## Tier 1: Unit Tests (Logic)
These tests are colocated within the source files to reduce cognitive load and keep them close to the code they verify.

- [x] `pty_session_builder.rs`:
    - Verify `+` operator for `PtySessionConfigOption`.
    - Verify `Default` values.
    - Verify "last write wins" for `Size` and capture flags.
- [x] `pty_input_event.rs`:
    - Verify `KeyPress` to `PtyInputEvent` mapping.
    - Test complex modifiers (Ctrl+Alt+Shift).
    - Test special keys (ArrowUp, F5, etc.).

## Tier 2: E2E Tests (Orchestration)
Located in `tui/src/core/pty/e2e_tests/`. These tests use real processes and the `cross_platform_commands.rs` helper to verify the full async stack across Linux, macOS, and Windows.

- [x] `read_only_session_test.rs`:
    - Spawn `echo "hello"`.
    - Verify `Output` event contains "hello".
    - Verify `Exit(0)` event is received.
- [x] `read_write_session_test.rs`:
    - Spawn `cat`.
    - Send `PtyInputEvent::Write`.
    - Verify `PtyOutputEvent::Output` matches.
    - Send `PtyInputEvent::Close`.
    - Verify session completes.
- [x] `osc_capture_test.rs`:
    - Spawn process emitting OSC sequences (using `printf` helper).
    - Verify `PtyOutputEvent::Osc` variant is received via `OscSequence` generator.
    - Verify `capture_osc: false` ignores them.
- [x] `error_handling_test.rs`:
    - Force kill child process (using `sleep` helper).
    - Verify `UnexpectedExit` or `Exit` variant reporting.
- [x] `resize_test.rs`:
    - Send `PtyInputEvent::Resize` (using `sh`/`cmd` helper).
    - Verify process reflects new terminal size via `stty size`.

## Tier 3: Logic Coverage Verification
- [x] Ensure all code paths in `pty_session_impl_shared.rs` (reader/writer loops) are exercised.
- [x] Verify `Continuation::Restart` and `Continuation::Stop` behavior in writer task.
- [x] Verify EOF handling in reader task.

---

## Implementation Strategy
1. Unit tests: Implemented as `#[cfg(test)] mod tests` at the bottom of source files.
2. E2E tests: Attached as `mod e2e_tests` to `pty/mod.rs`.
3. Reliability: All async tests use the "await handle -> small sleep -> drain channel" pattern to avoid race conditions.
4. Cross-Platform: All tests use `cross_platform_commands.rs` to abstract OS differences.
5. Validation: Verified with 20 back-to-back runs of `./check.fish --full`.

# Task: Pty Module Add Seamless Windows Support

## Status: DONE

## Problem
PTY integration tests required manual, error-prone boilerplate to work correctly on Windows. Specifically, the `ConPTY` engine blocks all output until a Device Status Report (DSR) handshake is performed. Most tests were bypassing this, making them non-functional on Windows. Additionally, PTY test helper logic was scattered across multiple small files (`read_lines_and_drain.rs`, `normalize_pty_output.rs`).

## Solution
Consolidate all PTY test helpers into a single, cohesive API and automate the Windows-specific handshake within the test macro.

### 1. Consolidation
- All helper types and functions were moved into `tui/src/core/test_fixtures/pty_test_fixtures/single_thread_safe_controlled_child.rs`.
- `ReadLinesResult` struct and `read_until_marker` function were moved from `read_lines_and_drain.rs`.
- `normalize_pty_line` was moved from `normalize_pty_output.rs`.
- Deleted deprecated files: `read_lines_and_drain.rs` and `normalize_pty_output.rs`.

### 2. Implementation of `PtyTestContext`
A new `PtyTestContext` struct was created to bundle all resources needed by a PTY test controller:
- `pty_pair`: The PTY pair handle.
- `child`: The `SingleThreadSafeControlledChild` guard.
- `buf_reader`: A ready-to-use buffered reader for controller output.
- `writer`: A `ControllerWriter` (pre-handshaked on Windows).

### 3. Seamless Handshake Automation
- Added `get_writer_with_handshake()` to `SingleThreadSafeControlledChild`. This method performs the ConPTY DSR handshake on Windows and is a zero-cost no-op on Unix.
- Updated the `generate_pty_test!` macro to:
    1. Prepare the PTY pair and child guard.
    2. Automatically perform the handshake and obtain the writer.
    3. Bundle all resources into a `PtyTestContext`.
    4. Pass the context to the user's controller function.

### 4. Batch Refactoring
Updated ~30 integration tests across the following directories to use the new `PtyTestContext` pattern:
- `tui/src/readline_async/readline_async_impl/integration_tests/`
- `tui/src/core/ansi/terminal_raw_mode/integration_tests/`
- `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/`
- `tui/src/core/resilient_reactor_thread/tests/`

### 5. Robust Synchronization
- Refactored `wait_for_ready()` to use the `BufRead` trait instead of raw `Read`.
- Removed manual "leftover" bytes handling and the `std::io::Chain` pattern. 
- Because a persistent `BufReader` is used throughout the test lifecycle (from handshake to completion), any data arriving after the ready signal is automatically preserved in the internal buffer, making it mathematically safe from data loss without manual intervention.

## Verification Results
- **Typecheck**: `cargo check --tests` passed with zero errors.
- **Documentation**: Fixed intra-doc links in `pty_test_watchdog.rs` and verified with `./check.fish --check`.
- **Stability**: Ran 20 iterations of all PTY integration tests back-to-back using `cargo test -p r3bl_tui test_pty -- --nocapture`.
- **Result**: All 20 iterations passed successfully, confirming the refactoring is robust and regression-free.

<!-- cspell:words SSOT -->

# Task: Fix isatty detection logic

## Overview

The interactivity checks in `tui/src/core/term.rs` have several problems:

1. **Windows workaround is unreachable.** `is_headless()` contains a Windows-specific workaround
   that assumes interactive when `CARGO` env vars are present. But in both `Spinner::try_start()`
   and `ReadlineAsyncContext::try_new()`, per-stream checks (`is_stdout_piped()`,
   `is_stdin_piped()`) bail *before* `is_headless()` runs. On Windows where `cargo run` falsely
   reports streams as non-TTY, the workaround never executes.

2. **Redundant and overlapping checks.** `is_headless()` (all three streams non-TTY) is the
   strictest check — hardest to trigger, catches fewest cases. When per-stream checks already
   guard the call site, `is_headless()` adds nothing. Conversely, using `is_headless()` alone
   would miss partial redirection (e.g., `command | grep foo` where only stdout is piped).

3. **Inconsistent API surface.** Production code uses `is_stdout_piped()` / `is_stdin_piped()`
   (returning `StdoutIsPipedResult` / `StdinIsPipedResult`), while tests use
   `is_output_interactive()` (returning `TTYResult`). Multiple enum types for the same concept.

4. **Inconsistent Detection (Logic Bugs)**: `detect_color_support.rs` and `raw_mode_unix.rs`
   perform their own `isatty` checks, bypassing the `term.rs` logic. This leads to cases
   where an app thinks it's interactive but disables colors, or vice-versa.

## Implementation plan

### Phase 1: Consolidation and cleanup (SSOT in `term.rs`)

Move all direct TTY detection into `term.rs` without changing existing algorithms. Also fix
bugs and improve readability in related code discovered during the consolidation.

- [x] **Refactor `term.rs`**:
  - Make `is_tty_stdin()`, `is_tty_stdout()`, and `is_tty_stderr()` `pub`.
  - Add `TtyStatus` enum (`IsTty` / `IsNotTty`) to replace `bool` return types.
  - Consolidate split `#[cfg(unix)]` / `#[cfg(not(unix))]` function pairs into single
    functions with internal `cfg` gates (for `is_tty_*` and `get_size()`).
- [x] **Update `detect_color_support.rs`**:
  - Remove its internal `is_a_tty` helper.
  - Update `examine_env_vars_to_determine_color_support` to dispatch via
    `term::is_tty_stdout()` / `term::is_tty_stderr()`.
  - Rewrite color detection logic for readability: separate early returns for `NO_COLOR`,
    `FORCE_COLOR`, and non-TTY.
  - Implement `FORCE_COLOR` spec levels (1/2 → `Ansi256`, 3+ → `Truecolor`).
  - Convert platform-specific `if` chain to `match env::consts::OS`, with `linux` folded
    into `_` default arm (covers FreeBSD, OpenBSD, etc.).
  - Add external reference links for all env vars (`NO_COLOR`, `TERM`, `FORCE_COLOR`, etc.).
- [x] **Update `raw_mode_unix.rs`**:
  - Update `get_terminal_fd()` to use `term::is_tty_stdin()`.
  - Rename `ORIGINAL_TERMIOS` → `SAVED_TERMIOS`, `original` → `guard_saved_termios`.
  - Fix bug: `disable_raw_mode()` now resets `SAVED_TERMIOS` to `None` after restoring,
    so subsequent `enable_raw_mode()` cycles save a fresh snapshot.
  - Extract `terminal_fd` module for `TerminalFd` enum and `get()` function.
- [x] **Update `test_multiple_cycles.rs`**:
  - Add double-enable restoration verification (assert cooked mode is restored).
  - Update doc comments to reference `SAVED_TERMIOS` and `enable_raw_mode()` /
    `disable_raw_mode()`.
- [x] **Fix broken links**: `gemini-cli`, `crossterm` version tag in `raw_mode_unix.rs`.
- [x] **Validation**: `./check.fish --full` passes with zero warnings.

### Phase 2: Fix API and Logic Bugs

Fix the design flaws and the Windows `cargo run` bug.

- [ ] **Fix Windows Workaround**:
  - Move the `cargo run` environment check from `is_headless()` into the low-level
    `is_tty_*` helpers so all components (color, raw mode, spinner) benefit.
  - **Note**: This is a conscious decision to favor "out of the box" experience on
    Windows (ensuring color and TUI work under `cargo run`) over strict detection.
- [ ] **Refactor API in `term.rs`**:
  - **Module Documentation**: Add comprehensive mod-level rustdoc explaining the global
    terminal interactivity strategy:
    - `is_input_interactive()`: Can we read keystrokes? (`stdin`).
    - `is_output_interactive()`: Can we render the TUI? (`stdout` only).
    - `is_fully_interactive()`: Is this a "clean" terminal (all three streams are TTYs)?
      Used for environment detection in tests.
    - `emit_stderr_redirection_disclaimer()`: Proactive signaling for redirected `stderr`.
    - **Justification for Windows Workaround**: Document the `cargo run` use case.
  - Rename `is_stdin_interactive()` to `is_input_interactive()`.
  - Add `is_fully_interactive()` (returns `IsInteractive` only if ALL THREE are TTYs).
  - Update `is_output_interactive()` to **focus on `stdout` ONLY**.
  - **Add `emit_stderr_redirection_disclaimer()`**: A helper that, if `is_tty_stderr()`
    is `IsNotTty`, writes a one-line message to `stderr`.
  - Move all functions to return `TTYResult`.
- [ ] **Deprecation/Cleanup**:
  - Remove `is_stdout_piped()`, `is_stdin_piped()`, `StdoutIsPipedResult`,
    `StdinIsPipedResult`, and `is_headless()`.
- [ ] **Update Call Sites**:
  - `Spinner::try_start()`: Use `is_output_interactive()`.
  - `ReadlineAsyncContext::try_new()`: Use `is_input_interactive()` and
    `is_output_interactive()`. Call `emit_stderr_redirection_disclaimer()`.
  - `pty_mux_example.rs`: Update to use `is_input_interactive()` and
    `is_output_interactive()`.
  - `main_event_loop.rs` (production): Call `emit_stderr_redirection_disclaimer()`.
  - `main_event_loop.rs` (test): Update to use `is_fully_interactive()`.
  - `choose_impl/event_loop.rs`: Verify `stdout`-only semantics are acceptable.

### Phase 3: PTY Integration & CI Migration

Migrate tests to run in CI using the PTY infrastructure.

- [ ] **Verify with PTY**:
  - Add `generate_pty_test!` for `term.rs` functions.
  - Include negative tests (controlled process redirects a stream and verifies
    `IsNotInteractive`).
- [ ] **Add tests for `examine_env_vars_to_determine_color_support()`**:
  - Currently has zero test coverage for the detection logic (only the cache/override
    layer is tested).
  - Env var tests (`NO_COLOR`, `TERM=dumb`, `FORCE_COLOR` levels 0/1/2/3, `COLORTERM`,
    `CLICOLOR`) can use `std::env::set_var` + `#[serial]` (same pattern as the existing
    hyperlink detection tests).
  - TTY-dependent behavior (step 3: non-TTY → NoColor) needs `generate_pty_test!` for
    the positive case (real TTY) and a piped stream for the negative case.
- [ ] **Migrate `detect_color_support.rs` tests to process isolation**:
  - The existing `#[serial]` tests in `detect_color_support.rs` modify global state
    (env vars, static overrides) and serialize across the entire test suite.
  - Migrate to the isolated process pattern used elsewhere in the codebase (e.g.,
    `fs_path.rs`, `at_most_one_instance_assert.rs`, `text_operations_rendered.rs`):
    spawn the test in a subprocess via `ISOLATED_TEST_RUNNER` env var so global state
    mutations cannot interfere with other tests.
  - This also applies to the new env var tests added above.
- [ ] **Migrate terminal I/O tests** to `generate_pty_test!`:
  - Move `spinner.rs` and `readline.rs` tests.
  - **Important**: Wrap `async` test logic in
    `tokio::runtime::Runtime::new().unwrap().block_on(...)`.
- [ ] **Network tests** (`github_api.rs`, `package_manager.rs`, `crates_api.rs`): Remove
  the `is_output_interactive()` guards entirely. These tests don't need a terminal — the
  interactivity check was a hack to skip them in CI.

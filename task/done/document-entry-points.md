# Task: Document TUI entry points

## Overview

The `r3bl_tui` crate has three distinct entry points into the "TUI world" - each takes
over the terminal (raw mode, input handling, rendering) and each independently performs
interactivity checks and the stderr redirection disclaimer. These entry points are not
cross-referenced in documentation, making it hard for users to discover alternatives or
understand the overall architecture.

The three entry points are:

1. **`TerminalWindow::main_event_loop()`** - Full TUI framework with `App` trait,
   `FlexBox` layout engine, editor components, dialog system, etc.
2. **`ReadlineAsyncContext::try_new()`** - Async readline with `SharedWriter`, spinner
   support, and `choose()` integration. Lighter weight than the full TUI.
3. **`PTYMux::run()`** - Terminal multiplexer that manages multiple PTY child processes
   with virtual terminal buffers and process switching.

Note: `Spinner::try_start()` and `choose()` are components, not entry points. They can
be used standalone but don't own the terminal lifecycle the way the three above do.

## Implementation plan

### Phase 1: Cross-reference entry points in rustdoc

Each entry point's doc comment should reference the other two, explaining when to use
which. Use intra-doc links with `crate::` paths and reference-style link definitions.

- [x] **`TerminalWindow::main_event_loop()`**
  (`tui/src/tui/terminal_window/terminal_window_api.rs`):
  - Add a "See also" section referencing `ReadlineAsyncContext::try_new()` (for simpler
    CLI-style line input) and `PTYMux::run()` (for terminal multiplexing).
- [x] **`ReadlineAsyncContext::try_new()`**
  (`tui/src/readline_async/readline_async_api.rs`):
  - Add a "See also" section referencing `TerminalWindow::main_event_loop()` (for full
    TUI apps with complex layouts) and `PTYMux::run()`.
- [x] **`PTYMux::run()`** (`tui/src/core/pty/pty_mux/mux.rs`):
  - Add a "See also" section referencing `TerminalWindow::main_event_loop()` and
    `ReadlineAsyncContext::try_new()`.

### Phase 2: Add entry point overview to module-level docs

- [x] **Update `tui/src/lib.rs` module doc**:
  - Add a "High-Level Entry Points" section listing the three entry points with brief
    descriptions of when to use each (using a comparison list or table).
  - Note: Since `README.md` is generated from `lib.rs` via `cargo-readme`, this single
    update covers both documentation and the project landing page.

### Phase 3: Consistency & Component Documentation

- [x] **Standardize Interactivity Error Messages**:
  - Ensure `PTYMux::run()` and `TerminalWindow::main_event_loop()` use consistent
    `miette::bail!` messages when `is_input_interactive()` or `is_output_interactive()`
    fail.
- [x] **Update `choose()` and `Spinner` rustdoc**:
  - Mention that these standalone components also invoke
    `emit_stderr_redirection_disclaimer()` to ensure consistent logging behavior even
    when used outside the primary entry points.
- [x] **Update `term_api.rs` documentation**:
  - Explicitly list `PTYMux` and `choose()` as primary consumers of the
    `is_output_interactive()` check.
- [x] **Fix `PTYMux::run()` return value**:
  - Update `PTYMux::run()` to `bail!` on non-interactive terminal instead of silent
    `Ok(())` to match `TerminalWindow` behavior.

# Task: Standardize TUI entry points to return TuiAvailability<T>

## Overview

This task refactors TUI entry points to provide a unified, non-panicking API for
application startup. We use a **Bifurcated Internalized Check** pattern.

1.  **Internalized Responsibility**: The entry point is now the "one-stop shop". It is
    responsible for calling the interactivity check, retrieving the terminal size, and
    initializing the resource.
2.  **Bifurcation**: The low-level check (`TerminalInteractiveStatus`) is **infallible**
    (2 variants: `Available`, `NotAvailable`) because `isatty` cannot fail. The
    high-level entry point result (`TuiAvailability<T>`) is **fallible** (3 variants:
    `Available(T)`, `NotAvailable`, `Broken`) because initialization (`get_size()`, raw
    mode, etc.) can fail. This is a genuine type-level distinction.
3.  **No Caller-Side Boilerplate**: Callers no longer need to call
    `check_is_terminal_interactive()` or pass a `Size` witness. They simply call the
    entry point and handle the 3-state result.

### Design Details & Clarifications

- **Renaming**: `TuiEnvironment` is renamed to `TerminalInteractiveStatus`. The variants
  change: `Available` (no payload), `NotAvailable(TerminalNotInteractiveReason)`. The
  `Broken` variant is removed from this type since `isatty` cannot fail.
- **Public API**: `check_is_terminal_interactive()` remains **public**. It is still
  required for custom terminal interactions (e.g., direct PTY creation in examples) that
  don't use the high-level entry points. It returns the infallible
  `TerminalInteractiveStatus` (no `get_size()` call).
- **No Convenience Escape Hatches**: `TuiAvailability<T>` intentionally does **not**
    implement `into_result()` or `unwrap()`. These would silently collapse `NotAvailable`
    into an opaque error or panic - exactly what this task is designed to prevent. Callers
    must always handle all three variants explicitly.
- **Scope**: This refactor affects ~60 call sites across the workspace, including
  `cmdr` logic (edi, giti, analytics_client), all `tui` examples, internal `tui`
  entry points, and rustdoc examples.
- **Infallible Internals**: Lower-level components (like `Readline::try_new` or
  `LineState::new`) that already accept `Size` will **not** be changed. Only the public
  facing "entry point" wrappers are updated.
- **Direct Check Users**: Some call sites use `check_is_terminal_interactive()` directly
  for non-entry-point purposes (e.g., `pty_simple_example.rs`, `mouse_inspector.rs`,
  `pty_rw_echo_example.rs`). Since `TerminalInteractiveStatus::Available` no longer
  carries `Size`, these callers must call `get_size()` themselves after the check.
  Both `check_is_terminal_interactive()` and `get_size()` remain **public** to support
  this.

#### Proposed Rust Definitions

```rust
/// Represents the interactivity status of the terminal. This does not represent any
/// fallible states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalInteractiveStatus {
    Available,
    NotAvailable(TerminalNotInteractiveReason),
}

/// Returned by entry points (which are fallible) for building an interactive terminal
/// app:
/// - [`main_event_loop()`]
/// - [`PTYMuxBuilder::build()`]
/// - [`ReadlineAsyncContext::try_new()`]
/// - [`choose()`]
///
/// Initialization ([`get_size()`], entering [raw mode], etc.) is fallible, since they
/// require the use of [`ioctl`] (which is wrapped by [`rustix`]), so this type has a
/// [`Broken`] variant to represent this state.
///
/// [`main_event_loop()`]: `TerminalWindow::main_event_loop`
/// [`PTYMuxBuilder::build()`]: `PTYMuxBuilder::build`
/// [`ReadlineAsyncContext::try_new()`]: `ReadlineAsyncContext::try_new`
/// [`choose()`]: crate::choose
/// [`ioctl`]: https://man7.org/linux/man-pages/man2/ioctl.2.html
/// [`rustix`]: rustix
/// [`Broken`]: Self::Broken
pub enum TuiAvailability<T> {
    Available(T),
    NotAvailable(TerminalNotInteractiveReason),
    Broken(miette::Report),
}

// No into_result() or unwrap() - callers must handle all three variants.

/// Gets the interactivity status of the terminal by querying [`isatty`] on [`stdin`]
/// and [`stdout`], which is infallible, so there is no [`Broken`] variant.
///
/// [`isatty`]: https://man7.org/linux/man-pages/man3/isatty.3.html
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`Broken`]: TuiAvailability::Broken
pub fn check_is_terminal_interactive() -> TerminalInteractiveStatus {
    match (is_input_interactive(), is_output_interactive()) {
        (IsInteractive, IsInteractive) => TerminalInteractiveStatus::Available,
        (IsNotInteractive, IsInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::StdinNotInteractive,
        ),
        (IsInteractive, IsNotInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::StdoutNotInteractive,
        ),
        (IsNotInteractive, IsNotInteractive) => TerminalInteractiveStatus::NotAvailable(
            TerminalNotInteractiveReason::BothStdinAndStdoutNotInteractive,
        ),
    }
}
```

Each entry point follows this internal pattern:
```rust
// Inside an entry point (e.g., main_event_loop).
// Returns TuiAvailability<T>, not Result, so ? is not available.
match check_is_terminal_interactive() {
    TerminalInteractiveStatus::Available => {
        // Fallible init in a closure so we can use ? internally.
        let init = || -> miette::Result<Resource> {
            let size = get_size()?;
            // ... create devices, build the resource ...
            Ok(resource)
        };
        match init() {
            Ok(resource) => TuiAvailability::Available(resource),
            Err(e) => TuiAvailability::Broken(e),
        }
    }
    TerminalInteractiveStatus::NotAvailable(reason) => {
        TuiAvailability::NotAvailable(reason)
    }
}
```

## Final Signatures Specification (5 Canonical Entry Points)

The `Size` parameter is **removed** from all public entry point signatures. Terminal
availability and size checks are internalized.

- **TerminalWindow**:
  ```rust
  pub fn main_event_loop<S, AS>(app, exit_keys, state) -> TuiAvailability<MainEventLoopFuture<S, AS>>
  ```

- **PTYMuxBuilder**:
  ```rust
  pub fn build(self) -> TuiAvailability<PTYMux>
  ```
  - builder accepts process configs via `.add(name, command, args)`
  - optional `.terminal_size(size)` override.

- **ReadlineAsyncContext**:
  ```rust
  pub async fn try_new(prompt, capacity) -> TuiAvailability<ReadlineAsyncContext>
  ```

- **choose()**:
  ```rust
  pub fn choose<'a> (
    header, options, maybe_max_height, maybe_max_width, how, stylesheet, io)
  -> TuiAvailability<ChooseFuture<'a>>
  ```
  - same params minus `size`.

- **Spinner**:
  ```rust
  pub async fn try_start(...) -> TuiAvailability<Spinner>
  ```
  - Only checks `is_output_interactive()` (not the full `check_is_terminal_interactive()`),
    so spinners work with piped stdin.
  - Two modes: **standalone** (pass `None` for `SharedWriter`) or **embedded** (with
    `ReadlineAsyncContext`, pass `SharedWriter` for coordinated output and Ctrl+C/Ctrl+D
    cancellation).

## Expected Call Site Patterns

### 1. TerminalWindow (Full TUI)
```rust
match TerminalWindow::main_event_loop(app, exit_keys, state) {
    TuiAvailability::Available(future) => future.await?,
    TuiAvailability::NotAvailable(reason) => eprintln!("{}", reason.as_err_msg()),
    TuiAvailability::Broken(e) => return Err(e),
}
```

### 2. ReadlineAsyncContext (CLI Input)
```rust
match ReadlineAsyncContext::try_new(Some("> "), None).await {
    TuiAvailability::Available(mut rl_ctx) => {
        let line = rl_ctx.read_line().await?;
    }
    TuiAvailability::NotAvailable(_) => { /* fallback */ }
    TuiAvailability::Broken(e) => return Err(e),
}
```

### 3. PTYMux (Terminal Multiplexer)

Note: `Process::new()` currently requires `Size` for its offscreen buffer. Since
callers no longer have `Size` by default, the builder API changes to accept process
configs (name, command, args) and construct `Process` objects internally with the
correct size during `build()`.

The builder accepts an **optional** `.terminal_size(size)` override. If omitted,
`build()` calls `get_size()` internally (the common mux case). If provided, the
caller's size is used (useful for custom PTY dimensions, testing, or embedding).

```rust
// Common case: use host terminal size (from internal get_size())
match PTYMux::builder()
    .add("bash", "bash", vec![])
    .add("htop", "htop", vec![])
    .build()
{
    TuiAvailability::Available(mux) => mux.run().await?,
    TuiAvailability::NotAvailable(reason) => { /* handle */ }
    TuiAvailability::Broken(e) => return Err(e),
}

// Override: caller provides explicit size
match PTYMux::builder()
    .add("bash", "bash", vec![])
    .terminal_size(custom_size)
    .build()
{
    TuiAvailability::Available(mux) => mux.run().await?,
    // ...
}
```

### 4. choose() (Component)
```rust
match choose(header, options, ..., io) {
    TuiAvailability::Available(future) => {
        let items = future.await?;
        // ... use items ...
    }
    TuiAvailability::NotAvailable(_) => { /* handle */ }
    TuiAvailability::Broken(e) => return Err(e),
}
```

## Implementation plan

### Phase 1: Core Types (`tui/src/core/term/term_api.rs`)

- [ ] **Rename `TuiEnvironment` to `TerminalInteractiveStatus`**:
  - Variants: `Available` (no payload), `NotAvailable(TerminalNotInteractiveReason)`.
  - Remove `Broken` variant (isatty cannot fail).
  - Derive `Clone`, `Copy`, `PartialEq`, `Eq` (now possible without `miette::Report`).
- [ ] **Define `TuiAvailability<T>`**:
  - `Available(T)`, `NotAvailable(TerminalNotInteractiveReason)`, `Broken(miette::Report)`.
  - No `into_result()` or `unwrap()` - force explicit 3-way match.
- [ ] **Update `check_is_terminal_interactive()`**:
  - Return `TerminalInteractiveStatus` (infallible, no `get_size()` call).

### Phase 2: Refactor Entry Points (Internalize)

- [ ] **`TerminalWindow::main_event_loop()`**: Call check internally, remove `Size` param.
- [ ] **`PTYMuxBuilder::build()`**: Call check internally. Replace `.add_process(Process)`
  and `.processes(Vec<Process>)` with `.add(name, command, args)` that stores raw config
  tuples. `build()` constructs `Process` instances with the correct size internally. Keep
  `.terminal_size(size)` as an **optional** override (if omitted, `build()` calls
  `get_size()`; if provided, uses the caller's size).
- [ ] **`ReadlineAsyncContext::try_new()`**: Call check internally, remove `Size` param.
- [ ] **`choose()`**: Call check internally, remove `Size` param.
- [ ] **`Spinner::try_start()`**: Change return type from `miette::Result<Option<Spinner>>`
  to `TuiAvailability<Spinner>`. Use `is_output_interactive()` internally (not the full
  `check_is_terminal_interactive()` — spinners only need stdout).

### Phase 3: Update Call Sites

- [ ] Update `cmdr` (analytics_client, edi, giti) to remove manual pre-checks and `Size`
      passing.
- [ ] Update all `tui/examples` to reflect the new entry point signatures.
- [ ] Update documentation examples to reflect the "one call" pattern, including rustdoc
  examples in `tui/src/core/pty/pty_mux/mod.rs`, `tui/src/lib.rs`, and `tui/README.md`
  that use `Process::new(name, cmd, args, terminal_size)` with the old API.

### Phase 4: Final Cleanup

- [ ] For callers of entry points, verify `get_size()` is not called by callers directly,
      since it should be called by each entry point internally.
- [ ] Verify `check_is_terminal_interactive()` is infallible (no `get_size()` call).
- [ ] Run `./check.fish --clippy` and `./check.fish --test`.

### Phase 5: Migrate mock-based tests to PTY integration tests

Now that `TuiAvailability` entry points internalize the `isatty` check, tests that
use mock I/O devices as a workaround for non-interactive terminals should be migrated
to `generate_pty_test!` so they run in a real PTY with real I/O.

Tests that use mocks for legitimate reasons (capturing render output, testing pure
state logic) remain as unit tests.

#### 5.1: `choose()` tests

- [x] Move `test_shared_writer_pause_works` from inline `#[cfg(test)]` in
      `choose_api.rs` to `choose_impl/integration_tests/pty_shared_writer_pause_test.rs`

#### 5.2: `Spinner` tests (`readline_async/spinner.rs`)

- [ ] `test_spinner_color` - uses `OutputDevice::new_mock()` to bypass `is_terminal_interactive()`
- [ ] `test_spinner_no_color` - same pattern
- [ ] `test_spinner_message_update` - same pattern

#### 5.3: `readline()` tests (`readline_async/readline_async_impl/readline.rs`)

- [ ] `test_readline` - calls `readline()` which checks isatty
- [ ] `test_pause_resume` - same
- [ ] `test_pause_resume_with_output` - same

#### 5.4: `main_event_loop` test (`tui/terminal_window/main_event_loop.rs`)

- [ ] `test_main_event_loop_impl` - checks `is_output_interactive()` with early return

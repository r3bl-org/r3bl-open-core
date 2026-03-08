# Task: Add Auto-Resize Config Option to PtySession

Add a configuration option to `PtySession` that allows it to periodically query the host terminal's size and resize the PTY session accordingly.

## Proposed Changes

### 1. `tui/src/core/pty/pty_session/pty_session_builder.rs`
- Update `PtySessionConfig` struct:
  ```rust
  pub struct PtySessionConfig {
      // ... existing fields ...
      pub auto_resize: Option<std::time::Duration>,
  }
  ```
- Update `PtySessionConfigOption` enum:
  ```rust
  pub enum PtySessionConfigOption {
      // ... existing variants ...
      /// Enable automatic resizing by periodically querying the host terminal size.
      /// If `None` is provided, defaults to 60 seconds.
      AutoResize(Option<std::time::Duration>),
      /// Disable automatic resizing.
      NoAutoResize,
  }
  ```
- Update `impl_default_pty_session_config`:
  - Set `auto_resize` to `None` by default.
- Update `impl_elegant_constructor_dsl_pattern`:
  - Handle `AutoResize` and `NoAutoResize` in the `apply` method.
  - Default `None` in `AutoResize` to `Duration::from_secs(60)`.

### 2. `tui/src/core/pty/pty_session/tasks/orchestrator.rs`
- Modify `spawn_orchestrator_task` to handle periodic resizing.
- Replace the simple `wait()` call with a `tokio::select!` loop if `config.auto_resize` is `Some`.
- Implementation detail:
  ```rust
  let mut last_size = config.pty_size;
  let mut interval = config.auto_resize.map(|d| tokio::time::interval(d));
  
  // Use tokio::select! to wait for either:
  // 1. Child process exits (wait_task_handle).
  // 2. Interval ticks (if enabled).
  ```
- In the interval tick branch:
  - Call `crate::get_size()` (imported from `crate::core::term`).
  - Compare with `last_size`.
  - If different, call `controller.resize(current_size.into())` and update `last_size`.

## Verification Plan

### Automated Tests
- Add unit tests in `pty_session_builder.rs` to verify that the `AutoResize` option is correctly applied to the config.
- Verify that `AutoResize(None)` correctly defaults to 60 seconds.

### Manual Verification
- Create an example or test script that starts a PTY session with a short auto-resize interval (e.g., 1 second).
- Resize the host terminal and verify that the PTY session receives the resize event (can be checked by running a command like `stty size` or a small TUI app inside the PTY).

Task: task/pty-module-pty-session-auto-resize.md

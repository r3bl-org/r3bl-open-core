_Task: PR 458 Integration & Fixes_

# Use case

When you run an interactive TUI application (like `htop` or `neovim`) inside the `pty_mux`
multiplexer example (`cargo run --example pty_mux_example`), the child process "running a
pane" often wants to support mouse clicks (e.g., clicking to sort a column in `htop`).

> For context, `pty_mux` is the terminal multiplexer module, similar to `tmux` or
> `screen`. It allows running multiple child shell processes in isolated virtual terminal
> buffers inside a single host terminal window. The example is a very basic chrome that
> uses this `pty_mux` module in order to function.

To enable this, the child application emits a special sequence (`CSI ? 1000 h`) to its
stdout asking for mouse tracking. Currently the user's mouse clicks in the host terminal
are captured by the `InputRouter` (in `pty_mux/input_router.rs`) and dropped.

With this fix, the virtual terminal parser (`OfsBufVT100`) intercepts the `CSI ? 1000 h`
sequence and tracks the state. The `InputRouter` then checks this state and correctly
translates the host mouse clicks into SGR-format bytes (`\x1b[<{button};{x};{y}M`),
forwarding them into the child application's stdin so the app can react to them
seamlessly. If the child process is a simple shell like `bash` (which does not request
mouse tracking), the clicks are gracefully dropped.

# Overview

PR 458 (by Cecile Tonglet) detects mouse mode changes from PTY child processes and
forwards mouse events to them in SGR format. Rather than creating a redundant
`MouseModeDetector` in `reader_task.rs`, we will integrate mouse tracking state directly
into the existing `vte` parser (`OfsBufVT100`). The `InputRouter` will then query this
state synchronously to decide whether to forward `crate::InputEvent::Mouse` events as
SGR-formatted bytes to the active PTY.

Additionally, we have discovered that the existing `CursorModeDetector` is dead code whose
events are ignored. We will clean this up.

# Execution Workflow

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and, where applicable, test
   functionality.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically
   touched files before the heading is marked as checked `[x]`.

_(Once all headings are successfully implemented and checked off, we will proceed to final
verification and cleanup.)_

# Core Fixes from PR #458

## [ ] 1. Add Mouse Mode Support to `vte` Parser State

Implement parsing and state tracking for xterm mouse mode escape sequences (e.g., 1000,
1002, 1006) natively within the VT100 parser.

- _Context:_ Interactive TUI programs (vim, htop) enable mouse tracking via sequences like
  `CSI ? 1000 h`. We need to track this state to know when to forward mouse events.
- _The Fix:_ Update `vt_100_shim_mode_ops.rs` and `OfsBufVT100` to track
  `mouse_tracking_mode` directly in the active terminal state.
- _File(s) Touched:_ To be determined during implementation.

## [ ] 2. Forward Mouse Events to SGR Format

Translate and route `crate::InputEvent::Mouse` events to the PTY stdin when mouse tracking
is active in the current process's buffer.

- _Context:_ The PTY process expects SGR-format bytes (`\x1b[<{button};{x};{y}M` or `m`)
  for mouse events.
- _The Fix:_ In `InputRouter`, check
  `process_manager.get_active_buffer().get_mouse_tracking_mode()`. If active, translate
  the event using pane-relative 1-based coordinates and send it to the PTY.
- _File(s) Touched:_ `input_router.rs` and potentially input event conversion helpers.

## [ ] 3. Clean Up Dead Code (`CursorModeDetector`)

Remove the abandoned `CursorModeDetector` pattern from the reader task.

- _Context:_ We discovered that `CursorModeDetector` emits
  `PtyOutputEvent::CursorModeChange` which is ignored by all consumers, and
  `CursorKeyMode::default()` is hardcoded everywhere.
- _The Fix:_ Delete `CursorModeDetector`, remove it from `reader_task.rs`, and remove
  `PtyOutputEvent::CursorModeChange` to simplify the PTY reader pipeline.
- _File(s) Touched:_ `reader_task.rs`, `pty_output_event.rs`, and related modules.

# Final Verification & Cleanup

- [ ] Verify full test suite coverage using `./check.fish --full`.
- [ ] Commit these changes locally on `main` and push to remote.
- [ ] Close PR #458 manually on GitHub, stating that its feature was subsumed by our newly
      designed commit. (We are NOT merging the PR or its code).
- [ ] **Important Attribution:** We are implementing our own fixes based on her original
      intent. We will add a
      `Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>` trailer to all of
      the commits we make for this task to ensure she gets proper attribution for the
      feature!
- [ ] Update the current meta-task (e.g. `task/prepare-v0.8.0-meta-task.md`) to check off
      PR #458.
- [ ] **Mandatory manual review:** Verify every file modified in this task for correct
      implementation and ensure no regressions.
  - [ ] `task/prepare-v0.8.0-meta-task.md`

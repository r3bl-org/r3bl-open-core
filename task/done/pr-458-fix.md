_Task: PR 458 Integration (Mouse Tracking Mode)_

<!-- cspell:words DECSET -->

# User Story & Context

## Problem

When running an interactive TUI application (like `htop`, `gitui`, or `hx`) inside the
`pty_mux` multiplexer example, the child process does not respond to mouse interactions.
For example, users cannot click to sort columns in `htop` or select text with the mouse in
`hx`.

## Root Cause

Interactive terminal applications enable mouse support by emitting specific `DECSET`
escape sequences (e.g., `CSI ? 1000 h` for basic tracking, and `CSI ? 1006 h` for SGR
format) to their `stdout`. Currently, our `OfsBufVT100` parser treats these sequences as
unsupported and ignores them. Consequently, because the engine doesn't know the child
process wants mouse events, the `InputRouter` simply drops host mouse events instead of
forwarding them.

## Expected Behavior

The `OfsBufVT100` parser should natively parse and track mouse mode escape sequences,
maintaining an accurate `mouse_tracking` state. When mouse tracking is enabled, the
`InputRouter` should intercept host mouse events, translate them into standard
SGR-formatted byte sequences (`\x1b[<{button};{x};{y}M` or `m`), and write them directly
to the child process's `stdin`.

# Overview

PR 458 (by Cecile Tonglet) implemented mouse mode detection and forwarded mouse events to
the PTY in SGR format. Rather than creating a redundant, separate detector in the
`reader_task`, we will seamlessly integrate this state tracking directly into our new
`OfsBufVT100` parser. The `InputRouter` will then query this state synchronously to route
events.

_(Additionally, while exploring the original PR, we discovered that an existing
`CursorModeDetector` is dead code, which we will clean up.)_

# Implementation Plan

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and, where applicable, test
   functionality.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically
   touched files before the heading is marked as checked `[x]`.

## Phase 1: Clean Up Dead Code (`CursorModeDetector`)

Remove the abandoned `CursorModeDetector` pattern from the reader task.

- _Context:_ We discovered that `CursorModeDetector` emits
  `PtyOutputEvent::CursorModeChange` which is ignored by all consumers.
- _The Fix:_ Delete `CursorModeDetector`, remove it from `reader_task.rs`, and remove
  `PtyOutputEvent::CursorModeChange` to simplify the PTY reader pipeline.
- _File(s) Touched:_ `tui/src/core/pty/pty_session/tasks/reader_task.rs`, `pty_output_event.rs`, and
  related modules.

## [x] Phase 2: VT100 Parser State Tracking

Implement parsing and state tracking for xterm mouse mode escape sequences (1000, 1002,
1003, 1006) natively within the VT100 parser.

- _Context:_ We need to track the application's mouse capabilities to know when to forward
  events.
- _The Fix:_
  - Update `PrivateModeType` in `protocols/csi_codes/private_mode.rs` to include mouse
    modes.
  - Update `vt_100_shim_mode_ops.rs` to handle these modes in `set_private_mode` and
    `reset_private_mode`.
  - Remove the `#[allow(dead_code)]` from `mouse_tracking` in `OfsBufVT100`.
- _File(s) Touched:_
  - `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/private_mode.rs`
  - `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_mode_ops.rs`
  - `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`

## [x] Phase 3: Input Routing & SGR Translation

Translate and route `crate::InputEvent::Mouse` events to the PTY stdin when mouse tracking
is active, utilizing strict type-safe coordinates.

- _Context:_ The PTY process expects standard SGR-format bytes (e.g., `\x1b[<0;10;10M` for
  a left click at x=10, y=10).
- _The Fix:_
  - Create a new generator builder `SgrMouseSequence` in `tui/src/core/ansi/generator/` to
    encapsulate the bitwise formatting logic. It maps `MouseInputKind` and modifiers to
    SGR byte sequences:

    ```rust
    use r3bl_tui::{
        Button, ModifierKeysMask, KeyState, MouseInput, MouseInputKind, TermCol, TermRow, CSI_PARAM_SEPARATOR,
        CSI_START, MOUSE_LEFT_BUTTON_CODE, MOUSE_MIDDLE_BUTTON_CODE, MOUSE_RIGHT_BUTTON_CODE,
        MOUSE_MOTION_FLAG, MOUSE_SCROLL_UP_BUTTON, MOUSE_SCROLL_DOWN_BUTTON, MOUSE_RELEASE_BUTTON_CODE,
        MOUSE_MODIFIER_SHIFT, MOUSE_MODIFIER_ALT, MOUSE_MODIFIER_CTRL, MOUSE_SGR_PRESS, MOUSE_SGR_RELEASE
    };

    impl SgrMouseSequence {
        pub fn generate(event: &MouseInput, x: TermCol, y: TermRow) -> Option<Vec<u8>> {
            // 1. Base button ID mapping
            let mut button_id: u16 = match event.kind {
                MouseInputKind::MouseDown(b) | MouseInputKind::MouseUp(b) => match b {
                    Button::Left => MOUSE_LEFT_BUTTON_CODE,
                    Button::Middle => MOUSE_MIDDLE_BUTTON_CODE,
                    Button::Right => MOUSE_RIGHT_BUTTON_CODE,
                },
                MouseInputKind::MouseDrag(b) => match b {
                    Button::Left => MOUSE_LEFT_BUTTON_CODE | MOUSE_MOTION_FLAG,
                    Button::Middle => MOUSE_MIDDLE_BUTTON_CODE | MOUSE_MOTION_FLAG,
                    Button::Right => MOUSE_RIGHT_BUTTON_CODE | MOUSE_MOTION_FLAG,
                },
                MouseInputKind::ScrollUp => MOUSE_SCROLL_UP_BUTTON,
                MouseInputKind::ScrollDown => MOUSE_SCROLL_DOWN_BUTTON,
                MouseInputKind::MouseMove => MOUSE_RELEASE_BUTTON_CODE | MOUSE_MOTION_FLAG,
                _ => return None,
            };

            // 2. Apply modifier bitmasks
            if let Some(modifiers) = event.maybe_modifier_keys {
                if modifiers.shift_key_state == KeyState::Pressed { button_id |= MOUSE_MODIFIER_SHIFT; }
                if modifiers.alt_key_state == KeyState::Pressed { button_id |= MOUSE_MODIFIER_ALT; }
                if modifiers.ctrl_key_state == KeyState::Pressed { button_id |= MOUSE_MODIFIER_CTRL; }
            }

            // 3. SGR state ('M' = press/scroll/drag, 'm' = release)
            let state_char: char = match event.kind {
                MouseInputKind::MouseUp(_) => MOUSE_SGR_RELEASE as char,
                _ => MOUSE_SGR_PRESS as char,
            };

            // 4. Format using standardized ANSI constants
            // Note: TermCol and TermRow implement Display, so we format them directly.
            Some(
                format!(
                    "{CSI_START}<{button_id}{CSI_PARAM_SEPARATOR}{x}{CSI_PARAM_SEPARATOR}{y}{state_char}"
                )
                .into_bytes(),
            )
        }
    }
    ```

  - In `InputRouter::handle_input`, check if
    `process.vt100_parser().terminal_mode.mouse_tracking.is_enabled()`.
  - If active, use `TermRow` and `TermCol` types (per the `check-bounds-safety` skill) to
    perform bounds-checking against the PTY size (ignoring clicks on the status bar) and
    safely convert the 0-based `InputEvent::Mouse` coordinates into type-safe 1-based
    coordinates.
  - Invoke `SgrMouseSequence::generate()` and send the resulting bytes into the PTY's
    input channel.

  **Implementation Snippet:**

  ```rust
  // Imports to avoid adding crate:: prefix in the code below.
  use crate::{DEBUG_TUI_PTY_MUX, ColIndex, RowIndex, RowHeight, TermRow, TermCol};

  // 1. Extract 0-based coordinates from the `r3bl_tui::MouseInput` event.
  let mouse_col: ColIndex = mouse_event.pos.col_index;
  let mouse_row: RowIndex = mouse_event.pos.row_index;

  // 2. Safely bounds-check against the PTY size using ArrayBoundsCheck trait.
  let pty_height: RowHeight = process.vt100_parser().get_size().row_height;
  if mouse_row.overflows(pty_height) == ArrayOverflowResult::Overflows {
      return; // Drop event; user clicked outside the child's UI surface.
  }

  // 3. Type-safe conversion to 1-based VT-100 coordinates.
  //    (Adding 1 is handled implicitly by the newtypes).
  let term_col: TermCol = mouse_col.into();
  let term_row: TermRow = mouse_row.into();

  // 4. Generate the payload.
  let sgr_bytes: Option<Vec<u8>> = SgrMouseSequence::generate(mouse_event, term_col, term_row);
  if let Some(bytes) = sgr_bytes {
      let _ = process.tx_input_event.try_send(PtyInputEvent::Write(bytes));
  } else {
      DEBUG_TUI_PTY_MUX.then(|| {
          // % is Display, ? is Debug.
          tracing::error! {
              message = "InputRouter::handle_input",
              status = "Unsupported mouse event for SGR translation",
              mouse_event = ?mouse_event,
          };
      });
  }
  ```

- _File(s) Touched:_
  - `tui/src/core/ansi/generator/sgr_mouse.rs`
  - `tui/src/core/pty/pty_mux/input_router.rs`

## [x] Phase 4: Testing

Add unit tests to ensure the new state machine and byte generator behave exactly according
to the VT-100/Xterm specifications.

- [x] **Parser State Tests:** In `vt_100_pty_output_conformance_tests.rs` (or similar),
      write tests verifying that sending `CSI ? 1000 h`, `1002 h`, etc. correctly
      transitions `mouse_tracking` to `Enabled`, and `l` resets it.
- [x] **SGR Byte Generator Tests:** Add unit tests for `SgrMouseSequence::generate()`
      verifying all the complex bitwise logic (avoid magic strings; build expected outputs
      using `CSI_START`, `MOUSE_*` constants, and formatting macros):
  - Left click at x=10, y=10 ->
    `format!("{CSI_START}<{MOUSE_LEFT_BUTTON_CODE}{CSI_PARAM_SEPARATOR}10{CSI_PARAM_SEPARATOR}10{MOUSE_SGR_PRESS}").into_bytes()`
  - Left release ->
    `format!("{CSI_START}<{MOUSE_LEFT_BUTTON_CODE}{CSI_PARAM_SEPARATOR}10{CSI_PARAM_SEPARATOR}10{MOUSE_SGR_RELEASE}").into_bytes()`
  - Right click with Shift modifier ->
    `format!("{CSI_START}<{}{CSI_PARAM_SEPARATOR}10{CSI_PARAM_SEPARATOR}10{MOUSE_SGR_PRESS}", MOUSE_RIGHT_BUTTON_CODE | MOUSE_MODIFIER_SHIFT).into_bytes()`
  - Scroll Up/Down -> correct button IDs and state characters

## [x] Phase 5: Clean up contract between pty_mux_example and pty_mux core module

1. **Problem Statement:** The core `InputRouter` currently hardcodes application-specific
   UX logic (like `Ctrl+Q` to exit, `F1-F12` for process switching, and desktop
   notifications). This conflates the example application's requirements with the core
   multiplexer logic.
2. **Injection Point (Interceptor):** Introduce a type alias for the interceptor closure
   that the `PTYMuxBuilder` can accept. This interceptor runs _before_ the `InputRouter`
   gets the event.
   ```rust
   // Readable type alias for the interceptor closure
   pub type InputInterceptorFn = Box<dyn FnMut(&InputEvent, &mut ProcessManager) -> EventPropagation>;
   ```
   We will use the existing `r3bl_tui::EventPropagation` enum to control the bubbling:
   - `Propagate`: Application ignored the event, let core route it.
   - `Consumed`: Application handled the event (e.g., switched process).
   - `ExitMainEventLoop`: Application handled the event and wants to shut down `PTYMux`.
3. **Refactor `InputRouter`:** Strip out the `F1-F12` and `Ctrl+Q` logic from
   `InputRouter::handle_input`. Its only responsibilities should be:
   - Translating `InputEvent::Mouse` into `SGR` sequences and sending them to the active
     PTY.
   - Fallback forwarding of all other `InputEvent::Keyboard` events to the active PTY.
   - Handling `InputEvent::Resize` and `Shutdown`.
4. **Update `pty_mux_example`:** Implement the interceptor in the example code to provide
   a closure configuring `F1-F12` to switch processes via `process_manager.switch_to(idx)`
   and return `EventConsumed::Consumed`, and returning `EventConsumed::Propagate` for all
   others.
5. **Clean up dependencies:** Move `show_notification_non_blocking` into
   `core/notification.rs`.

## [x] Phase 6: String Allocation Architecture (`fast_strings`)

Establish a definitive Zero-Allocation and Performance String architecture for the entire
codebase.

- _Context:_ We identified confusion and redundant documentation regarding
  `FastStringify`, `format_no_alloc!`, and `inline_string!`.
- _The Fix:_
  - Extracted performance-critical string operations into a new module:
    `tui/src/core/common/fast_strings/`.
  - Created a definitive Source of Truth (SOT) in `fast_strings/mod.rs` outlining the
    Performance Hierarchy.
  - Linked all relevant types and macros to this central SOT via intra-doc links.
  - Created a new `fast-string-allocations` skill in `.agents/skills/`.

---

## Final Verification & Cleanup

- [x] Verify full test suite coverage using `./check.fish --full`. and run
      `check code quality` skill.
- [x] **Mandatory manual review:** Verify every file modified in this task.
  - [x] `tui/src/core/common/fast_strings/fast_stringify.rs`
  - [x] `tui/src/core/common/fast_strings/format_no_alloc.rs`
  - [x] `tui/src/core/common/fast_strings/mod.rs`
  - [x] `tui/src/core/pty/pty_mux/mod.rs`
  - [x] `tui/src/core/pty/pty_mux/mux.rs`
  - [x] `tui/src/core/pty/pty_mux/adaptive_render_budget.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/private_mode.rs`
  - [x] `tui/src/core/pty/pty_mux/input_router.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/types.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_mode_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [x] `tui/src/core/pty/pty_session/tasks/reader_task.rs`
  - [x] `tui/src/core/ansi/generator/sgr_mouse_sequence.rs`
  - [x] `tui/src/core/pty/pty_mux/process_manager.rs`
  - [x] `tui/src/core/mod.rs`
  - [x] `tui/src/core/notification.rs`
  - [x] `tui/src/tui/mod.rs`
  - [x] `tui/examples/pty_mux_example.rs`
  - [x] `task/prepare-v0.8.0-meta-task.md`
- [ ] Ensure all work was done on a new branch (e.g., `feat-pty-mouse-tracking`), rather
      than committing directly to `main` or Cecile's divergent branch.
- [ ] When ready to merge, use the `/merge-pr` slash command to cleanly rebase and merge
      to `main`. Include `Supersedes #458` in the description to gracefully close Cecile's
      draft PR.
- [ ] **Important Attribution:** We are implementing our own fixes based on her original
      intent. We will add a `Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>`
      trailer to all of the commits we make for this task to ensure she gets proper
      attribution for the feature!
- [ ] Update the current meta-task `task/prepare-v0.8.0-meta-task.md` to check off PR
      #458.

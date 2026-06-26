_Task: PR 456 Integration (VT100 Pending Wrap fix)_

<!-- cspell:words wrapneeded -->

# User Story & Context

## Problem

When a user launches a terminal multiplexer (using our library) and opens a shell like
`fish`, typing a long command that reaches the exact rightmost column of the terminal
introduces a spurious blank line. If the user hits Backspace immediately, `fish` expects
the cursor to still conceptually be at the end of the previous line. The immediate wrap
disrupts the UI and breaks the shell prompt rendering.

## Root Cause

The parser currently wraps _immediately_ as soon as a character is printed at the
rightmost column, rather than waiting for the next printable character as required by
VT100 specifications.

## Expected Behavior

When a character is printed at the rightmost column, the cursor should stay at that
column. The actual wrap to the next line should be deferred (a "pending wrap" state) and
only happen when the _next_ printable character is received.

## VT100 spec context

The VT100 specification is over 45 years old, and its edge cases can definitely be a bit
tricky to wrap your head around at first.

Those three concepts—scrolling regions (DECSTBM), auto-wrapping (DECAWM), and cursor
movement rules—all interlock perfectly to create the illusion of a continuous, infinite
scroll of text on a fixed rectangular grid.

### Mental Model

To understand how a VT100 terminal handles wrapping, let's think of it as a mechanical
typewriter. When you type a long sentence and hit the right edge, the typewriter doesn't
automatically drop to the next line (this is the **pending wrap** state). It waits until
you strike the _next_ letter. Before that letter can hit the paper, it must perform two
mechanical steps:

1. **Carriage Return (`handle_carriage_return()`)**: Pushes the carriage all the way to
   the left (Column 0).
2. **Line Feed (`index_down()`)**: Moves the cursor down to the next line. However, if you
   are already at the bottom of the scrolling region, the cursor can't go down any
   further. Instead, it pulls the "paper" up (scrolling the screen) to make room.

By delegating to `handle_carriage_return()` and `index_down()`, we perfectly mimic this
mechanical hardware behavior!

### 1. What does "respect DECSTBM scroll region bounds" mean?

DECSTBM (Set Top and Bottom Margins) is a VT100 sequence that allows terminal applications
to define a specific "scrolling region".

For example, the text editor `nano` sets a scroll region that excludes its top title bar
(rows 1-2) and bottom shortcut menu (rows 23-24). When you scroll down in the document,
`nano` just issues a `Line Feed` command. The terminal instantly shifts only the text in
the middle (rows 3-22) up by one line, leaving the status bars pinned in place perfectly
without needing to redraw the entire screen!

- If you're at the bottom of this region (row 20) and a Line Feed occurs, only the lines
  inside this region (5-20) shift up. Lines 0-4 (headers) and 21+ (footers) remain
  completely untouched!
- How it's implemented: In our codebase, scrolling is handled by
  `OfsBufVT100::index_down()`. This method checks if the cursor is at the
  `scroll_region_bottom`. If it is, it calls `shift_lines_up()` to shift only the bounded
  lines up by one row, clearing the bottom line. By reusing `index_down()`, we get all of
  this complex boundary logic for free!

> Note: `vim` and `tmux` also use this for optimized scrolling and panes, but modern
> full-screen TUIs like `btop` or `hx` **DO NOT** use DECSTBM—they use the Alternate
> Screen Buffer and manually redraw every frame.

### 2. How is wrapping expected to work, and what is DECAWM?

Wrapping occurs when text hits the exact right edge (last column) of the terminal.

- DECAWM Enabled (Auto-Wrap ON): When you print a character at the right margin, the
  cursor does not immediately move to the next line. Instead, the terminal enters a
  "pending wrap" state. It waits. When the very next printable character arrives, the
  terminal first executes a Carriage Return & Line Feed (wrapping to the next line,
  possibly scrolling the screen), and then prints the character.
- DECAWM Disabled (Auto-Wrap OFF): The terminal does not wrap at all. The cursor gets
  permanently pinned to the rightmost column. Any new characters typed will simply
  overwrite whatever character is currently at that right edge.

### 3. How is cursor movement tied to scrolling and pending-wrap?

Pending wrap is a fragile state—it only survives as long as a continuous stream of
printable text is arriving. If the terminal is sitting at the right margin waiting to
wrap, and it suddenly receives an explicit cursor movement command (like Backspace, Tab,
Carriage Return, or a "Move Cursor Up" sequence), the terminal must immediately abort the
pending wrap. For example, if you hit the right margin in fish and immediately hit
Backspace, the cursor should just move left by one. If we didn't clear the pending_wrap
state, the terminal might still attempt to randomly wrap to the next line later!

# Overview

PR 456 (by Cecile Tonglet) implements correct VT100 pending-wrap (wrapneeded) state and
respects DECSTBM scroll region bounds in `apply_pending_wrap()`. This is needed so that
printing at the right margin correctly defers wrapping until the next printable character,
resolving spurious blank line issues in `fish`.

# PR Handling & Attribution

We are implementing our own fixes based on Cecile's original intent and closing her PR
without merging it directly due to underlying architectural changes. We will add a add a
`Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>` trailer to all of the commits
we make for this task to ensure she gets proper attribution for the feature!

# Implementation plan

## Phase 1: Pending Wrap Logic

- 1. [x] Add `pending_wrap` state:
  - **File:** `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - **Action:** Add
    `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum PendingWrap { Yes, No }`.
  - **Action:** Add `pub pending_wrap: PendingWrap` to `ParserGlobalState`. Default to
    `PendingWrap::No`.
- 2. [x] Implement `apply_pending_wrap()`:
  - **File:**
    `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
  - **Action:** Create `pub fn apply_pending_wrap(&mut self) -> miette::Result<()>`.
  - **Implementation:** Call `self.handle_carriage_return()` and `self.index_down()?`,
    then set `self.parser_global_state.pending_wrap = false;`. By using `index_down()`, it
    automatically respects `DECSTBM` scroll region bounds.
- 3. [x] Update `print_char()` for deferred wrapping:
  - **File:**
    `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
  - **Action:** At the very start of `print_char()`, if `pending_wrap` is true, call
    `self.apply_pending_wrap()?`.
  - **Action:** After advancing the cursor (`new_col = current_col + 1`), if it overflows
    (`col_max`), check `auto_wrap_mode`. If enabled (`DECAWM`), set `pending_wrap = true`
    and clamp `cursor_pos.col_index = col_max`. (Remove the buggy manual row increment
    that bypasses scrolling).
- 4. [x] Clear `pending_wrap` on cursor movement:
  - **Files:** `vt_100_impl_cursor_ops.rs` and `vt_100_impl_control_ops.rs`.
  - **Action:** Update all cursor movement operations (e.g., `cursor_up`, `cursor_down`,
    `cursor_forward`, `cursor_backward`, `cursor_to_position`, `cursor_to_column`,
    `cursor_to_line_start`, `cursor_to_next_line_start`, `cursor_to_row`) and control ops
    (`handle_backspace`, `handle_tab`, `handle_line_feed`, `handle_carriage_return`) to
    set `self.parser_global_state.pending_wrap = false;`.
- 5. [x] Testing: Run `./check.fish --check` and `./check.fish --test` to ensure
         correctness.
- 6. [x] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.
- 7. [x] **Mandatory manual review:** Verify every file modified in this phase for correct
         implementation and ensure no regressions.
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_cursor_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_control_ops.rs`
  - [x] `task/prepare-v0.8.0-meta-task.md`
  - [x] Manual testing in `pty_mux_example` with fish shell confirmed that typing past the
        right margin correctly avoids spurious blank lines and correctly respects cursor
        movement (e.g. backspace).

_Task: PR 459 Integration (Scrollback Buffer)_

# User Story & Context

## Problem

When using the PTY multiplexer (e.g., by running `cargo run --example pty_mux_example`),
the terminal emulator currently has no memory of lines that scroll off the top of the
screen. For example, if a user switches to the `fish` or `bash` tab and runs a command
that produces a lot of output like `ls -la /etc`, any text that exceeds the height of the
terminal window is permanently lost and cannot be scrolled back to (since there is no
concept of a scrollback buffer).

## Root Cause

The `OffscreenBuffer` in our VT100 architecture acts purely as a 2D bitblt canvas
representing the active viewport. When a `LineFeed` causes the screen to scroll at the
bottom margin, the top row is simply discarded to make room. The system currently lacks a
dedicated data structure and state management to capture and store these evicted lines.

## Expected Behavior

When text scrolls off the top of the PTY canvas, it should be preserved with its
formatting intact so the user can scroll back to view historical output. Furthermore, when
the application sends the `CSI 3 J` control sequence (clear scrollback), this historical
buffer should be properly emptied.

# Overview

PR 459 by Cecile Tonglet correctly identified the lack of a scrollback buffer and
implemented a custom ring buffer to capture evicted lines and handle `CSI 3 J`. We will
implement this intended capability, but adapt it to use our pre-existing `RingBufferHeap`
primitive and align it with our newly decoupled `OfsBufVT100` parser architecture.

# Implementation Plan

## Final Design (Session 2026-06-19)

We are implementing the scrollback buffer intent proposed in PR #459, but adapting it to
the newly decoupled VT100 parser architecture and leveraging existing core data
structures.

The architecture cleanly separates the PTY canvas from the historical log:

- **`OffscreenBuffer` (Canvas):** A pure 2D bitblt grid. It has no concept of history or
  scrollback.
- **`ScrollbackBuffer` (History):** We are using a `VecDeque`-backed `ScrollbackBuffer`
  (in `tui/src/core/ansi/vt_100_pty_output_parser/parser_state/scrollback_buffer.rs`)
  instead of the older `RingBufferHeap`. It explicitly tracks memory limits and evicts
  older lines when the capacity is exceeded.
- **`OfsBufVT100` (The Brain):** The terminal emulator state machine. It owns both the
  `OffscreenBuffer` and the `ScrollbackBuffer`.

**The Flow:** When `OfsBufVT100` evaluates a `LineFeed` that causes a scroll at the bottom
margin, it will:

1. Grab Row 0 from `OffscreenBuffer`.
2. Push that row into the `ScrollbackBuffer` scrollback.
3. Command `OffscreenBuffer` to shift rows up by 1.

## Phase 1: Backend & History State (`OfsBufVT100`)

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and test functionality.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically
   touched files before the heading is marked as checked `[x]`.

#### [x] 1. Wire up `ScrollbackBuffer` to `OfsBufVT100`

- _Context:_ `OfsBufVT100` needs to hold the scrollback history state.
- _The Fix:_ Encapsulate the scrollback state in `ScrollbackBuffer` tracking its capacity,
  lines (`VecDeque`), and cached memory size. Add it to `OfsBufVT100`.
- _File(s) Touched:_ `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`,
  `tui/src/core/ansi/vt_100_pty_output_parser/parser_state/scrollback_buffer.rs`

#### [x] 2. Capture Evicted Lines on Scroll

- _Context:_ When the screen scrolls, the top line is normally discarded.
- _The Fix:_ In the VT100 parser logic, extract row 0 from the `OffscreenBuffer` and push
  it to `OfsBufVT100.scrollback` before executing the `shift_up()` behavior.
- _File(s) Touched:_
  `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`

#### [x] 3. Clearing Scrollback (`CSI 3 J`)

- _Context:_ The terminal sequence `CSI 3 J` must clear the scrollback history.
- _The Fix:_ Add handling for `erase_display_scrollback` to call `.clear()` on the
  `ScrollbackBuffer` in `OfsBufVT100`.
- _File(s) Touched:_
  `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`

### Backend Testing

- [x] Write unit tests verifying that lines scrolled off the top are correctly pushed into
      the `ScrollbackBuffer` with their styles intact.
- [x] Write unit tests verifying that `CSI 3 J` properly wipes the `ScrollbackBuffer`.
- [x] 3. MANUAL REVIEW

- _Context:_ To prevent catastrophic failures or hallucinatory rewrites, the user MUST
  manually review the changes made in Phase 1 before proceeding to Phase 2.
- _The Fix:_ Provide the following checklist to the user and prompt them to open their
  IDE:
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/parser_state/scrollback_buffer.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_clear_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_clear_ops.rs`
  - [x] `tui/src/core/ansi/constants/generic.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_terminal_ops.rs`

---

## Phase 2: UI Integration (`PTYMux`)

Now that the backend is storing history, the engine needs to intercept inputs and render
the scrollback.

**Prerequisite:** This phase assumes `task/pr-458-fix.md` has been completed, meaning
`OfsBufVT100` accurately tracks `mouse_tracking` mode.

#### [x] 1. Process State Tracking (`scroll_offset`)

- _Context:_ Each virtual terminal process must maintain its own scroll position.
- _The Fix:_ Add `pub scroll_offset: usize` to the `Process` struct (in
  `process_manager.rs`). Add helper methods to increment/decrement this offset (clamped
  between 0 and `scrollback.len()`).
- _File(s) Touched:_ `tui/src/core/pty/pty_mux/process_manager.rs`

#### [x] 2. Input Interception (`InputRouter`)

- _Context:_ We must intercept mouse wheel and Shift+PageUp/Down to scroll the buffer, but
  only when appropriate.
- _The Fix:_ In `InputRouter::handle_input`, check the active process's `OfsBufVT100`
  state.
  - If `is_alt_screen` is true, NEVER intercept scrolling.
  - If `mouse_tracking` is true, NEVER intercept Mouse Wheel events.
  - Otherwise, intercept `InputEvent::Mouse(ScrollUp/Down)` and
    `InputEvent::Keyboard(Shift+PageUp/Down)` and call the new offset helper methods on
    `Process`.
- _File(s) Touched:_ `tui/src/core/pty/pty_mux/input_router.rs`

#### [x] 3. Rendering the Scrollback (`OutputRenderer`)

- _Context:_ The terminal component must read from the scrollback tape when offset.
- _The Fix:_ Update `OutputRenderer::render_from_active_buffer`. If `scroll_offset > 0`,
  it should stitch together lines from `OfsBufVT100.scrollback` (the history) and
  `OffscreenBuffer` (the active canvas) into a unified view before writing to the
  `OutputDevice`.
- _File(s) Touched:_ `tui/src/core/pty/pty_mux/output_renderer.rs`

#### [x] 4. Mandatory code review

- [x] **Mandatory manual review:** Verify every file modified in this task.
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/parser_state/scrollback_buffer.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
  - [x] `tui/src/core/pty/pty_mux/process_manager.rs`
  - [x] `tui/src/core/pty/pty_mux/input_router.rs`
  - [x] `tui/src/core/pty/pty_mux/output_renderer.rs`

---

## Phase 3: Examples & Visual Verification

#### [x] 1. Verify `pty_mux_example`

- _Context:_ The existing example should automatically benefit from the engine upgrades.
- _The Fix:_ Run `cargo run --example pty_mux_example`, switch to the `bash` or `fish`
  tabs, and run `ls -la /etc`. Verify that mouse scrolling and Shift+PageUp/Down correctly
  navigate the scrollback history, and that switching between tabs preserves their
  independent scroll states.
- _Results:_ Verified manually. `bash` and `fish` (Primary Screen) successfully
  intercepted mouse wheel and `Shift+PageUp/Down` to scroll the multiplexer history tape.
  `hx` and `gitui` (Alt Screen + Mouse Enabled) correctly received SGR (1006) mouse
  forwarding and responded natively. `less` (Alt Screen + Mouse Disabled) correctly
  ignored mouse events. `htop` failed to scroll - the scroll wheel did nothing.

  > Why `htop` fails: `htop` uses the older `ncurses` library for rendering. Based on its
  > `terminfo` detection, it often requests legacy mouse formatting (where clicks are
  > encoded as raw bytes like `\x1b[M...` ). However, our engine ignores what `htop` asked
  > for and forcibly sends it the modern SGR (1006) sequence ( `\x1b[<...` ). `htop`
  > receives those modern bytes, doesn't recognize them as a valid legacy mouse click, and
  > silently drops them!
  >
  > In a fully-compliant, production-grade terminal emulator, you would actually have to
  > store the specific `mouse.format` the app requested in the `TerminalModeState`, and
  > dynamically change how you generate the ANSI bytes (Legacy vs UTF-8 vs SGR) depending
  > on what the app asked for.
  >
  > But for our `pty_mux` engine, we opted for the "simplified firehose" approach because
  > supporting legacy mouse byte-encoding protocols is a nightmare (they literally break
  > if your terminal is wider than 223 columns).

#### [x] 2. Fix the firehose mouse tracking so ncurses apps work

- _Context:_ ncurses apps like `htop` request legacy mouse sequences, but our multiplexer
  force-feeds them modern SGR sequences, causing them to ignore the events. Additionally,
  `htop` issues chained CSI private modes (e.g., `ESC [ ? 1000 ; 1006 h`) which exposed a
  critical bug: our parser was only processing the very first parameter and ignoring all
  subsequent chained parameters, causing important mode changes to be silently dropped.
- _The Fix:_ Update the CSI parser (`vt_100_shim_mode_ops.rs`) to iterate through and
  process every parameter in a sequence. Track the requested mouse encoding format
  (`Normal` vs `SGR`) in `TerminalModeState` and generate the appropriate byte sequences
  in the input router. Re-architect `CsiSequence::EnablePrivateMode` and
  `DisablePrivateMode` to support chained modes using `SmallVec`. Refactor
  `ansi_output.rs` and the `fast_int_fmt.rs` macros to safely generate and process complex
  chained ANSI sequences without heap allocations.
- [x] **Manual test:** Make sure that `cargo run --example pty_mux_example` `htop` works
      as expected
- [x] **Mandatory manual review:** Verify every file modified in this task.
  - [x] `tui/src/core/ansi/generator/mod.rs`
  - [x] `tui/src/core/ansi/generator/mouse_x10.rs`
  - [x] `tui/src/core/ansi/generator/mouse_sgr.rs`
  - [x] `tui/src/core/ansi/generator/ansi_output.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_mode_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/private_mode.rs`
  - [x] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [x] `tui/src/core/stack_alloc_types/fast_int_fmt.rs`

---

## Phase 4: Rearchitect the OfsBufVT100 to use "Canvas and Viewport" architecture

This future enhancement has been moved to its own task file:
[`task/ofsbufvt100-change-algo-to-canvas-and-viewport.md`](file:///home/nazmul/github/roc/task/ofsbufvt100-change-algo-to-canvas-and-viewport.md)

---

## Final Verification & Cleanup

- [ ] Verify full test suite coverage using `./check.fish --full`.
- [ ] Ensure all work was done on a new branch (e.g., `feat-scrollback-buffer`), rather
      than committing directly to `main` or Cecile's divergent branch.
- [ ] When ready to merge, use the `/merge-pr` slash command to cleanly rebase and merge
      to `main`. Include `Supersedes #459` in the PR description to gracefully close
      Cecile's draft PR.
- [ ] **Important Attribution:** We are implementing our own fixes based on her original
      intent. We will add a `Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>`
      trailer to all of the commits we make for this task to ensure she gets proper
      attribution for the feature!
- [ ] Update the meta-task `task/prepare-v0.8.0-meta-task.md` to check off PR #459.

---

# Historical Context & PR Divergence

**Why we diverged from Cecile's original PR (#459):**

Cecile (contributor) correctly identified the missing scrollback capability in our PTY mux
and took the initiative to build it. We are capturing her exact intent—providing a bounded
scrollback buffer for the terminal emulator that captures lines on scroll and clears on
`CSI 3 J`.

However, her PR was written as a draft roughly 8+ days ago, and our underlying
architecture has shifted fundamentally since then:

1. **Extraction of `OfsBufVT100`**: Cecile placed the scrollback logic directly inside
   `OffscreenBuffer` because, at the time, `OffscreenBuffer` was a monolithic struct
   holding all VT100 state. Since then, we have decoupled the VT100 parser into its own
   `OfsBufVT100` struct. To maintain this new separation of concerns (where
   `OffscreenBuffer` is just a dumb 2D bitblt canvas), the scrollback history _must_ live
   in `OfsBufVT100`.
2. **Use of a lightweight `ScrollbackBuffer` vs. `RingBufferHeap`**: Cecile implemented
   over 200 lines of custom ring-buffer logic (`ScrollbackBuffer`). While we initially
   planned to use our existing `RingBufferHeap<T, N>`, we opted for an even simpler,
   memory-bounded `VecDeque`-backed `ScrollbackBuffer`. It leverages `PixelCharLine`'s
   memory tracking to cleanly evict older lines while staying within the configured
   `ScrollbackCapacity` limits, completely removing the need for a complex custom
   implementation.

By implementing it this way, we respect and fulfill Cecile's exact feature request while
strictly adhering to the new architectural boundaries of the codebase.

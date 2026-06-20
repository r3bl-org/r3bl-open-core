# Task: PR 459 Integration & Fixes (Scrollback Buffer)

## Final Design (Session 2026-06-19)

We are implementing the scrollback buffer intent proposed in PR #459, but adapting it to
the newly decoupled VT100 parser architecture and leveraging existing core data
structures.

The architecture cleanly separates the PTY canvas from the historical log:

- **`OffscreenBuffer` (Canvas):** A pure 2D bitblt grid. It has no concept of history or
  scrollback.
- **`ScrollbackBuffer` (History):** We will use our existing, battle-tested
  `RingBufferHeap<Vec<StyleAndText>, N>` (from `tui/src/core/common/ring_buffer_heap.rs`).
- **`OfsBufVT100` (The Brain):** The terminal emulator state machine. It owns both the
  `OffscreenBuffer` and the `RingBufferHeap`.

**The Flow:** When `OfsBufVT100` evaluates a `LineFeed` that causes a scroll at the bottom
margin, it will:

1. Grab Row 0 from `OffscreenBuffer`.
2. Push that row into the `RingBufferHeap` scrollback.
3. Command `OffscreenBuffer` to shift rows up by 1.

## Phase 1: Backend & Testing Execution

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and test functionality.
3. **Mandatory Manual Review:** Manually review the touched files before marking as
   checked `[x]`.

### Core Implementation Steps

#### [ ] 1. Wire up `RingBufferHeap` to `OfsBufVT100`

- _Context:_ `OfsBufVT100` needs to hold the scrollback state.
- _The Fix:_ Add `pub scrollback: RingBufferHeap<Vec<StyleAndText>, CAPACITY>` to
  `OfsBufVT100`. (e.g., CAPACITY = 1000).
- _File(s) Touched:_ `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`

#### [ ] 2. Capture Evicted Lines on Scroll

- _Context:_ When the screen scrolls, the top line is normally discarded.
- _The Fix:_ In the VT100 parser logic, extract row 0 from the `OffscreenBuffer` and push
  it to `OfsBufVT100.scrollback` before executing the `shift_up()` behavior.
- _File(s) Touched:_ `tui/src/tui/terminal_lib_backends/vt100/vt_100_impl_control_ops.rs`

#### [ ] 3. Clearing Scrollback (`CSI 3 J`)

- _Context:_ The terminal sequence `CSI 3 J` must clear the scrollback history.
- _The Fix:_ Add handling for `EntireScreenAndScrollback` to call `.clear()` on the
  `RingBufferHeap` in `OfsBufVT100`.
- _File(s) Touched:_ `tui/src/tui/terminal_lib_backends/vt100/vt_100_impl_control_ops.rs`

### Backend Testing

- [ ] Write unit tests verifying that lines scrolled off the top are correctly pushed into
      the `RingBufferHeap` with their styles intact.
- [ ] Write unit tests verifying that `CSI 3 J` properly wipes the `RingBufferHeap`.
- [ ] Add these to the existing `vt_100_pty_output_conformance_tests` suite.

---

## Phase 2: UI Integration & Visual Example

Once the backend is solid, we need a way to actually view the scrollback in the UI and
verify it visually.

#### [ ] 1. PTY UI State & Rendering

- _Context:_ The terminal component must read from the scrollback tape when offset.
- _The Fix:_ Update the PTY component state to track a `scroll_offset`. Update its render
  loop to stitch together lines from `OfsBufVT100.scrollback` and the active
  `OffscreenBuffer` when `scroll_offset > 0`.
- _File(s) Touched:_ `tui/src/core/pty/...` (Specific UI component rendering the PTY)

#### [ ] 2. New Scrollback Example App

- _Context:_ We need a standalone example to visually verify scrollback features (mouse
  wheel up/down, pgup/pgdown).
- _The Fix:_ Create `tui/examples/pty_scrollback_example.rs` that streams highly verbose
  output (like a mock log generator) and allows the user to scroll back up through the
  history using keyboard/mouse.
- _File(s) Touched:_ `tui/examples/pty_scrollback_example.rs`

### Final Verification & Cleanup

- [ ] Verify full test suite coverage using `./check.fish --full`.
- [ ] Ensure all work was done on a new branch (e.g., `feat-scrollback-buffer`), rather than committing directly to `main` or Cecile's divergent branch.
- [ ] When ready to merge, open a PR and include `Supersedes #459` in the description to gracefully close Cecile's draft PR. (Optionally include `Co-authored-by: cecton <email>` in the commit message).
- [ ] Use the `/merge-pr` slash command to cleanly rebase and merge to `main`.
- [ ] Update the meta-task `task/prepare-v0.8.0-meta-task.md` to check off PR #459.
- [ ] **Mandatory manual review:** Verify every file modified in this task.
  - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100.rs`
  - [ ] `tui/src/tui/terminal_lib_backends/vt100/vt_100_impl_control_ops.rs`
  - [ ] `tui/examples/pty_scrollback_example.rs`
  - [ ] Test files...
  - [ ] `task/prepare-v0.8.0-meta-task.md`

---

## Historical Context & PR Divergence

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
2. **Pre-existing `RingBufferHeap`**: Cecile implemented over 200 lines of custom
   ring-buffer logic (`ScrollbackBuffer`). Since our `core` module already provides a
   battle-tested `RingBufferHeap<T, N>`, we can completely drop the custom implementation
   and reuse our robust primitive.

By implementing it this way, we respect and fulfill Cecile's exact feature request while
strictly adhering to the new architectural boundaries of the codebase.

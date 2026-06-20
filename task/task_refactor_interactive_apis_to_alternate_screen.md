# Task: Modernize choose API and readline_async

## Overview

Modernize the `choose` API and `readline_async` API, which are two of the main entry
points into the `r3bl_tui` framework for making an app interactive. Currently, both of
these APIs do not use `OffscreenBuffer` or `RenderOps` (the standard TUI rendering
infrastructure) and instead manage terminal cursor positioning directly inline with the
terminal's native scrollback.

We must consolidate their implementations to leverage the standard `r3bl_tui` code. This
will eliminate a massive amount of complexity and brittle terminal behavior (like screen
tearing and cursor jumping) while unifying the rendering architecture.

### The Architectural Shift (The `fzf` Illusion)

Moving to `OffscreenBuffer` and `RenderOps` means we fundamentally shift our mental model.
We are abandoning the idea of injecting interactive components directly inline with the
terminal's native scrollback buffer.

Instead, both tools will transition to an **"alternate screen"** model, using full raw
mode (similar to `fzf` or `lazygit`). While active, they will own the entire terminal UI,
routing all rendering through `compositor_render_ops_to_ofs_buf`.

Upon completion, they will exit the alternate screen and dump the necessary output back to
the cooked terminal, creating the seamless illusion that the user was working inline the
whole time.

#### `readline_async` Design

- **Concept:** Provide a non-blocking prompt while background tasks emit output.
- **Scrollback Handling:** We will manually implement scrollback. By using standard
  `r3bl_tui` layout mechanics (flexbox, viewports), we gain complete control over mouse
  wheels and keyboard scrolling inside the alternate screen.
- **The Illusion:** When the user exits the loop, we will dump the output/input to the
  cooked terminal. **Open Design Question:** Determine whether it is better to dump the
  raw contents of the final `OffscreenBuffer` or to maintain a separate "virtual
  scrollback" history vector that gets flushed to the standard output upon exit.

#### `choose` Design

- **Concept:** Provide a visual way to select items from a list.
- **Timeout Support:** Add support for a configurable timeout that automatically closes
  the interactive session and returns a default or empty selection if the user takes too
  long.
- **Scrollback Handling:** The unselected list items do not need to persist in terminal
  history. Scrollback is unnecessary.
- **The Illusion:** The list renders in the alternate screen. When the user makes a
  selection and presses `Enter`, the alternate screen is torn down, and we simply
  `println!` the chosen selection to standard output in cooked mode.

## Implementation plan

### Phase 1: Research and design

- [ ] Analyze the exact usage of `choose` and `readline_async` across the repository to
      ensure API compatibility.
- [ ] Design the architecture for `readline_async`'s virtual scrollback. (Decide between
      `OffscreenBuffer` dumping vs. History Vector dumping).
- [ ] Prototype the new `OffscreenBuffer` rendering pipeline for `choose` (alternate
      screen, raw mode initialization, full-screen event loop).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

### Phase 2: Refactor `choose` API

- [ ] Transition `choose_api.rs` to initialize the alternate screen and clear standard
      output device overrides.
- [ ] Replace `crossterm` queueing in `select_component.rs` with a standard `Component`
      implementation emitting `RenderOps`.
- [ ] Implement the tear-down and cooked-mode `println!` of the final selection.
- [ ] Add the timeout feature to gracefully exit and return a default/empty selection when
      the timer expires.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

### Phase 3: Refactor `readline_async` API

- [ ] Adapt `readline.rs` to generate `RenderOps` and render to the `OffscreenBuffer`
      instead of using the `SharedWriter` line-state control loop.
- [ ] Build the custom scrollback viewport / log window for `SharedWriter` concurrent
      output.
- [ ] Implement the exit strategy: dumping the virtual scrollback/buffer to the primary
      terminal in cooked mode.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

### Phase 4: Final polish and testing

- [ ] Run comprehensive tests (`./check.fish --full`) to verify legacy behaviors are
      safely deprecated or matched.
- [ ] Update documentation reflecting the architectural shift to full-screen alternate
      terminal usage for these APIs.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

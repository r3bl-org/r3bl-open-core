# Task: Enable Mouse Support for Editor

## Background

As part of modernizing the editor, we need to add robust mouse support. This will allow
users to interact with the editor fluidly using standard mouse inputs rather than relying
entirely on keyboard shortcuts.

## Goals

1. **Mouse Scrolling:**
    - Support scrolling up and down (vertical).
    - Support scrolling left and right (horizontal).
2. **Mouse Click & Drag:**
    - Support clicking to position the caret.
    - Support clicking and dragging (e.g., to select text or move the caret/viewport).

## Phases

### Phase 1: Research and Design

- [ ] Identify where the editor engine currently handles input events (e.g.,
      `InputEvent::Mouse` or similar `crossterm` events).
- [ ] Determine how to map global terminal mouse coordinates to the editor's local
      canvas/viewport coordinates.
- [ ] Outline the architecture for click-and-drag interactions.

### Phase 2: Implementation - Mouse Scrolling

- [ ] Implement vertical mouse scrolling (mouse wheel up/down).
- [ ] Implement horizontal mouse scrolling (mouse wheel left/right or shift+scroll).
- [ ] Ensure viewport panning logic (`pan_to_include`, etc.) integrates smoothly with
      scroll events.
- [ ] Add tests to verify scrolling bounds.

### Phase 3: Implementation - Click & Drag

- [ ] Implement mouse click to precisely position the canvas caret.
- [ ] Implement mouse drag behavior (tracking the caret while the mouse is held down).
- [ ] Add tests to verify caret positioning via mouse.

### Phase 4: Final Polish and Review

- [ ] Run `./check.fish --full` to verify no regressions.
- [ ] Ensure rustdoc comments are added/updated for all new mouse handling functions.
- [ ] Complete Mandatory Manual Review for all modified files:
    - [ ] `task/editor-mouse-enable.md`

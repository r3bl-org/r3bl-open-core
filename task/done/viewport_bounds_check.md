# Task: Implement ViewportBoundsCheck consistently

## Background

During the execution of recent tasks (`modernize-editor-using-new-units.md`,
`modernize-layout-engine.md`, `ofsbuf_trait_growable_impl.md`, and `refactor-units.md`),
we introduced the `ViewportBoundsCheck` trait
(`tui/src/core/coordinates/bounds_check/viewport_bounds_check.rs`) to elegantly handle
rendering and viewport visibility checks using `[start, start+len)` exclusive bounds.

However, an audit of the editor engine and core coordinate subsystems reveals that we are
not fully utilizing this trait. Instead, we have reverted to manual, raw `usize`
arithmetic to perform bounds checks for viewports. Worse, in some places like the
rendering pipeline, we are using the `CursorBoundsCheck` trait which employs
`index <= length` (inclusive bounds), resulting in an off-by-one rendering bug where lines
can be rendered one row past the viewport height.

## Findings

### 1. Rendering Pipeline Bug (`engine_public_api.rs`)

In `engine_public_api.rs`, the rendering loop checks if it should stop drawing rows.
Currently, it uses `CursorBoundsCheck`:

```rust
/// Uses [cursor-style bounds checking] (`index <= length`) because viewport rendering
/// fills screen space and needs to render at positions [0, length] inclusive.
fn should_stop_rendering(row_index: VPRow, max_display_row_count: VPHeight) -> bool {
    max_display_row_count.check_cursor_position_bounds(row_index)
        == CursorPositionBoundsStatus::Beyond
}
```

**The Problem:** `CursorBoundsCheck` is designed for text editing, where being _at_ the
end of the text (`index == length`) is a valid cursor position. For viewports, `length`
(height) is an exclusive upper bound (`[0, length)`). Because `CursorBoundsCheck`
considers index `length` to be `AtEnd` (not `Beyond`), this code allows the engine to
render one row past the actual viewport bounds.

### 2. Reinventing Viewport Math (`scroll_editor_content.rs`)

In `scroll_editor_content.rs`, we are manually calculating viewport edges and bounds to
determine if the caret has moved off-screen.

For example, in `inc_caret_col_by`:

```rust
let vp_right_edge = viewport_origin.col_index.as_usize() + vp_width.as_usize();
if canvas_caret.col_index.as_usize() >= vp_right_edge {
    // Scroll right...
}
```

**The Problem:** These manual calculations (`< origin` and `>= origin + width`) exactly
mirror the internal logic of `check_viewport_bounds`.

### 3. Simplifying Camera Logic (`canvas_panning.rs`)

Even in the core `canvas_panning.rs` logic, we aren't utilizing `ViewportBoundsCheck`.
Look at how `pan_to_include` is currently implemented for `CRow`:

```rust
if target_pos < current_origin {
    CRow::from(target_pos) // Underflow
} else {
    let max_visible_offset = vp_height.as_usize().saturating_sub(1);
    let bottom_visible_pos = current_origin + max_visible_offset;

    if target_pos > bottom_visible_pos {
        CRow::from(target_pos - max_visible_offset) // Overflow
    } else {
        self // Within
    }
}
```

**The Problem:** Camera panning logic uses manual saturating math and branching rather
than yielding to the semantic `RangeBoundsResult` of `check_viewport_bounds`.

## Execution Plan

### [x] Phase 1: Fix Rendering Bug

- [x] Update `should_stop_rendering` in
      `tui/src/tui/editor/editor_engine/engine_public_api.rs` to use `ViewportBoundsCheck`
      instead of `CursorBoundsCheck`.
- [x] Ensure it properly checks
      `row_index.check_viewport_bounds(vp_row(0), max_display_row_count) == RangeBoundsResult::Overflowed`.
- [x] Update the documentation to reflect that viewport bounds are exclusive
      (`[0, length)`).

### [x] Phase 2: Core Coordinate Math (`canvas_panning.rs`)

- [x] Rewrite `pan_to_include` for `CRow` in
      `tui/src/core/coordinates/canvas/canvas_panning.rs` using
      `match target.check_viewport_bounds(...)`.
- [x] Rewrite `pan_to_include` for `CCol` using the same match pattern.

### [x] Phase 3: Cleanup Editor Scrolling (`scroll_editor_content.rs`)

- [x] Refactor `inc_caret_col_by` (replace manual `vp_right_edge` check with semantic
      panning/bounding).
- [x] Refactor `dec_caret_col_by` (replace manual `< viewport_origin` check with semantic
      panning/bounding).
- [x] Refactor `inc_caret_row` (replace manual `vp_bottom_edge` check with semantic
      panning/bounding).
- [x] Refactor `dec_caret_row` (replace manual `< viewport_origin` check with semantic
      panning/bounding).
- [x] Clean up `c_caret.col_index.as_usize() > line_display_width.as_usize()` using
      `CursorBoundsCheck` where appropriate.
- [x] Clean up `desired_c_caret_row_index.as_usize() > max_row_index.as_usize()` using
      `ArrayBoundsCheck` or `CursorBoundsCheck` where appropriate.

### [x] Phase 4: Cleanup `scroll_editor_content.rs` Panning

- [x] Modify the signature of `dec_caret_col_by` to accept an additional
      `vp_width: VPWidth` parameter.
- [x] Modify the signature of `dec_caret_row` to accept an additional
      `vp_height: VPHeight` parameter.
- [x] Refactor `dec_caret_col_by` to use
      `viewport_origin.col_index = viewport_origin.col_index.pan_to_include(canvas_caret.col_index, vp_width);`
      instead of the manual `< viewport_origin.col_index` conditional assignment.
- [x] Refactor `dec_caret_row` to use
      `viewport_origin.row_index = viewport_origin.row_index.pan_to_include(canvas_caret.row_index, vp_height);`
      instead of the manual `< viewport_origin.row_index` conditional assignment.
- [x] Update all call sites across the codebase (e.g., inside `set_caret_col_to` and
      `change_caret_row_by`) that consume these two functions to pass the respective
      `VPWidth` or `VPHeight` from the engine/context.

### [x] Phase 5: Refactor `viewport.rs` to implement `CanvasCameraExt`

- [x] Rename `pan_to_include_target` to `pan_to_keep_coord_in_view` across trait, methods,
      comments, and tests.
- [x] Refactor `CanvasCameraExt<InputCoord>` trait definition in
      `tui/src/core/coordinates/canvas/canvas_camera_ext.rs` to be generic over input
      coordinate (`CCol`, `CRow`, `CPos`).
- [x] Implement `CanvasCameraExt<CCol>`, `CanvasCameraExt<CRow>`, and
      `CanvasCameraExt<CPos>` on `Viewport` in
      `tui/src/core/coordinates/canvas/canvas_camera_ext.rs`.
- [x] Remove `CanvasCameraExt` trait implementations from `CRow`, `CCol`, and `CPos`
      (keeping coordinates purely as data types).
- [x] Update `to_vp` projection on `Viewport` so `viewport.to_vp(c_pos)` / `c_col` /
      `c_row` projects relative to `viewport.get_origin_pos()`.
- [x] Refactor `pan_viewport_to_include_row` in
      `tui/src/tui/dialog/dialog_engine/dialog_engine_struct.rs` to use a transient
      `Viewport` and `viewport.pan_to_keep_coord_in_view(target_row)`.
- [x] Update all call sites across `editor_engine`, `validate_scroll_on_resize.rs`,
      `scroll_editor_content.rs`, `validate_buffer_mut.rs`, and tests to consume the new
      `Viewport::pan_to_keep_coord_in_view` and `Viewport::to_vp` API.
- [x] Verify everything builds and passes tests (`./check.fish --test`).
- [x] Complete Mandatory Manual Review for Phase 5 modified files:
    - [x] `tui/src/core/coordinates/canvas/canvas_camera_ext.rs`
    - [x] `tui/src/core/coordinates/canvas/viewport.rs`
    - [x] `tui/src/tui/dialog/dialog_engine/dialog_engine_struct.rs`
    - [x] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [x] `tui/src/lib.rs`
    - [x] `task/viewport_bounds_check.md`

### [x] Phase 6: Refactor `dialog_engine_api.rs` Rendering Bounds & Panning

- [x] Inside `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs` within
      `render_results_panel_inner::paint_results`, identify the bounds checking logic that
      skips or breaks the loop using `row_index < scroll_offset_row_index` and
      `row_index >= max_display_row_index`.
- [x] Replace this manual `usize` arithmetic with a
      `match row_index.check_viewport_bounds(scroll_offset_row_index, viewport_height)`.
- [x] Map the `RangeBoundsResult` as follows:
    - `Underflowed` => `continue;`
    - `Within` => Execute the row rendering logic.
    - `Overflowed` => `break;`
- [x] In the same file, locate the keyboard event handlers for `Up` (around line 937) and
      `Down` (around line 962) where `dialog_engine.scroll_offset_row_index` is manually
      incremented/decremented when the `selected_row_index` goes out of bounds.
- [x] Replace this manual panning arithmetic with a single call to
      `CanvasPanningExt::pan_to_include`:
      `dialog_engine.scroll_offset_row_index = dialog_engine.scroll_offset_row_index.pan_to_include(dialog_engine.selected_row_index, results_panel_viewport_height_row_count);`

### [x] Phase 7: Streamline `readline_async` Viewport Classification

- [x] Refactor `locate_cursor_in_viewport` in
      `tui/src/readline_async/choose_impl/scroll.rs` to group its spatial states under a
      `match abs_row_index.check_viewport_bounds(scroll_offset_row_index, display_height)`
      statement.
- [x] Ensure that `AtAbsoluteTop` and `AtAbsoluteBottom` (which depend on `items_size - 1`
      and `ch(0)`) are evaluated before the viewport bounds checking, as they take
      precedence.
- [x] Inside the `Within` match arm, use strict equalities to classify if the index is
      `AtTopOfViewport` (`== scroll_offset_row_index`), `AtBottomOfViewport`
      (`== scroll_offset_row_index + display_height - 1`), or `InMiddleOfViewport`
      (everything else within the bound).

### [x] Phase 8: Document Architecture Synergy in `lib.rs`

- [x] Modify the `## Panning / Scrolling and Component Integration` section in
      `tui/src/lib.rs` (around line 1029).
- [x] Explicitly explain that `CanvasPanningExt::pan_to_include` leverages the
      `ViewportBoundsCheck` trait.
- [x] Detail how `pan_to_include` uses the `RangeBoundsResult` (`Underflowed` to snap
      backward, `Overflowed` to push forward, `Within` for no change) to unify boundary
      math for scrolling across the framework.

### [x] Phase 9: Final Validation

- [x] Check all modified codebase locations.
- [x] Verify everything builds and passes tests with `./check.fish --full`.
- [x] Run skill `remove-crate-prefix` on all the git changed files.
- [x] Complete Mandatory Manual Review for all modified files:
    - [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_panning.rs`
    - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [x] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [x] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [x] `tui/src/readline_async/choose_impl/scroll.rs`
    - [x] `tui/src/lib.rs`
    - [x] `task/viewport_bounds_check.md`

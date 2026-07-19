# Task: Rasterize Editor Component Rendering & Eliminate String-Space Clipping

## Overview

Eliminate the upstream string-space clipping pipeline in the editor component by moving
horizontal viewport clipping downstream to the 2D rasterization stage (the [`OfsBuf`]
compositor).

This replaces the multi-pass string pattern matching state machine
([`PatternMatcherStateMachine`]) and intermediate string allocations with zero-copy 2D
cell grid writing.

```text
BEFORE (Current Multi-Pass String Slicing):
┌──────────────────┐
│ StyleUSSpanLine  │
└────────┬─────────┘
         │ 1. Extract plain text line
         ▼
┌──────────────────┐
│ GCStringOwned    │ ──→ GCStringOwned::clip(scroll_offset, max_cols)
└────────┬─────────┘     (Produces plain_text_pattern)
         │
         │ 2. Character-by-character pattern matching loop
         ▼
┌───────────────────────────────┐
│ PatternMatcherStateMachine    │ ──→ Allocates temporary InlineString & RenderList
└────────┬──────────────────────┘
         │
         │ 3. Emit sliced spans into RenderOps
         ▼
┌──────────────────┐
│ RenderOpIRVec    │ ──→ Compositor prints pre-clipped strings into OfsBuf
└──────────────────┘

AFTER (Direct 2D Rasterization):
┌──────────────────┐
│ StyleUSSpanLine  │ ──→ Emits unclipped styled spans with viewport bounds
└────────┬─────────┘
         │
         │ Single direct pass into OfsBuf row slice
         ▼
┌───────────────────────────────┐
│ Compositor / OfsBuf           │
│ - target_col = col - vp_left  │ ──→ Writes PixelChar directly into cell grid
│ - Discards target_col < 0     │ ──→ Handles wide-char boundary split with Spacer
│ - Stops at max_display_cols   │
└───────────────────────────────┘
```

---

## Architectural Problem & Inefficiencies

Currently, every render frame (e.g., cursor movement, typing, horizontal scrolling):

1. **Redundant Plain-Text Extraction**: For each visible row in the editor,
   [`StyleUSSpanLine::get_plain_text()`] reconstructs an unhighlighted string.
2. **Intermediate Clipping & Allocations**: [`GCStringOwned::clip()`] truncates the plain
   text string to generate a reference pattern.
3. **State Machine Iteration Overhead**: [`PatternMatcherStateMachine`] iterates through
   every character in every styled span, allocating temporary [`InlineString`] and
   [`RenderList`] structures just to produce sliced [`TuiStyledTexts`].
4. **Compositor Double-Work**: The compositor receives these pre-clipped strings and
   performs yet another boundary check before placing characters in [`OfsBuf`].

---

## Target Solution

### 1. Viewport-Aware Cell Rasterization in the Compositor

Extend the compositor or introduce a direct styled text painter that accepts:
- A sequence of styled text spans (`&StyleUSSpanLine` or `&[StyleUSSpan]`).
- The horizontal viewport origin (`vp_origin.col_index`).
- The maximum visible column count (`max_display_col_count`).
- The target screen insertion row and column.

### 2. Wide Character Boundary Handling at the Left Edge

When horizontal scrolling clips into the middle of a 2-column wide character (e.g., CJK
glyph or emoji):
- If the character occupies `col = -1` and `col = 0` relative to the viewport:
  - The left half is off-screen.
  - The right half at `col = 0` cannot render half a glyph.
  - The rasterizer writes [`PixelChar::Spacer`] at column 0 to prevent terminal artifacting.

### 3. Deletion of Vestigial Modules

Completely remove:
- [`PatternMatcherStateMachine`] and [`CharacterMatchResult`].
- `tui/src/tui/syntax_highlighting/pattern_matcher.rs`.
- [`StyleUSSpanLine::clip()`] and associated unit tests in
  `tui/src/tui/syntax_highlighting/intermediate_types.rs`.

---

## Implementation Plan

### [ ] Phase 1: Compositor Viewport-Aware Text Rasterization

Add zero-copy, viewport-bounded rasterization logic to the compositor.

- [ ] Implement `print_styled_spans_with_viewport_bounds()` in
  `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`.
    - Iterate directly over styled spans and grapheme segments (`Seg`).
    - Calculate cell placement: `target_col = span_col - vp_origin_col`.
    - Handle left-edge wide character splitting by inserting a `PixelChar::Spacer` when a
      multi-column segment starts before `vp_origin_col` but overlaps into column 0.
    - Write [`PixelChar::PlainText`] directly into the target row slice.
    - Terminate iteration early when `target_col >= max_display_col_count`.
- [ ] Add comprehensive unit tests in `compositor_render_ops_to_ofs_buf.rs` for:
    - Normal ASCII text with horizontal scroll offsets.
    - Multi-byte emojis and CJK wide characters at scroll boundaries.
    - Multi-styled spans sliced across left and right edges.
- [ ] **Mandatory manual review:** Verify compositor rasterization logic and tests.
    - [ ] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`

---

### [ ] Phase 2: Update Editor Rendering Pipeline

Migrate editor rendering to pass unclipped styled spans directly to the compositor.

- [ ] Update `RenderOpIR` or `RenderOpCommon` to carry viewport-bounded styled text
  operations if needed, or update `render_content` in
  `tui/src/tui/editor/editor_engine/engine_public_api.rs`.
- [ ] In `tui/src/tui/editor/editor_engine/engine_public_api.rs`:
    - Remove calls to `line.clip(vp_origin, max_display_col_count)`.
    - Emit styled spans directly for each visible line (`skip(vp_origin.row_index)` up to
      `max_display_row_count`).
- [ ] Run `./check.fish --check` to verify compiler validation.
- [ ] **Mandatory manual review:** Verify editor engine rendering changes.
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`

---

### [ ] Phase 3: Remove `PatternMatcherStateMachine` & Clean Up

Eradicate the legacy pattern matching and string-slicing code.

- [ ] Delete `tui/src/tui/syntax_highlighting/pattern_matcher.rs`.
- [ ] Remove `pub mod pattern_matcher;` and `pub use pattern_matcher::*;` from
  `tui/src/tui/syntax_highlighting/mod.rs`.
- [ ] Remove `StyleUSSpanLine::clip()` and `StyleUSSpanLine::get_plain_text_clipped()`
  from `tui/src/tui/syntax_highlighting/intermediate_types.rs`.
- [ ] Clean up tests in `intermediate_types.rs` that tested `clip()`.
- [ ] Run `./check.fish --check` and `./check.fish --clippy`.
- [ ] **Mandatory manual review:** Verify clean removal of vestigial pattern matching.
    - [ ] `tui/src/tui/syntax_highlighting/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/intermediate_types.rs`

---

### [ ] Phase 4: Integration Testing & Verification

Ensure no visual regressions, verify emoji/wide-char behavior, and measure performance.

- [ ] Verify existing editor rendering tests and add new integration tests in
  `tui/src/tui/editor/test_fixtures_editor.rs` and
  `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/`.
- [ ] Test interactive horizontal scrolling in editor examples (`cargo run --example
  ex_editor`).
- [ ] Run `./check.fish --full` to verify full workspace compilation, clippy, doctests, and
  tests.
- [ ] Run skill `remove-crate-prefix` on all modified files.
- [ ] **Mandatory manual review:** Verify all modified files across the task.
    - [ ] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/syntax_highlighting/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/intermediate_types.rs`
    - [ ] `task/rasterize-editor-component-rendering.md`

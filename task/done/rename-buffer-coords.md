# Task: Rename Buffer Coordinates & Eliminate ViewportOrigin

## 1. Overview & Goals

This refactor streamlines coordinate domain types across `r3bl_tui`:

1. **Explicit Viewport Type Alignment**: Rename base 0-based `u16` primitives (`Pos`,
   `Size`, `RowIndex`, `ColIndex`, `RowHeight`, `ColWidth`) to explicit `Viewport*` /
   `VP*` names (`VPPos`, `VPSize`, `VPRowIndex`, `VPColIndex`, `VPRowHeight`,
   `VPColWidth`) for zero ambiguity between viewport screen space (`u16`) and canvas
   storage space (`usize`).
2. **`ViewportOrigin` Elimination**: Remove the redundant `ViewportOrigin(pub Pos)`
   wrapper and migrate caret math (`caret.rs`, `EditorBuffer`) to use `CanvasPos` (`CPos`)
   or `Viewport::get_origin_pos()`.
3. **Module Taxonomy Cleanup**: Rename `tui/src/core/coordinates/buffer_coords/` ->
   `tui/src/core/coordinates/viewport_coords/` (or `viewport/`), creating a clean,
   symmetrical pairing with `tui/src/core/coordinates/canvas/`.
4. **Rustdoc & Quality Verification**: Update intra-doc links, taxonomy tables, and verify
   clean compilation via `./check.fish`.

---

## 2. Architecture & Domain Flow

- [x] **Done**: Architecture & Domain Flow diagram added to rustdocs at
      [`tui/src/core/coordinates/canvas/mod.rs`](../tui/src/core/coordinates/canvas/mod.rs#L43-L72).

---

## 3. Step-by-Step Refactoring Plan

### [x] Step 1: Global Type & Directory Rename (VSCode / User Execution)

1. [x] **Rename Module Directory**:
    - `tui/src/core/coordinates/buffer_coords/` ->
      `tui/src/core/coordinates/viewport_coords/`

2. [x] **Perform Global Symbol Renames in VSCode across `tui/src/`**:

| Current Name        | New Explicit Name | Type Alias | Domain / Backing Primitive    |
| :------------------ | :---------------- | :--------- | :---------------------------- |
| [x] `buffer_coords` | `viewport_coords` | n/a        | Module directory name         |
| [x] `Pos`           | `VPPos`           | n/a        | 0-based `u16` screen position |
| [x] `Size`          | `VPSize`          | n/a        | 1-based `u16` screen extent   |
| [x] `RowIndex`      | `VPRowIndex`      | `VPRow`    | 0-based `u16` screen row      |
| [x] `ColIndex`      | `VPColIndex`      | `VPCol`    | 0-based `u16` screen col      |
| [x] `RowHeight`     | `VPRowHeight`     | `VPHeight` | 1-based `u16` screen height   |
| [x] `ColWidth`      | `VPColWidth`      | `VPWidth`  | 1-based `u16` screen width    |

---

### [x] Step 2: Remove `ViewportOrigin` & Migrate Caret Math (Agent Execution)

- [x] **Delete `ViewportOrigin` Module**:
    - [x] Remove `tui/src/core/coordinates/viewport_coords/viewport_origin.rs`.
    - [x] Remove `pub mod viewport_origin;` and `pub use viewport_origin::*;` from
          [`tui/src/core/coordinates/viewport_coords/mod.rs`](tui/src/core/coordinates/viewport_coords/mod.rs).
    - [x] Remove `viewport_origin` re-export from
          [`tui/src/core/coordinates/mod.rs`](tui/src/core/coordinates/mod.rs).

- [x] **Migrate Caret Math
      ([`caret.rs`](tui/src/core/coordinates/viewport_coords/caret.rs))**:
    - [x] Replace all `(ViewportCaret, ViewportOrigin)` math with
          `(ViewportCaret, CanvasPos)` -> `CanvasCaret`.
    - [x] Implement `Add<CanvasPos>` for `ViewportCaret` -> `CanvasCaret`
          (`viewport_caret + scroll_offset` -> `canvas_caret`).
    - [x] Implement `Add<ViewportCaret>` for `CanvasPos` -> `CanvasCaret`
          (`scroll_offset + viewport_caret` -> `canvas_caret`).
    - [x] Implement `Add<CanvasPos>` for `CanvasCaret` -> `ViewportCaret`
          (`canvas_caret + scroll_offset` -> `viewport_caret`).
    - [x] Implement `From<(ViewportCaret, CanvasPos)>` and
          `From<(CanvasPos, ViewportCaret)>` for `CanvasCaret`.
    - [x] Implement `From<(CanvasCaret, CanvasPos)>` and `From<(CanvasPos, CanvasCaret)>`
          for `ViewportCaret`.
    - [x] Delete all obsolete `ViewportOrigin` impls and update rustdocs/unit tests in
          `caret.rs`.

- [x] **Migrate `EditorBuffer` & Accessors
      ([`buffer_struct.rs`](tui/src/tui/editor/editor_buffer/buffer_struct.rs))**:
    - [x] In `EditorContent` struct: change `pub viewport_origin: ViewportOrigin` field ->
          `pub viewport_origin: CanvasPos`.
    - [x] Update `EditorContent::default()` to initialize
          `viewport_origin: CanvasPos::default()`.
    - [x] Update `EditorBuffer` accessor:
          `pub fn get_viewport_origin(&self) -> CanvasPos`.
    - [x] Update `get_canvas_caret()` implementation:
          `self.content.viewport_caret + self.content.viewport_origin`.

- [x] **Update Editor Callers & Engine Operations**:
    - [x] [`render_cache.rs`](tui/src/tui/editor/editor_buffer/render_cache.rs): Update
          `viewport_origin` key to type `CanvasPos`.
    - [x] [`caret_locate.rs`](tui/src/tui/editor/editor_buffer/caret_locate.rs): Update
          locator math to use `CanvasPos` for `viewport_origin`.
    - [x] [`caret_mut.rs`](tui/src/tui/editor/editor_engine/caret_mut.rs),
          [`engine_public_api.rs`](tui/src/tui/editor/editor_engine/engine_public_api.rs),
          [`scroll_editor_content.rs`](tui/src/tui/editor/editor_engine/scroll_editor_content.rs),
          [`validate_buffer_mut.rs`](tui/src/tui/editor/editor_engine/validate_buffer_mut.rs),
          [`validate_scroll_on_resize.rs`](tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs):
          Update all caret positioning and scroll validation logic to use
          `viewport_origin: CanvasPos`.

- [x] **Local Verification**:
    - [x] Run `./check.fish --check`, `./check.fish --build`, and `./check.fish --test`.

- [x] **Mandatory manual review**:
    - [x] [`tui/src/tui/editor/editor_buffer/buffer_struct.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/buffer_struct.rs)
    - [x] [`tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_engine/validate_buffer_mut.rs)
    - [x] [`tui/src/tui/editor/editor_buffer/render_cache.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/render_cache.rs)
    - [x] [`tui/src/tui/editor/editor_buffer/caret_locate.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/caret_locate.rs)
    - [x] [`tui/src/tui/editor/editor_engine/content_mut.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_engine/content_mut.rs)
    - [x] [`tui/src/tui/editor/editor_engine/scroll_editor_content.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_engine/scroll_editor_content.rs)
    - [x] [`tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs)
    - [x] [`tui/src/tui/editor/editor_buffer/selection_range.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_buffer/selection_range.rs)
    - [x] [`tui/src/tui/editor/editor_engine/engine_public_api.rs`](file:///home/nazmul/github/roc/tui/src/tui/editor/editor_engine/engine_public_api.rs)
    - [x] [`tui/src/tui/syntax_highlighting/intermediate_types.rs`](file:///home/nazmul/github/roc/tui/src/tui/syntax_highlighting/intermediate_types.rs)

---

### [x] Step 3: Comprehensive Rustdoc & Intra-Doc Link Cleanup (Agent Execution)

Update all stale rustdoc comments, taxonomy tables, ASCII sitemaps, doc comments, code
snippets in documentation, and intra-doc reference links across the codebase to reflect
renamed types (`VPPos`, `VPSize`, `VPRowIndex`, `VPColIndex`, `VPRowHeight`,
`VPColWidth`), renamed module (`viewport_coords`), and the removal of `ViewportOrigin`.

- [x] **Phase 1: Core Coordinate Taxonomy & Module Top-Level Docs**
    - [x] Update overview table, sitemaps (`buffer_coords` -> `viewport_coords`), and
          reference links in [`tui/src/lib.rs`](tui/src/lib.rs).
    - [x] Update taxonomy tables and sitemaps in
          [`tui/src/core/coordinates/mod.rs`](tui/src/core/coordinates/mod.rs).
    - [x] Update module documentation and re-export links in
          [`tui/src/core/coordinates/viewport_coords/mod.rs`](tui/src/core/coordinates/viewport_coords/mod.rs).
    - [x] Update architecture diagram and remove `ViewportOrigin` doc references in
          [`tui/src/core/coordinates/canvas/mod.rs`](tui/src/core/coordinates/canvas/mod.rs).
    - [x] Update module links referencing `buffer_coords` in
          [`tui/src/core/coordinates/primitives/mod.rs`](tui/src/core/coordinates/primitives/mod.rs).
    - [x] Update module links in
          [`tui/src/core/coordinates/percent_spec/mod.rs`](tui/src/core/coordinates/percent_spec/mod.rs)
          and
          [`tui/src/core/coordinates/percent_spec/req_size_pc.rs`](tui/src/core/coordinates/percent_spec/req_size_pc.rs).
    - [x] Update module links in
          [`tui/src/core/coordinates/vt_100_ansi_coords/mod.rs`](tui/src/core/coordinates/vt_100_ansi_coords/mod.rs).

- [x] **Phase 2: Viewport & Canvas Primitive Types Docs**
    - [x] Update `Pos` -> `VPPos`, `RowIndex`/`ColIndex` -> `VPRowIndex`/`VPColIndex`, and
          remove `ViewportOrigin` references in
          [`tui/src/core/coordinates/viewport_coords/pos.rs`](tui/src/core/coordinates/viewport_coords/pos.rs).
    - [x] Update `Size` -> `VPSize` in
          [`tui/src/core/coordinates/viewport_coords/size.rs`](tui/src/core/coordinates/viewport_coords/size.rs).
    - [x] Update
          [`tui/src/core/coordinates/viewport_coords/row_index.rs`](tui/src/core/coordinates/viewport_coords/row_index.rs).
    - [x] Update
          [`tui/src/core/coordinates/viewport_coords/col_index.rs`](tui/src/core/coordinates/viewport_coords/col_index.rs).
    - [x] Update
          [`tui/src/core/coordinates/viewport_coords/row_height.rs`](tui/src/core/coordinates/viewport_coords/row_height.rs).
    - [x] Update
          [`tui/src/core/coordinates/viewport_coords/col_width.rs`](tui/src/core/coordinates/viewport_coords/col_width.rs).
    - [x] Update
          [`tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`](tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs).
    - [x] Update caret math doc comments (`ViewportCaret` + `CanvasPos` -> `CanvasCaret`)
          in
          [`tui/src/core/coordinates/viewport_coords/caret.rs`](tui/src/core/coordinates/viewport_coords/caret.rs).
    - [x] Clean up or remove deprecated doc file
          [`tui/src/core/coordinates/viewport_coords/viewport_origin.rs`](tui/src/core/coordinates/viewport_coords/viewport_origin.rs).

- [x] **Phase 3: Bounds Checking & Range Operations Docs**
    - [x] Update trait and type pairings in doc comments and intra-doc links in
          [`tui/src/core/coordinates/bounds_check/mod.rs`](tui/src/core/coordinates/bounds_check/mod.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/array_bounds_check.rs`](tui/src/core/coordinates/bounds_check/array_bounds_check.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs`](tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/index_ops.rs`](tui/src/core/coordinates/bounds_check/index_ops.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/length_ops.rs`](tui/src/core/coordinates/bounds_check/length_ops.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/numeric_value.rs`](tui/src/core/coordinates/bounds_check/numeric_value.rs).
    - [x] Update range sub-module docs in
          [`tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`](tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`](tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`](tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs).
    - [x] Update
          [`tui/src/core/coordinates/bounds_check/range/range_ext.rs`](tui/src/core/coordinates/bounds_check/range/range_ext.rs).

- [x] **Phase 4: VT-100, ANSI & PTY Subsystems Docs**
    - [x] Update doc comments and links in
          [`tui/src/core/ansi/generator/cli_text.rs`](tui/src/core/ansi/generator/cli_text.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/config.rs`](tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/config.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs`](tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_cursor_ops.rs`](tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_cursor_ops.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_pty_output_parser/protocols/params_ext.rs`](tui/src/core/ansi/vt_100_pty_output_parser/protocols/params_ext.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`](tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs).
    - [x] Update
          [`tui/src/core/ansi/vt_100_terminal_input_parser/utf8.rs`](tui/src/core/ansi/vt_100_terminal_input_parser/utf8.rs).
    - [x] Update
          [`tui/src/core/pty/pty_engine/pty_size.rs`](tui/src/core/pty/pty_engine/pty_size.rs).
    - [x] Update
          [`tui/src/core/pty/pty_mux/scrollback_amount.rs`](tui/src/core/pty/pty_mux/scrollback_amount.rs).
    - [x] Update
          [`tui/src/core/pty/pty_session/pty_session_builder.rs`](tui/src/core/pty/pty_session/pty_session_builder.rs).
    - [x] Update
          [`tui/src/core/terminal_io/input_event.rs`](tui/src/core/terminal_io/input_event.rs).
    - [x] Update
          [`tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`](tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs).
    - [x] Update [`tui/src/core/graphemes/mod.rs`](tui/src/core/graphemes/mod.rs).
    - [x] Update
          [`tui/src/core/graphemes/unicode_segment/seg.rs`](tui/src/core/graphemes/unicode_segment/seg.rs).
    - [x] Update
          [`tui/src/core/graphemes/gc_string/owned/gc_string_owned_editor_impl.rs`](tui/src/core/graphemes/gc_string/owned/gc_string_owned_editor_impl.rs).
    - [x] Update
          [`tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`](tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs).
    - [x] Update
          [`tui/src/readline_async/readline_async_impl/line_state/mod.rs`](tui/src/readline_async/readline_async_impl/line_state/mod.rs).

- [x] **Phase 5: Layout, RenderOps & Editor Subsystems Docs**
    - [x] Update
          [`tui/src/tui/layout/bounding_box.rs`](tui/src/tui/layout/bounding_box.rs).
    - [x] Update [`tui/src/tui/layout/flex_box.rs`](tui/src/tui/layout/flex_box.rs).
    - [x] Update [`tui/src/tui/layout/flex_box_id.rs`](tui/src/tui/layout/flex_box_id.rs).
    - [x] Update
          [`tui/src/tui/layout/layout_and_positioning_traits.rs`](tui/src/tui/layout/layout_and_positioning_traits.rs).
    - [x] Update [`tui/src/tui/layout/props.rs`](tui/src/tui/layout/props.rs).
    - [x] Update [`tui/src/tui/layout/surface.rs`](tui/src/tui/layout/surface.rs).
    - [x] Update
          [`tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs`](tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_buffer/buffer_struct.rs`](tui/src/tui/editor/editor_buffer/buffer_struct.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_buffer/render_cache.rs`](tui/src/tui/editor/editor_buffer/render_cache.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_buffer/selection_list.rs`](tui/src/tui/editor/editor_buffer/selection_list.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_buffer/selection_range.rs`](tui/src/tui/editor/editor_buffer/selection_range.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_buffer/sizing.rs`](tui/src/tui/editor/editor_buffer/sizing.rs).
    - [x] Update
          [`tui/src/tui/editor/editor_engine/scroll_editor_content.rs`](tui/src/tui/editor/editor_engine/scroll_editor_content.rs).

- [x] **Phase 6: Documentation Formatting & Mandatory Manual Review**
    - [x] Format doc comments with `cargo rustdoc-fmt` across all modified files.
    - [x] Ensure all intra-doc reference links are placed at the bottom of doc blocks.
    - [x] Verify ASCII hyphen compliance (no connecting dashes, en dashes, or em dashes).
    - [x] **Mandatory manual review:** Verify all rustdoc modifications across all 57
          files:
        - [x] `tui/src/lib.rs`
        - [x] `tui/src/core/coordinates/mod.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/mod.rs`
        - [x] `tui/src/core/coordinates/canvas/mod.rs`
        - [x] `tui/src/core/coordinates/primitives/mod.rs`
        - [x] `tui/src/core/coordinates/percent_spec/mod.rs`
        - [x] `tui/src/core/coordinates/percent_spec/req_size_pc.rs`
        - [x] `tui/src/core/coordinates/vt_100_ansi_coords/mod.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/pos.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/size.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/row_index.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/col_index.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/row_height.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/col_width.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/caret.rs`
        - [x] `tui/src/core/coordinates/viewport_coords/viewport_origin.rs`
        - [x] `tui/src/core/coordinates/bounds_check/mod.rs`
        - [x] `tui/src/core/coordinates/bounds_check/array_bounds_check.rs`
        - [x] `tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs`
        - [x] `tui/src/core/coordinates/bounds_check/index_ops.rs`
        - [x] `tui/src/core/coordinates/bounds_check/length_ops.rs`
        - [x] `tui/src/core/coordinates/bounds_check/numeric_value.rs`
        - [x] `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`
        - [x] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
        - [x] `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`
        - [x] `tui/src/core/coordinates/bounds_check/range/range_ext.rs`
        - [x] `tui/src/core/ansi/generator/cli_text.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/config.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_cursor_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/params_ext.rs`
        - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
        - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/utf8.rs`
        - [x] `tui/src/core/pty/pty_engine/pty_size.rs`
        - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
        - [x] `tui/src/core/pty/pty_session/pty_session_builder.rs`
        - [x] `tui/src/core/terminal_io/input_event.rs`
        - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
        - [x] `tui/src/core/graphemes/mod.rs`
        - [x] `tui/src/core/graphemes/unicode_segment/seg.rs`
        - [x] `tui/src/core/graphemes/gc_string/owned/gc_string_owned_editor_impl.rs`
        - [x] `tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`
        - [x] `tui/src/readline_async/readline_async_impl/line_state/mod.rs`
        - [x] `tui/src/tui/layout/bounding_box.rs`
        - [x] `tui/src/tui/layout/flex_box.rs`
        - [x] `tui/src/tui/layout/flex_box_id.rs`
        - [x] `tui/src/tui/layout/layout_and_positioning_traits.rs`
        - [x] `tui/src/tui/layout/props.rs`
        - [x] `tui/src/tui/layout/surface.rs`
        - [x] `tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs`
        - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
        - [x] `tui/src/tui/editor/editor_buffer/render_cache.rs`
        - [x] `tui/src/tui/editor/editor_buffer/selection_list.rs`
        - [x] `tui/src/tui/editor/editor_buffer/selection_range.rs`
        - [x] `tui/src/tui/editor/editor_buffer/sizing.rs`
        - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`

---

### [x] Step 4: Implement Canvas Panning & Projection (canvas_panning.rs)

Created a new file `canvas/canvas_panning.rs` containing the `CanvasPanningExt` trait to
cleanly isolate coordinate projection and panning logic:

- Added `pan_to_include()` to adjust the `viewport_origin` based on the caret position.
- Added `to_vp()` to compute the relative `Viewport` coordinate from an absolute `Canvas`
  coordinate and an origin.
- Refactored `validate_scroll_on_resize.rs` to use `pan_to_include` and `to_vp`,
  eliminating the manual `ArrayOverflowResult` branching.
- Refactored `engine_public_api.rs` and `validate_buffer_mut.rs` to use the `to_vp()`
  projection math instead of manually subtracting primitive types.
- Updated `Sub` implementations for `CanvasRowIndex` and `CanvasColIndex` to return
  semantic types (`CanvasRowHeight` and `CanvasColWidth` respectively) instead of
  returning `Self`.

### [x] Step 5: Quality Checks & Verification (Agent Execution)

Scope validation exclusively to documentation building while code refactoring is in
progress:

1. [x] `./check.fish --quick-doc` (verify workspace doc generation completes without
       broken intra-doc link warnings).
2. [x] _(Deferred until code compilation is restored: `./check.fish --check`,
       `./check.fish --clippy`, `./check.fish --test`)._ (Completed successfully)

### [x] Step 6: Mandatory Manual Review

- [x] `tui/src/core/coordinates/canvas/canvas_coords.rs`
- [x] `tui/src/core/coordinates/canvas/canvas_panning.rs`
- [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
- [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
- [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`
- [x] `tui/src/core/coordinates/viewport_coords/size.rs`
- [x] `tui/src/core/coordinates/viewport_coords/pos.rs`
- [x] `tui/src/core/coordinates/viewport_coords/caret.rs`
- [x] `tui/src/core/coordinates/bounds_check/numeric_value.rs`
- [x] `tui/src/core/coordinates/bounds_check/range/range_ext.rs`
- [x] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
- [x] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
- [x] `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
- [x] `tui/src/core/pty/pty_engine/pty_pair.rs`
- [x] `tui/src/core/terminal_io/mouse_input.rs`
- [x] `tui/src/core/misc/formatter.rs`
- [x] `tui/src/tui/terminal_lib_backends/render_op/render_ops_exec.rs`

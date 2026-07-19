# Task: Migrate `scroll_offset` to `vp_origin` in `r3bl_tui::editor`

## Overview

Standardize coordinate terminology across the editor crate by replacing vestigial
`scroll_offset` usages with `vp_origin` (`viewport.origin_pos`). This unifies API method
names, enum types, internal helper functions, tracing logs, comments, and ASCII diagrams
with the core Canvas vs Viewport coordinate domain model.

## Implementation Plan

### Phase 1: Core Selection Range API (`selection_range.rs`)

- [x] Replace `ScrollOffsetColLocationInRange` with `SelectionStartVPLocation`
      (`VisibleInsideVP`, `NotVisibleAtLeftOfVPOrigin`)
- [x] Implement `locate_start_rel_to_vp_origin` using half-open range `.contains(...)`
      (`vp_origin.col_index..`)
- [x] Implement `clip_left_to_vp_origin` encapsulating left-edge selection clipping
- [x] Update existing unit tests and add new tests for `locate_start_rel_to_vp_origin` and
      `clip_left_to_vp_origin`

### Phase 2: Selection Rendering (`engine_public_api.rs`)

- [x] Simplify `render_selection` to call
      `sel_range.clip_left_to_vp_origin(vp_origin, row_index).clip_to_range_str(line_with_info)`
- [x] Remove `ScrollOffsetColLocationInRange` from imports
- [x] Update tracing debug key `scroll_offset = ?vp_origin` to `vp_origin = ?vp_origin`
- [x] Rename remaining local variables like `scroll_offset_col_index` to `vp_origin_col`

### Phase 3: Grapheme Validation Functions (`validate_buffer_mut.rs` & `scroll_editor_content.rs`)

- [x] Rename `is_scroll_offset_in_middle_of_grapheme_cluster` to
      `is_vp_origin_in_middle_of_grapheme_cluster`
- [x] Rename `adjust_scroll_offset_because_in_middle_of_grapheme_cluster` to
      `adjust_vp_origin_because_in_middle_of_grapheme_cluster`
- [x] Rename internal variables and test functions accordingly

### Phase 4: Documentation & RenderCache (`buffer_struct.rs` & `render_cache.rs`)

- [x] Update ASCII diagrams in `buffer_struct.rs` replacing `scroll_offset` with
      `vp_origin`
- [x] Update doc comments and test names in `render_cache.rs`
      (`test_scroll_offset_change_causes_cache_miss` to
      `test_vp_origin_change_causes_cache_miss`)

### Phase 5: Verification & Review

- [x] Run `./check.fish --check`, `--fmt`, `--clippy`, `--test` to verify builds and tests
- [x] Mandatory manual review checklist
    - [x] `task/migrate-scroll-offset-to-vp-origin.md`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection/selection_range.rs`
    - [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

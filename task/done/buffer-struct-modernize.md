# Task: Modernize `EditorBuffer` & `EditorContent` Encapsulation and Configuration DSL

## Overview

The `EditorBuffer` and `EditorContent` structs in
`tui/src/tui/editor/editor_buffer/buffer_struct.rs` currently have `pub` fields. This
allowed external code (like `cmdr/src/edi/app_main.rs`) to directly mutate internal
content fields (e.g. `editor_buffer.content.maybe_file_path`) and bypass validation or
cache invalidation.

This task modernizes `EditorBuffer` by:

1. Making all fields on `EditorBuffer` and `EditorContent` strictly private (`pub`
   removed).
2. Adding explicit metadata setters (`set_file_path`, `set_file_extension`) on
   `EditorBuffer` that clear caches upon update.
3. Introducing `pub mod editor_buffer_config` with a composable `Add` DSL
   (`FileExtension` + `FilePath`) for initializing `EditorBuffer::new_empty`.
4. Updating all call sites across `cmdr`, `tui` examples, and unit tests to use the clean
   public API.

---

## Implementation Plan

### Phase 1: Create `pub mod editor_buffer_config` & Private Encapsulation

- [x] In `tui/src/tui/editor/editor_buffer/buffer_struct.rs`:
    - [x] Make all fields on `EditorBuffer` strictly private (`content`, `history`,
          `render_cache`, `memory_size_calc_cache`).
    - [x] Make all fields on `EditorContent` strictly private (`lines`, `c_caret`,
          `viewport`, `maybe_file_extension`, `maybe_file_path`, `sel_list`).
    - [x] Add `pub mod editor_buffer_config` containing `FileExtension<'a>`,
          `FilePath<'a>`, `EditorBufferConfig`, `From` implementations for `()` and single
          attributes, and `Add` trait implementations for DSL composition
          (`FileExtension` + `FilePath`).
    - [x] Re-export `pub use editor_buffer_config::*;` in `buffer_struct.rs`.
    - [x] Update `EditorBuffer::new_empty` signature to
          `pub fn new_empty(arg_config: impl Into<EditorBufferConfig>) -> Self`.
    - [x] Add `pub fn set_file_path(&mut self, path: impl Into<InlineString>)` and
          `pub fn set_file_extension(&mut self, ext: impl Into<TinyInlineString>)` to
          `EditorBuffer`.

### Phase 2: Refactor Call Sites Across Workspace

- [x] Update `cmdr/src/edi/app_main.rs`:
    - [x] Replace `editor_buffer.content.maybe_file_path.clone()` with
          `editor_buffer.get_file_path()`.
    - [x] Replace direct mutation of `content.maybe_file_path` and
          `content.maybe_file_extension` with `set_file_path` and `set_file_extension`.
- [x] Update `cmdr/src/edi/state.rs`:
    - [x] Migrate `EditorBuffer::new_empty` call site to use `FileExtension` + `FilePath`
          DSL.
- [x] Update `tui/examples/tui_apps/ex_editor/app_main.rs`:
    - [x] Migrate `EditorBuffer::new_empty(None, None)` to `EditorBuffer::new_empty(())`.
- [x] Update `tui/examples/tui_apps/ex_editor/state.rs`, `ex_pitch/state.rs`,
      `ex_rc/state.rs`:
    - [x] Migrate `EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None)` to
          `EditorBuffer::new_empty(FileExtension(DEFAULT_SYN_HI_FILE_EXT))`.
- [x] Update `tui/src/tui/editor/editor_buffer/history/editor_history.rs` and
      `buffer_struct.rs` unit tests:
    - [x] Refactor unit tests accessing `buffer.content` directly to use public accessors
          and methods (`buffer.add()`, `buffer.get_lines()`).

### Phase 3: Verification & Quality Checks

- [x] Run `./check.fish --check` to verify clean workspace typechecking.
- [x] Run `./check.fish --clippy` to verify zero clippy warnings.
- [x] Run `./check.fish --quick-doc` to verify rustdoc builds cleanly without unresolved
      link warnings.
- [x] Run `./check.fish --test` to verify all workspace tests pass.
- [x] Run `cargo fmt --all` for final code formatting.

### Phase 4: Comprehensive Unit Test Coverage for `buffer_struct.rs`

- [x] Add `test_editor_buffer_config_dsl` to test `FilePath`, tuple `From`, and `Add` DSL
      implementations.
- [x] Add `test_display_and_debug_impls` to test `Display` and `Debug` formatting for
      `EditorBuffer` and `EditorContent`.
- [x] Add `test_content_near_caret_helpers` to test `line_at_caret_is_empty`,
      `line_at_c_caret`, `seg_at_end_of_line_at_c_caret`, `seg_to_right_of_caret`,
      `seg_to_left_of_caret`, `prev_line_above_caret`, and
      `next_line_below_caret_to_string`.
- [x] Add `test_content_display_width_helpers` to test `get_max_row_index`,
      `get_line_display_width_at_c_caret`, and `get_line_display_width_at_row_index`.
- [x] Add `test_access_and_mutate_helpers` to test metadata setters, string formatters,
      caret/viewport getters, and struct accessors.

### Phase 6: Extract `buffer_config_struct.rs`

- [x] Create `tui/src/tui/editor/editor_buffer/buffer_config_struct.rs`.
- [x] Move `EditorBufferConfig`, `FileExtension`, `FilePath`, and
      `impl_editor_buffer_config` from `buffer_struct.rs` to `buffer_config_struct.rs`.
- [x] Re-export `mod buffer_config_struct` in `tui/src/tui/editor/editor_buffer/mod.rs`.
- [x] Update imports in `buffer_struct.rs` and verify codebase builds cleanly
      (`./check.fish --check`).
- [x] Run clippy, docs, tests, and formatting checks.

### Phase 7: Rename Option Types to `*Token` and Standardize Constructor DSL Documentation

- [x] In `tui/src/tui/editor/editor_buffer/buffer_config_struct.rs`:
    - [x] Rename `FileExtensionOption` to `FileExtensionToken`.
    - [x] Rename `FilePathOption` to `FilePathToken`.
    - [x] Update `EditorBufferConfig`, `From`, `Add`, and test implementations.
    - [x] Add `impl_elegant_constructor_dsl_pattern` module doc explaining "Constructor
          DSL Tokens vs Storage Types".
- [x] In `tui/src/tui/editor/editor_buffer/buffer_struct.rs`:
    - [x] Update doc comments and tests to use `FileExtensionToken` and `FilePathToken`.
- [x] In call sites across `tui` and `cmdr`:
    - [x] Update `cmdr/src/edi/state.rs`.
    - [x] Update `tui/examples/tui_apps/ex_editor/state.rs`, `ex_pitch/state.rs`,
          `ex_rc/state.rs`.
    - [x] Update `tui/src/tui/dialog/dialog_buffer/dialog_buffer_struct.rs`.
    - [x] Update `tui/src/tui/editor/editor_buffer/clipboard/clipboard_support.rs`.
    - [x] Update `tui/src/tui/editor/editor_component/editor_component_struct.rs`,
          `editor_event.rs`.
    - [x] Update `tui/src/tui/editor/editor_engine/caret_mut.rs`, `content_mut.rs`,
          `engine_internal_api.rs`, `scroll_editor_content.rs`,
          `validate_scroll_on_resize.rs`.
- [x] Standardize and enrich doc comments on all `// XMARK: Elegant Constructor DSL.`
      modules:
    - [x] `tui/src/core/common/telemetry.rs`
    - [x] `tui/src/core/log/log_public_api.rs`
    - [x] `tui/src/core/pty/pty_session/pty_session_builder.rs`
    - [x] `tui/src/core/tui_style/tui_style_attribs.rs`
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --test`, and
      `./check.fish --quick-doc` to verify code quality.

### Phase 8: Migrate HashMap to FxHashMap across Workspace & Document in AGENTS.md

- [x] Add `rustc-hash = "2.1.0"` to `cmdr/Cargo.toml` and `build-infra/Cargo.toml`.
- [x] In `cmdr/src/edi/state.rs`: Replace `std::collections::HashMap` with
      `rustc_hash::FxHashMap`.
- [x] In `tui/src`:
    - [x] `tui/src/tui/terminal_window/manage_focus/component_registry.rs`: Replace
          `HashMap` with `FxHashMap`.
    - [x] `tui/src/core/common/ordered_map.rs`: Replace `HashMap` with `FxHashMap`.
    - [x] `tui/src/core/common/string_repeat_cache.rs`: Replace `HashMap` with
          `FxHashMap`.
    - [x] `tui/src/core/common/telemetry.rs`: Replace `HashMap` with `FxHashMap`.
    - [x] `tui/src/core/pty/pty_session/pty_session_builder.rs`: Replace `HashMap` with
          `FxHashMap`.
    - [x] Update test files (`test_dialog.rs`, `test_fixtures_editor.rs`,
          `backend_compat_input_test.rs`, `compositor_render_ops_to_ofs_buf.rs`).
- [x] In `build-infra/src`:
    - [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_dictionary.rs`: Replace
          `HashMap` with `FxHashMap`.
    - [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs`: Replace `HashMap`
          with `FxHashMap`.
- [x] Update `AGENTS.md` and `.agents/skills/` to mandate `FxHashMap` / `FxHashSet` over
      standard `HashMap` / `HashSet`.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --test`, and
      `./check.fish --quick-doc`.
- [x] Run `cargo install --path build-infra --force`.

### Phase 9: Rename Index/Length to VPIndex/VPLength to Harmonize with CIndex/CLength

- [x] Rename `Index` to `VPIndex` in `tui/src/core/coordinates/viewport_coords/index.rs`
      and update constructors `vp_idx` / `vp_index`.
- [x] Rename `Length` to `VPLength` in
      `tui/src/core/coordinates/viewport_coords/length.rs` and update constructor
      `vp_len`.
- [x] Update macro in
      `tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`.
- [x] Update re-exports in `tui/src/core/coordinates/viewport_coords/mod.rs` and
      `tui/src/core/coordinates/mod.rs`.
- [x] Update `canvas_coords.rs` conversion `From<VPIndex> for CIndex`.
- [x] Update bounds check traits in `tui/src/core/coordinates/bounds_check/`
      (`range_construct_ext.rs`, `range_convert_ext.rs`, `range_ext.rs`,
      `numeric_value.rs`).
- [x] Update call sites in `tui` (`params_ext.rs`, `vt_100_impl_scroll_ops.rs`,
      `color_wheel_impl.rs`).
- [x] Update test suites in `tui/src/core/coordinates/bounds_check/`
      (`array_bounds_check.rs`, `cursor_bounds_check.rs`, `index_ops.rs`, `length_ops.rs`,
      `range_bounds_check.rs`, `integration_tests.rs`).
- [x] Update `current_slide_index` in `tui/examples/tui_apps/ex_pitch/state.rs` and
      `tui/examples/tui_apps/ex_rc/state.rs` to use `CIndex`.
- [x] Update documentation and diagrams in `coordinates/` and
      `.agents/skills/check-bounds-safety/SKILL.md`.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --test`, and
      `./check.fish --quick-doc`.

### Phase 10: Color Wheel Type Modernization & Cast Cleanup

- [x] Update `ColorWheel` struct fields to `index: VPIndex` and `counter: VPLength` in
      `tui/src/core/color_wheel/color_wheel_impl.rs`.
- [x] Update `GradientLengthKind::ColorWheel` from `usize` to `VPLength` in
      `tui/src/core/color_wheel/color_wheel_config.rs` and update example call sites
      (`tui/examples/tui_apps/ex_app_no_layout/app_main.rs`).
- [x] Replace `unsafe transmute` in `Ansi256GradientIndex::from(u8)` with safe pattern
      match in `tui/src/core/color_wheel/gradients/ansi_256.rs`.
- [x] Simplify lolcat seed-to-index conversion in `convert_lolcat_seed_to_index` in
      `tui/src/core/color_wheel/color_wheel_impl.rs` with `// XMARK` annotation.
- [x] Update `update_index_with_direction` in
      `tui/src/core/color_wheel/color_wheel_impl.rs` to accept
      `gradient_length:     VPLength`.
- [x] Add comprehensive test coverage for `ColorWheel` coordinate bounds checking and
      seed-to-index conversion in `tui/src/core/color_wheel/color_wheel_impl.rs`.

### Phase 11: Eliminate Boolean Soup via SyntaxHighlightPipeline Enum & Skill Documentation

- [x] Create `SyntaxHighlightPipeline<'a>` enum in
      `tui/src/tui/editor/editor_buffer/buffer_struct.rs` with `R3BLMarkdown`,
      `Syntect(&'a str)`, and `PlainText` variants.
- [x] Add `EditorBuffer::get_syntax_highlight_pipeline()` method to replace boolean-blind
      `is_file_extension_default()` dispatch.
- [x] Refactor `engine_public_api.rs` (`render_content` & `syn_hi_syntect_path`) to match
      on `get_syntax_highlight_pipeline()` and pass `file_ext` payload directly.
- [x] Enrich `EditorBuffer` top-level struct documentation with
      `# Why EditorBuffer and EditorContent are split`.
- [x] Update `design-philosophy` skill (`.agents/skills/design-philosophy/SKILL.md` &
      `patterns.md`) with "Eliminate Boolean Soup / Boolean Blindness" guidelines and Bad
      vs. Good code examples.
- [x] Update unit tests in `buffer_struct.rs` (`test_file_extension_functions`) to test
      `SyntaxHighlightPipeline` variants.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --test`, and
      `./check.fish --quick-doc`.

### Phase 12: Mandatory Manual Review

- [x] **Mandatory manual review:** Verify every file modified across the entire task for
      correct implementation and ensure no regressions.
    - [x] `.agents/skills/check-bounds-safety/SKILL.md`
    - [x] `.agents/skills/check-bounds-safety/decision-trees.md`
    - [x] `.agents/skills/design-philosophy/SKILL.md`
    - [x] `.agents/skills/design-philosophy/patterns.md`
    - [x] `AGENTS.md`
    - [x] `build-infra/Cargo.toml`
    - [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_dictionary.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/technical_term_linker.rs`
    - [x] `cmdr/Cargo.toml`
    - [x] `cmdr/src/edi/app_main.rs`
    - [x] `cmdr/src/edi/state.rs`
    - [x] `tui/examples/tui_apps/ex_app_no_layout/app_main.rs`
    - [x] `tui/examples/tui_apps/ex_editor/app_main.rs`
    - [x] `tui/examples/tui_apps/ex_editor/state.rs`
    - [x] `tui/examples/tui_apps/ex_pitch/app_main.rs`
    - [x] `tui/examples/tui_apps/ex_pitch/state.rs`
    - [x] `tui/examples/tui_apps/ex_rc/app_main.rs`
    - [x] `tui/examples/tui_apps/ex_rc/state.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/active_buffer_routing.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/params_ext.rs`
    - [x] `tui/src/core/color_wheel/color_wheel_config.rs`
    - [x] `tui/src/core/color_wheel/color_wheel_impl.rs`
    - [x] `tui/src/core/color_wheel/gradients/ansi_256.rs`
    - [x] `tui/src/core/common/flat_2d_array/array_1d_simd_access.rs`
    - [x] `tui/src/core/common/ordered_map.rs`
    - [x] `tui/src/core/common/string_repeat_cache.rs`
    - [x] `tui/src/core/common/telemetry.rs`
    - [x] `tui/src/core/coordinates/bounds_check/array_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/index_ops.rs`
    - [x] `tui/src/core/coordinates/bounds_check/integration_tests.rs`
    - [x] `tui/src/core/coordinates/bounds_check/length_ops.rs`
    - [x] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [x] `tui/src/core/coordinates/bounds_check/numeric_value.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/result_enums.rs`
    - [x] `tui/src/core/coordinates/bounds_check/viewport_bounds_check.rs`
    - [x] `tui/src/core/coordinates/byte/byte_index.rs`
    - [x] `tui/src/core/coordinates/byte/byte_length.rs`
    - [x] `tui/src/core/coordinates/byte/byte_offset.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_coords.rs`
    - [x] `tui/src/core/coordinates/mod.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/col_index.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/index.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/length.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/mod.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/row_index.rs`
    - [x] `tui/src/core/graphemes/traits/seg_content.rs`
    - [x] `tui/src/core/graphemes/unicode_segment/seg.rs`
    - [x] `tui/src/core/graphemes/unicode_segment/seg_index.rs`
    - [x] `tui/src/core/graphemes/unicode_segment/segment_builder.rs`
    - [x] `tui/src/core/log/log_public_api.rs`
    - [x] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [x] `tui/src/core/pty/pty_session/pty_session_builder.rs`
    - [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
    - [x] `tui/src/core/tui_style/tui_style_attribs.rs`
    - [x] `tui/src/readline_async/readline_async_impl/readline_history.rs`
    - [x] `tui/src/tui/dialog/dialog_buffer/dialog_buffer_struct.rs`
    - [x] `tui/src/tui/dialog/test_dialog.rs`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_config_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/caret_locate.rs`
    - [x] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_support.rs`
    - [x] `tui/src/tui/editor/editor_buffer/history/editor_history.rs`
    - [x] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection/line_selection.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection/multiline_selection.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection/selection_range.rs`
    - [x] `tui/src/tui/editor/editor_buffer/sizing.rs`
    - [x] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [x] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [x] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_internal_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`
    - [x] `tui/src/tui/editor/test_fixtures_editor.rs`
    - [x] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/char_ops.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/line_level_ops.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/types.rs`
    - [x] `tui/src/tui/terminal_window/manage_focus/component_registry.rs`

### Phase 13: Standardize Getter Names Across `EditorBuffer` & `ZeroCopyGapBuffer`

- [x] In `tui/src/tui/editor/editor_buffer/buffer_struct.rs`:
    - [x] Rename `pub fn line_at_row_index` to `pub fn get_line_at_row_index`.
    - [x] Rename `pub fn c_height` to `pub fn get_c_height`.
- [x] In `tui/src/tui/editor/editor_buffer/buffer_struct.rs`:
    - [x] Rename `line_at_caret_is_empty` -> `is_line_at_caret_empty`.
    - [x] Rename `line_at_c_caret` -> `get_line_at_c_caret`.
    - [x] Rename `seg_at_end_of_line_at_c_caret` -> `get_seg_at_end_of_line_at_c_caret`.
    - [x] Rename `seg_to_right_of_caret` -> `get_seg_to_right_of_caret`.
    - [x] Rename `seg_to_left_of_caret` -> `get_seg_to_left_of_caret`.
    - [x] Rename `prev_line_above_caret` -> `get_prev_line_above_caret`.
    - [x] Rename `seg_and_line_at_caret` -> `get_seg_and_line_at_caret`.
    - [x] Rename `str_at_caret` -> `get_str_at_caret`.
    - [x] Rename `seg_at_caret` -> `get_seg_at_caret`.
    - [x] Rename `next_line_below_caret_to_string` -> `get_next_line_below_caret`.
- [x] In `tui/src/tui/editor/zero_copy_gap_buffer/`:
    - [x] Rename `c_len` -> `get_c_len` in `zcgb_basic_ops.rs`.
    - [x] Rename `line_count` -> `get_line_count` in `zcgb_core.rs`.
    - [x] Rename `line_is_empty` -> `is_line_empty` in `zcgb_basic_ops.rs`.
    - [x] Rename `check_is_in_middle_of_grapheme` -> `is_in_middle_of_grapheme` in
          `zcgb_basic_ops.rs`.
- [x] Update call sites across workspace.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --test`, and
      `./check.fish --quick-doc`.

### Phase 14: ZeroCopyGapBuffer Mutator Return Type Refactoring & Constructor Conventions

- [x] Refactor `ZeroCopyGapBuffer::remove_line` to return `Option<LineMetadata>`.
- [x] Remove dead code `can_insert()` and its associated unit test and benchmark.
- [x] Derive `Default` for `ZeroCopyGapBuffer`, remove redundant no-arg `pub fn new()`,
      and replace calls across the workspace with `ZeroCopyGapBuffer::default()`.
- [x] Refactor `insert_line` to return `Option<CRow>`.
- [x] Refactor `set_line` to return `Option<()>`.
- [x] Refactor `delete_at_col` to return `Option<()>`.
- [x] Refactor `merge_with_next_line` to return `Option<LineMetadata>`.
- [x] Flatten control flow in `merge_with_next_line`, `set_line`, and `delete_at_col`
      using `?` and `let Ok(()) = ... else { return None; }` to eliminate `.ok()?` method
      chaining.
- [x] Update `AGENTS.md` and `.agents/skills/design-philosophy/SKILL.md` with guidelines
      for Constructor Conventions (`Default` over No-Arg `new()`) and avoiding boolean
      blindness on mutator returns (`Option<T>` / `Option<()>`).
- [x] Update all affected unit tests and doc comments across the workspace.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --quick-doc`, and
      `./check.fish --test`.
- [x] Mandatory manual review.

### Phase 16: Comprehensive Rustdoc Documentation Coverage in `editor_buffer` Module

- [x] Add rustdoc comments to all `EditorContent` methods (`get_lines`, `get_c_caret`,
      `get_viewport`, `get_maybe_file_extension`, `get_maybe_file_path`, `get_selection`).
- [x] Add rustdoc comments to `EditorBuffer` history methods (`add`, `undo`, `redo`).
- [x] Add rustdoc comments to `EditorBuffer` content display width methods
      (`get_max_row_index`, `get_line_display_width_at_c_caret`).
- [x] Add rustdoc comments to `EditorBuffer` content near caret methods
      (`is_line_at_caret_empty`, `get_line_at_c_caret`,
      `get_seg_at_end_of_line_at_c_caret`, `get_seg_to_right_of_caret`,
      `get_seg_to_left_of_caret`, `get_prev_line_above_caret`, `get_str_at_caret`,
      `get_seg_at_caret`, `get_next_line_below_caret`).
- [x] Add rustdoc comments to `EditorBuffer` access & mutation methods
      (`get_syntax_highlight_pipeline`, `get_maybe_file_extension`, `is_empty`,
      `get_line_at_row_index`, `get_c_height`, `get_lines`,
      `get_as_string_with_comma_instead_of_newlines`, `get_as_string_with_newlines`,
      `get_c_caret`, `get_vp_origin`, `has_selection`, `get_file_path`, `set_file_path`,
      `set_file_extension`, and struct field accessors).
- [x] Add rustdoc comments to helper methods in `caret_locate.rs`, `render_cache.rs`, and
      `sizing.rs`.
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --quick-doc`, and
      `./check.fish --test`.

### Phase 17: Mandatory Manual Review

- [x] **Mandatory manual review:** Verify modified files.
    - [x] `task/buffer-struct-modernize.md`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/caret_locate.rs`
    - [x] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [x] `tui/src/tui/editor/editor_buffer/sizing.rs`

### Phase 18: Remove Redundant `impl EditorContent` & Clean Up `EditorBuffer` Accessors

- [x] Remove the redundant `impl EditorContent` block from `buffer_struct.rs`.
- [x] Update `sizing.rs` to access `EditorContent` fields directly (`self.lines`,
      `self.selection`).
- [x] Simplify `EditorBuffer::get_maybe_file_extension` using
      `self.content.maybe_file_extension.as_deref()`.
- [x] Simplify `EditorBuffer::get_file_path` using
      `self.content.maybe_file_path.as_ref()`.
- [x] Clean up remaining `EditorBuffer` accessors (`get_c_caret`, `get_lines`,
      `has_selection`).
- [x] Run `./check.fish --check`, `./check.fish --clippy`, `./check.fish --quick-doc`, and
      `./check.fish --test`.

### Phase 19: Mandatory Manual Review

- [x] **Mandatory manual review:** Verify modified files.
    - [x] `task/buffer-struct-modernize.md`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/sizing.rs`

# Test Migration Plan - Moving Tests from test_editor.rs

**Last Updated**: 2025-07-20

## Current State Summary

This document tracks the migration of tests from `test_editor.rs` to their respective modules, and identifies untested public functions. The tests can continue to use `editor_test_fixtures.rs` for shared test utilities.

### ✅ Modules with Good Test Coverage
- `editor_buffer/selection_range.rs` - Has tests for `test_locate()` and `test_tuple()`
- `editor_buffer/selection_list.rs` - Has `test_selection_map_direction_change()`
- `editor_buffer/render_cache.rs` - Has comprehensive tests
- `editor_buffer/history.rs` - Has extensive test coverage
- `editor_buffer/cur_index.rs` - Has 4 tests
- `editor_buffer/caret_locate.rs` - Has 2 tests
- `editor_engine/validate_scroll_on_resize.rs` - Has tests module with 3 tests

### ❌ Modules That Still Need More Test Coverage
After the migration, most modules now have tests, but some areas still need attention for Phase 3.

## ✅ Phase 2 Complete: All Tests Successfully Migrated from test_editor.rs

**Migration completed on**: 2025-07-20

All tests have been successfully migrated from the centralized `test_editor.rs` file to their respective modules following idiomatic Rust patterns. Here's the final migration summary:

### ✅ Tests in `mod test_config_options` (2 tests) → `editor_component/editor_event.rs`
- `test_multiline_true()` - Tests multiline editor configuration
- `test_multiline_false()` - Tests single-line editor configuration

### ✅ Tests in `mod test_editor_ops` (15 tests) → Various modules
**Content mutation tests → `editor_engine/content_mut.rs`:**
- `editor_delete()`
- `editor_backspace()`
- `editor_insert_new_line()`
- `editor_insertion()`

**Navigation tests → `editor_engine/caret_mut.rs`:**
- `editor_validate_caret_pos_on_up()`
- `editor_validate_caret_pos_on_down()`
- `editor_move_caret_up_down()`
- `editor_move_caret_left_right()`
- `editor_move_caret_home_end()`
- `editor_move_caret_home_end_overflow_viewport()`
- `editor_move_caret_page_up_page_down()`

**Scroll tests → `editor_engine/scroll_editor_content.rs`:**
- `editor_scroll_vertical()`
- `editor_scroll_horizontal()`
- `editor_scroll_right_horizontal_long_line_with_jumbo_emoji()`

**Buffer tests → `editor_buffer/buffer_struct.rs`:**
- `editor_empty_state()`

### ✅ Tests in `mod selection_tests` (1 test) → `editor_component/editor_event.rs`
- `test_text_selection()` - Tests comprehensive selection behavior through EditorEvents

### ✅ Tests in `mod clipboard_tests` (3 tests) → `editor_buffer/clipboard_support.rs`
- `test_copy()`
- `test_paste()`
- `test_cut()`

### ✅ Tests in `mod test_batch_operations` (6 tests) → `editor_engine/content_mut.rs`
- `test_insert_lines_batch_at_caret_basic()`
- `test_insert_lines_batch_with_empty_lines()`
- `test_insert_lines_batch_at_middle_of_line()`
- `test_batch_vs_individual_insert_result_equivalence()`
- `test_insert_lines_batch_empty_vector()`
- `test_insert_lines_batch_large_content()`

### ✅ Tests in `mod test_engine_internal_api` (10 tests) → `editor_engine/engine_internal_api.rs`
- `test_select_all()`
- `test_clear_selection()`
- `test_delete_selected()`
- `test_copy_editor_selection_to_clipboard()`
- `test_delete_selected_with_partial_selection()`
- `test_line_at_caret_to_string()`
- `test_navigation_with_selection()`
- `test_page_navigation()`
- `test_home_end_navigation()`

## Test Coverage Status After Migration

The migration has significantly improved test coverage. Here's the updated status:

### 🟢 Modules with Good Test Coverage (After Migration)
- ✅ `editor_component/editor_event.rs` - Now has config and selection tests
- ✅ `editor_engine/content_mut.rs` - Now has comprehensive content mutation and batch operation tests
- ✅ `editor_engine/caret_mut.rs` - Now has all navigation tests
- ✅ `editor_engine/engine_internal_api.rs` - Now has internal API tests
- ✅ `editor_engine/scroll_editor_content.rs` - Now has scroll behavior tests
- ✅ `editor_buffer/clipboard_support.rs` - Now has clipboard operation tests
- ✅ `editor_buffer/buffer_struct.rs` - Already had tests, added empty state test
- ✅ `editor_buffer/selection_range.rs` - Already has comprehensive tests
- ✅ `editor_buffer/selection_list.rs` - Already has comprehensive tests
- ✅ `editor_buffer/render_cache.rs` - Already has comprehensive tests
- ✅ `editor_buffer/history.rs` - Already has extensive test coverage
- ✅ `editor_buffer/cur_index.rs` - Already has 4 tests
- ✅ `editor_buffer/caret_locate.rs` - Already has 2 tests
- ✅ `editor_engine/validate_scroll_on_resize.rs` - Already has tests module with 3 tests

## Note on Missing Tests from Original Plan

The original migration plan referenced many tests that don't actually exist in `test_editor.rs`. The tests listed above are the actual tests that need to be migrated.

## Notes on Test Migration

1. **Preserve `editor_test_fixtures.rs`**: All moved tests should continue to use the shared test fixtures for consistency
2. **Update Import Paths**: When moving tests, update the `use` statements to import from the correct modules
3. **Keep Integration Tests**: Some high-level integration tests in `test_editor.rs` should remain for end-to-end testing
4. **Test Organization**: Each target file should have a `#[cfg(test)]` mod containing the relevant unit tests
5. **Incremental Migration**: Tests can be moved incrementally, ensuring each module's tests pass after migration

## Benefits of This Migration

1. **Co-location**: Tests will be next to the code they test, making maintenance easier
2. **Faster Feedback**: Developers working on a module can run just that module's tests
3. **Better Organization**: Each module becomes self-contained with its own test suite
4. **Reduced Test File Size**: The large `test_editor.rs` file becomes more manageable

# Untested Public Functions in Editor Module

**Last Updated**: 2025-07-20

## Summary
Based on the analysis of the editor module, here are all public functions that currently lack test coverage. This document tracks both untested functions and the migration status of tests from `test_editor.rs` to their respective modules.

## editor_buffer module

### selection_support.rs
- `pub fn dummy_viewport() -> Size`
- `pub fn handle_selection_single_line_caret_movement(...)`
- `pub fn handle_selection_multiline_caret_movement(...)`
- `pub fn handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(...)`

### clipboard_support.rs (untested)
- `pub fn copy_to_clipboard(...)`
- `fn try_to_put_content_into_clipboard(...)` (ClipboardService trait method)
- `fn try_to_get_content_from_clipboard(...)` (ClipboardService trait method)

### selection_support.rs helper modules (all public functions untested)
Note: These functions are in helper modules within selection_support.rs:

**single_line_select_helper module:**
- `pub fn create_new_range(...)`
- `pub fn log_range_debug_info(...)`
- `pub fn handle_left_shrink_end(...)`
- `pub fn handle_left_grow_start(...)`
- `pub fn handle_right_grow_end(...)`
- `pub fn handle_right_shrink_start(...)`
- `pub fn remove_empty_range(...)`

**multiline_select_helper module:**
- `pub fn handle_two_lines(...)`

**handle_two_lines_helper module:**
- `pub fn validate_preconditions(...)`
- `pub fn setup_and_log_debug(...)`

**start_selection_helper module:**
- `pub fn start_select_down(...)`
- `pub fn start_select_up(...)`

**continue_selection_helper module:**
- `pub fn continue_select_down(...)`
- `pub fn continue_select_up(...)`

**direction_change_helper module:**
- `pub fn continue_direction_change_select_up(...)`
- `pub fn continue_direction_change_select_down(...)`

### clipboard_support.rs
- `pub fn copy_to_clipboard(...)`

### selection_range.rs (mostly tested)
The following functions are already tested:
- `pub fn start()` ✓ TESTED
- `pub fn end()` ✓ TESTED
- `pub fn get_start_display_col_index_as_width()` ✓ TESTED
- `pub fn clip_to_range()` ✓ TESTED
- `pub fn locate_scroll_offset_col()` ✓ TESTED
- `pub fn caret_movement_direction()` ✓ TESTED
- `pub fn caret_movement_direction_up_down()` ✓ TESTED
- `pub fn caret_movement_direction_left_right()` ✓ TESTED
- `pub fn locate_column()` ✓ TESTED
- `pub fn new()` ✓ TESTED
- `pub fn grow_end_by()` ✓ TESTED
- `pub fn shrink_end_by()` ✓ TESTED
- `pub fn grow_start_by()` ✓ TESTED
- `pub fn shrink_start_by()` ✓ TESTED
- `pub fn as_tuple()` ✓ TESTED

All major functions in selection_range.rs are covered by tests!

### caret_locate.rs (partially tested)
The following functions are tested:
- `pub fn col_index_for_width()` ✓ TESTED
- `pub fn row_index_for_height()` ✓ TESTED

Untested functions:
- `pub fn locate_col(editor_buffer: &EditorBuffer) -> CaretColLocationInLine`
- `pub fn locate_row(buffer: &EditorBuffer) -> CaretRowLocationInBuffer`

### selection_list.rs (fully tested)
All major functions are tested ✓:
- `pub fn get_caret_at_start_of_range_scroll_adjusted()` ✓ TESTED
- `pub fn get_selected_lines()` ✓ TESTED
- `pub fn get_ordered_indices()` ✓ TESTED
- `pub fn get_ordered_list()` ✓ TESTED
- `pub fn is_empty()` ✓ TESTED
- `pub fn clear()` ✓ TESTED
- `pub fn len()` ✓ TESTED
- `pub fn iter()` ✓ TESTED
- `pub fn get()` ✓ TESTED
- `pub fn insert()` ✓ TESTED
- `pub fn remove()` ✓ TESTED
- `pub fn update_previous_direction()` ✓ TESTED
- `pub fn remove_previous_direction()` ✓ TESTED
- `pub fn locate_row()` ✓ TESTED
- Direction change related functions ✓ TESTED

All functions in selection_list.rs have good test coverage!

### buffer_struct.rs (partially tested)
The following functions are already tested:
- `pub fn new_with_one_empty_line()` ✓ TESTED (Note: replaced the old `new_empty()` function)
- `pub fn set_lines()` ✓ TESTED
- `pub fn get_lines()` ✓ TESTED
- `pub fn get_max_row_index()` ✓ TESTED
- `pub fn line_at_row_index()` ✓ TESTED
- `pub fn get_as_string_with_newlines()` ✓ TESTED
- `pub fn get_as_string_with_separator()` ✓ TESTED
- `pub fn get_as_string_with_comma_instead_of_newlines()` ✓ TESTED
- `pub fn line_at_caret_is_empty()` ✓ TESTED
- `pub fn get_caret_raw()` ✓ TESTED
- `pub fn get_caret_scr_adj()` ✓ TESTED
- `pub fn get_scr_ofs()` ✓ TESTED
- `pub fn get_line_display_width_at_caret_scr_adj()` ✓ TESTED
- `pub fn get_line_display_width_at_row_index()` ✓ TESTED
- `pub fn line_at_caret_scr_adj()` ✓ TESTED
- `pub fn has_selection()` ✓ TESTED
- `pub fn string_at_end_of_line_at_caret_scr_adj()` ✓ TESTED
- `pub fn string_to_right_of_caret()` ✓ TESTED
- `pub fn string_to_left_of_caret()` ✓ TESTED
- `pub fn string_at_caret()` ✓ TESTED
- `pub fn prev_line_above_caret()` ✓ TESTED
- `pub fn next_line_below_caret_to_string()` ✓ TESTED
- `pub fn impl_get_line_display_width_at_caret_scr_adj()` ✓ TESTED
- `pub fn impl_get_line_display_width_at_row_index()` ✓ TESTED

Untested functions:
- `pub fn add()` (history related)
- `pub fn undo()` (history related)
- `pub fn redo()` (history related)
- `pub fn is_file_extension_default()`
- `pub fn has_file_extension()`
- `pub fn get_maybe_file_extension()`
- `pub fn is_empty()`
- `pub fn len()`
- `pub fn get_mut()`
- `pub fn get_mut_no_drop()`
- `pub fn clear_selection()`
- `pub fn get_selection_list()`
- `pub fn invalidate_memory_size_calc_cache()`
- `pub fn upsert_memory_size_calc_cache()`
- `pub fn get_memory_size_calc_cached()`

### render_cache.rs (fully tested)
All functions are tested ✓:
- `pub fn new()` (Key constructor) ✓ TESTED
- `pub fn new()` (RenderCacheEntry constructor) ✓ TESTED
- `pub fn clear()` ✓ TESTED
- `pub fn get()` ✓ TESTED
- `pub fn insert()` ✓ TESTED
- `pub fn render_content()` ✓ TESTED

All functions in render_cache.rs have comprehensive test coverage!

## editor_engine module

### content_mut.rs (well tested)
The following functions are tested ✓:
- `pub fn insert_chunk_at_caret()` ✓ TESTED
- `pub fn insert_new_line_at_caret()` ✓ TESTED
- `pub fn delete_at_caret()` ✓ TESTED
- `pub fn backspace_at_caret()` ✓ TESTED
- `pub fn delete_selected()` ✓ TESTED
- `pub fn insert_lines_batch_at_caret()` ✓ TESTED

Untested functions:
- Helper functions in ContentMutatorImpl* structs (these are internal implementation details)

### engine_struct.rs (partially tested)
Untested functions:
- `pub fn new(config_options: EditorEngineConfig) -> Self`
- `pub fn set_ast_cache(&mut self, ast_cache: StyleUSSpanLines)`
- `pub fn clear_ast_cache(&mut self)`

### engine_public_api.rs (partially tested)
Untested functions:
- `pub fn apply_event(...)`
- `pub fn render_engine(...)`
- `pub fn render_content(...)`
- `pub fn render_selection(...)`
- `pub fn render_caret(...)`
- `pub fn render_empty_state(...)`

### validate_scroll_on_resize.rs (untested) - NEW FILE
- `pub fn validate_scroll_on_resize(args: EditorArgsMut<'_>)` - Handles scroll validation during terminal resize

### validate_buffer_mut.rs (untested)
All public functions need testing:
- `pub fn get_line_display_width_at_caret_scr_adj_row_index(&self) -> ColWidth`
- `pub fn new<'a>(...) for OnCaretMutator`
- `pub fn new<'a>(...) for OnScrollMutator`
- `pub fn new<'a>(...) for OnContentMutator`
- `pub fn perform_validation_checks_after_mutation(...)`
- `pub fn is_scroll_offset_in_middle_of_grapheme_cluster(...)`
- `pub fn adjust_scroll_offset_because_in_middle_of_grapheme_cluster(...)`
- `pub fn adjust_caret_col_if_not_in_middle_of_grapheme_cluster(...)`

### scroll_editor_content.rs (untested)
All public functions need testing:
- `pub fn inc_caret_col_by(...)`
- `pub fn clip_caret_to_content_width(...)`
- `pub fn set_caret_col_to(...)`
- `pub fn dec_caret_col_by(...)`
- `pub fn reset_caret_col(...)`
- `pub fn dec_caret_row(...)`
- `pub fn change_caret_row_by(...)`
- `pub fn clip_caret_row_to_content_height(...)`
- `pub fn inc_caret_row(...)`

### caret_mut.rs (mostly untested)
All movement functions need testing:
- `pub fn up(...)`
- `pub fn page_up(...)`
- `pub fn down(...)`
- `pub fn page_down(...)`
- `pub fn to_start_of_line(...)`
- `pub fn to_end_of_line(...)`
- `pub fn right(...)`
- `pub fn left(...)`
- Helper functions in CaretMutatorImpl* structs

### select_mode.rs (untested)
All public functions need testing:
- `pub fn get_caret_scr_adj(...)`
- `pub fn handle_selection_single_line_caret_movement(...)`
- `pub fn update_selection_based_on_caret_movement_in_multiple_lines(...)`

### engine_internal_api.rs (partially tested)
Most functions are tested through integration tests, but may need dedicated unit tests:
- `pub fn up(...)` - tested in integration
- `pub fn left(...)` - tested in integration
- `pub fn right(...)` - tested in integration
- `pub fn down(...)` - tested in integration
- `pub fn page_up(...)` - tested in integration
- `pub fn page_down(...)` - tested in integration
- `pub fn home(...)` - tested in integration
- `pub fn end(...)` - tested in integration
- `pub fn select_all(...)` - tested in integration
- `pub fn clear_selection(...)` - tested in integration
- `pub fn line_at_caret_to_string(...)` - tested in integration
- `pub fn insert_str_at_caret(...)` - tested in integration
- `pub fn insert_str_batch_at_caret(...)` - tested in integration
- `pub fn insert_new_line_at_caret(...)` - tested in integration
- `pub fn delete_at_caret(...)` - tested in integration
- `pub fn delete_selected(...)` - tested in integration
- `pub fn backspace_at_caret(...)` - tested in integration
- `pub fn copy_editor_selection_to_clipboard(...)` - tested in integration

## editor_component module

### editor_component_struct.rs (untested)
- `pub fn new(...)`
- `pub fn new_boxed(...)`

### editor_event.rs (partially tested)
These functions are tested through integration tests but may benefit from dedicated unit tests:
- `pub fn apply_editor_event(...)` - tested in integration
- `pub fn apply_editor_events<S, AS>(...)` - tested in integration

## Recently Added Functions Requiring Tests

These functions were added recently and need test coverage:

1. **validate_scroll_on_resize.rs** (NEW):
   - `validate_scroll_on_resize()` - Critical for handling terminal resize events properly

2. **ClipboardService trait methods**:
   - `try_to_put_content_into_clipboard()` - Essential for clipboard operations
   - `try_to_get_content_from_clipboard()` - Essential for clipboard operations

## Priority Functions to Test

Based on current test coverage analysis, here are the top priority functions that should be tested first:

### **HIGH PRIORITY - Core Untested Functions**

1. **Buffer Operations (Untested)**:
   - `EditorBuffer::is_empty()`
   - `EditorBuffer::len()`
   - `EditorBuffer::get_mut()` and `EditorBuffer::get_mut_no_drop()`
   - File extension related functions: `is_file_extension_default()`, `has_file_extension()`, `get_maybe_file_extension()`
   - Memory cache functions: `invalidate_memory_size_calc_cache()`, `upsert_memory_size_calc_cache()`, `get_memory_size_calc_cached()`

2. **Engine Core Functions (Untested)**:
   - `EditorEngine::new()`
   - `apply_event()`
   - Rendering pipeline: `render_engine()`, `render_content()`, `render_selection()`, `render_caret()`

3. **Caret Movement (Untested)**:
   - All directional movement functions in `caret_mut.rs`
   - Page up/down operations
   - Home/End operations
   - Scroll content functions in `scroll_editor_content.rs`

4. **Selection Support (Untested)**:
   - All functions in selection_support.rs helper modules
   - `dummy_viewport()`
   - Multiline selection handling functions

5. **Editor Component (Untested)**:
   - `EditorComponent::new()` and `EditorComponent::new_boxed()`

### **MEDIUM PRIORITY - Validation & Support Functions**

6. **Buffer Validation (Untested)**:
   - All functions in `validate_buffer_mut.rs`
   - Grapheme cluster validation functions

7. **Selection Mode (Untested)**:
   - All functions in `select_mode.rs`

8. **Caret Location (Partially Tested)**:
   - `locate_col()` and `locate_row()` functions

### **LOW PRIORITY - Well Tested Areas**

The following areas have good test coverage and are lower priority:
- ✅ `selection_range.rs` - Fully tested
- ✅ `selection_list.rs` - Fully tested
- ✅ `render_cache.rs` - Fully tested
- ✅ `content_mut.rs` - Well tested (main functions)
- ✅ Most of `buffer_struct.rs` - Well tested
- ✅ Integration tests cover many `engine_internal_api.rs` functions

## Test Coverage Recommendations

1. **Focus on Unit Tests**: Create dedicated unit tests for the HIGH PRIORITY functions listed above
2. **Buffer Operations**: Test edge cases like empty buffers, single character operations, and boundary conditions
3. **Caret Movement**: Test all directional movements, especially edge cases at document boundaries
4. **Selection Operations**: Test complex multiline selection scenarios
5. **Rendering Pipeline**: Test the rendering functions with various editor states
6. **Unicode/Emoji Handling**: Ensure all functions handle Unicode correctly (some already tested in content_mut.rs)
7. **Error Conditions**: Test validation functions with invalid inputs
8. **Memory Management**: Test cache invalidation and memory size calculations


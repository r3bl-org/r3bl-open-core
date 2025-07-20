/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! Functions that implement the internal (not re-exported in `mod.rs`) & functional API
//! of the editor engine. See [`mod@super::engine_public_api`] for the public event based
//! API.

use super::{DeleteSelectionWith, SelectMode, caret_mut, content_mut};
use crate::{EditorArgsMut, EditorBuffer, EditorEngine, GCString, clipboard_support,
            clipboard_support::ClipboardService};

pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::up(buffer, engine, sel_mod);
}

pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::left(buffer, engine, sel_mod);
}

pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::right(buffer, engine, sel_mod);
}

pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::down(buffer, engine, sel_mod);
}

pub fn page_up(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    caret_mut::page_up(buffer, engine, sel_mod);
}

pub fn page_down(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    caret_mut::page_down(buffer, engine, sel_mod);
}

pub fn home(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::to_start_of_line(buffer, engine, sel_mod);
}

pub fn end(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::to_end_of_line(buffer, engine, sel_mod);
}

pub fn select_all(buffer: &mut EditorBuffer, sel_mod: SelectMode) {
    caret_mut::select_all(buffer, sel_mod);
}

pub fn clear_selection(buffer: &mut EditorBuffer) { buffer.clear_selection(); }

#[must_use]
pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&GCString> {
    buffer.line_at_caret_scr_adj()
}

pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    content_mut::insert_chunk_at_caret(args, chunk);
}

/// Inserts multiple lines of text at the caret position in a single batch operation.
///
/// # Performance Benefits
/// This function is significantly more efficient than inserting lines individually because:
/// 
/// 1. **Single validation pass**: The editor buffer validation (scroll position, caret bounds,
///    selection ranges, etc.) only happens once when the batch operation completes, rather 
///    than after each line insertion.
///
/// 2. **Atomic operation**: All lines are inserted within a single `EditorBufferMutWithDrop`
///    scope, which defers validation until the scope ends.
///
/// 3. **Reduced overhead**: For N lines, this reduces the operation count from 2N-1 
///    (N line insertions + N-1 newline insertions, each with validation) to just 1 
///    batch operation with a single validation.
///
/// # Example
/// ```ignore
/// // Slow approach - validates after each operation:
/// for line in lines {
///     insert_str_at_caret(args, line);      // Validates
///     insert_new_line_at_caret(args);       // Validates again
/// }
///
/// // Fast approach - validates once at the end:
/// insert_str_batch_at_caret(args, lines);   // Validates once
/// ```
///
/// # Arguments
/// * `args` - Mutable references to the editor engine and buffer
/// * `lines` - Vector of string slices to insert, with newlines automatically added between them
pub fn insert_str_batch_at_caret(args: EditorArgsMut<'_>, lines: Vec<&str>) {
    content_mut::insert_lines_batch_at_caret(args, lines);
}

pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
    content_mut::insert_new_line_at_caret(args);
}

pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) {
    content_mut::delete_at_caret(buffer, engine);
}

pub fn delete_selected(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    with: DeleteSelectionWith,
) {
    content_mut::delete_selected(buffer, engine, with);
}

pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) {
    content_mut::backspace_at_caret(buffer, engine);
}

pub fn copy_editor_selection_to_clipboard(
    buffer: &EditorBuffer,
    clipboard: &mut impl ClipboardService,
) {
    clipboard_support::copy_to_clipboard(buffer, clipboard);
}

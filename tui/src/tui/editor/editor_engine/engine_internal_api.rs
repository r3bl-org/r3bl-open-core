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
//! of the editor engine. See [mod@super::engine_public_api] for the public event based
//! API.

use super::{caret_mut, content_mut, DeleteSelectionWith, SelectMode};
use crate::{clipboard_support,
            clipboard_support::ClipboardService,
            EditorArgsMut,
            EditorBuffer,
            EditorEngine,
            GCString};

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

pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&GCString> {
    buffer.line_at_caret_scr_adj()
}

pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    content_mut::insert_chunk_at_caret(args, chunk);
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

pub fn paste_clipboard_content_into_editor(
    args: EditorArgsMut<'_>,
    clipboard: &mut impl ClipboardService,
) {
    clipboard_support::paste_from_clipboard(args, clipboard);
}

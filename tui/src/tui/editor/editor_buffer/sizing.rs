/*
 *   Copyright (c) 2025 R3BL LLC
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

use r3bl_core::{GCString, RingBufferHeap};
use smallvec::SmallVec;

use super::{history::EditorHistory, EditorContent};

pub type VecEditorContentLines = SmallVec<[GCString; DEFAULT_EDITOR_LINES_SIZE]>;
const DEFAULT_EDITOR_LINES_SIZE: usize = 32;

/// The version history is stored on the heap, as a ring buffer.
pub type HistoryBuffer = RingBufferHeap<EditorContent, MAX_UNDO_REDO_SIZE>;
/// This is the absolute maximum number of undo/redo steps that will ever be stored.
pub const MAX_UNDO_REDO_SIZE: usize = 16;

impl size_of::SizeOf for EditorContent {
    fn size_of_children(&self, context: &mut size_of::Context) {
        context.add(size_of_val(&self.lines)); /* use for fields that can expand or contract */
        context.add(size_of_val(&self.maybe_file_extension)); /* use for fields that can expand or contract */
        context.add(size_of_val(&self.maybe_file_path)); /* use for fields that can expand or contract */
        context.add(self.caret_raw.size_of().total_bytes());
        context.add(self.scr_ofs.size_of().total_bytes());
        context.add(self.sel_list.size_of().total_bytes());
    }
}

impl size_of::SizeOf for EditorHistory {
    fn size_of_children(&self, context: &mut size_of::Context) {
        context.add(size_of_val(&self.versions)); /* use for fields that can expand or contract */
        context.add(self.current_index.size_of().total_bytes());
    }
}

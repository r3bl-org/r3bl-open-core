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

use smallvec::SmallVec;

use super::{cur_index::CurIndex, history::EditorHistory, EditorContent};
use crate::{get_mem_size,
            CaretRaw,
            GCString,
            GetMemSize,
            InlineString,
            RingBufferHeap,
            ScrOfs,
            TinyInlineString};

pub type VecEditorContentLines = SmallVec<[GCString; DEFAULT_EDITOR_LINES_SIZE]>;
const DEFAULT_EDITOR_LINES_SIZE: usize = 32;

/// The version history is stored on the heap, as a ring buffer.
pub type HistoryBuffer = RingBufferHeap<EditorContent, MAX_UNDO_REDO_SIZE>;

/// This is the absolute maximum number of undo/redo steps that will ever be stored.
pub const MAX_UNDO_REDO_SIZE: usize = 16;

impl GetMemSize for EditorContent {
    fn get_mem_size(&self) -> usize {
        get_mem_size::slice_size(&self.lines)
            + std::mem::size_of::<CaretRaw>()
            + std::mem::size_of::<ScrOfs>()
            + std::mem::size_of::<Option<TinyInlineString>>()
            + std::mem::size_of::<Option<InlineString>>()
            + self.sel_list.get_mem_size()
    }
}

impl GetMemSize for EditorHistory {
    fn get_mem_size(&self) -> usize {
        let versions_size = get_mem_size::ring_buffer_size(&self.versions);
        let cur_index_field_size = std::mem::size_of::<CurIndex>();
        versions_size + cur_index_field_size
    }
}

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

use std::fmt::{Debug, Formatter, Result};

use r3bl_core::{call_if_true,
                ch,
                format_as_kilobytes_with_commas,
                i16,
                RingBuffer as _};
use size_of::SizeOf as _;

use super::{buffer_struct::{cache, sizing},
            EditorBuffer,
            EditorContent};
use crate::DEBUG_TUI_COPY_PASTE;

pub const MIN_INDEX: CurIndexNumber = -1;

/// # Undo/Redo Algorithm
///
/// The `EditorHistory` struct manages the undo/redo functionality for the `EditorBuffer`.
/// It uses a ring buffer (`versions`) to store the different states of the
/// `EditorContent`. The `current_index` field points to the current state in the
/// `versions` buffer.
///
/// ## Pushing a new state (history::push)
///
/// 1. The current render cache is cleared to invalidate any cached rendering.
/// 2. A copy of the current `EditorContent` is created.
/// 3. If the `current_index` is not the last index in the `versions` buffer, the history
///    from `current_index + 1` to the end of the buffer is truncated (removed). This
///    discards any "future" states that were previously redone.
/// 4. The copy of the `EditorContent` is added to the `versions` buffer.
/// 5. The `current_index` is incremented to point to the newly added state.
///
/// ## Undoing (history::undo)
///
/// 1. The current render cache is cleared to invalidate any cached rendering.
/// 2. The current caret position is retained.
/// 3. If there is a previous state in the `versions` buffer (i.e., `current_index > 0`),
///    the `current_index` is decremented.
/// 4. The `EditorContent` at the new `current_index` is retrieved from the `versions`
///    buffer and set as the current content of the `EditorBuffer`.
/// 5. The caret position is restored.
///
/// ## Redoing (history::redo)
///
/// 1. The current render cache is cleared to invalidate any cached rendering.
/// 2. If there is a next state in the `versions` buffer (i.e., `current_index <
///    versions.len() - 1`), the `current_index` is incremented.
/// 3. The `EditorContent` at the new `current_index` is retrieved from the `versions`
///    buffer and set as the current content of the `EditorBuffer`.
///
/// ## Notes
///
/// - The `versions` buffer has a maximum size (`MAX_UNDO_REDO_SIZE`). When the buffer is
///   full, adding a new state will overwrite the oldest state in the buffer.
/// - The `current_index` can be -1 if the buffer is empty.
/// - The caret position is retained during undo operations.
#[derive(Clone, PartialEq)]
pub struct EditorHistory {
    pub versions: super::buffer_struct::sizing::VersionHistory,
    pub current_index: CurIndex,
}

type CurIndexNumber = i16;

#[derive(Clone, Copy, PartialEq, size_of::SizeOf, Debug)]
pub struct CurIndex(pub CurIndexNumber);

impl Default for CurIndex {
    fn default() -> Self { Self(MIN_INDEX) }
}

impl From<usize> for CurIndex {
    fn from(val: usize) -> Self { Self(val as CurIndexNumber) }
}

impl From<isize> for CurIndex {
    fn from(val: isize) -> Self { Self(val as CurIndexNumber) }
}

impl From<i32> for CurIndex {
    fn from(val: i32) -> Self { Self(val as CurIndexNumber) }
}

impl From<i16> for CurIndex {
    fn from(val: i16) -> Self { Self(val) }
}

impl CurIndex {
    /// This won't be negative. Even if a negative number is passed in, it will be
    /// converted to 0.
    pub fn as_usize(self) -> usize {
        if self.0 < 0 {
            0
        } else {
            self.0.try_into().unwrap_or(self.0 as usize)
        }
    }

    /// If the history buffer is empty, this will return `None`. Otherwise, it will return
    /// `Some(true)` if the current index is at the start of the history buffer, and
    /// `Some(false)` otherwise.
    pub fn is_at_start(&self, versions: &sizing::VersionHistory) -> Option<bool> {
        if versions.is_empty() {
            None
        } else {
            Some(self.0 == MIN_INDEX)
        }
    }

    /// If the history buffer is empty, this will return `None`. Otherwise, it will return
    /// `Some(true)` if the current index is at the end of the history buffer, and
    /// `Some(false)` otherwise.
    pub fn is_at_end(&self, versions: &sizing::VersionHistory) -> Option<bool> {
        if versions.is_empty() {
            None
        } else {
            let max_index = ch(versions.len()) - ch(1);
            Some(self.0 == i16(max_index))
        }
    }

    /// Reset the current index to the start of the history buffer.
    pub fn clear(&mut self) { self.0 = MIN_INDEX; }

    /// Increment the current index. If the current index is at the end of the history
    /// buffer, or the buffer is empty, this does nothing.
    pub fn inc(&mut self, versions: &sizing::VersionHistory) {
        match self.is_at_end(versions) {
            None => {
                // Is empty. Nothing to increment.
            }
            Some(true) => {
                // Already at end of history buffer. Nothing to increment.
            }
            Some(false) => {
                // Increment index.
                self.0 += 1;
            }
        }
    }

    /// Decrement the current index. If the current index is at the start of the history
    /// buffer, or the buffer is empty, this does nothing.
    pub fn dec(&mut self, versions: &sizing::VersionHistory) {
        match self.is_at_start(versions) {
            None => {
                // Is empty. Nothing to decrement.
            }
            Some(true) => {
                // Already at start of history buffer. Nothing to decrement.
            }
            Some(false) => {
                // Decrement index.
                self.0 -= 1;
            }
        }
    }
}

impl EditorHistory {
    pub fn is_empty(&self) -> bool { self.versions.is_empty() }

    pub fn clear(&mut self) {
        self.versions.clear();
        self.current_index.clear();
    }

    /// Get the current index in the history buffer. If the buffer is empty, this will
    /// return `None`.
    pub fn current_index(&self) -> Option<CurIndex> {
        if self.is_empty() {
            None
        } else {
            Some(self.current_index)
        }
    }

    /// This function adds a state to the history buffer. It is called whenever the
    /// content of the editor changes. Once this is called, the current index is
    /// incremented. And [EditorHistory::prev_content] can be called to undo.
    ///
    /// Any dangling redos are truncated when a new state is added to the buffer.
    pub fn add_content(&mut self, content: EditorContent) {
        if let Some(current_index) = self.current_index() {
            // Delete the history from the current version index + 1 to the end.
            self.versions.truncate(current_index.as_usize() + 1);
        }
        self.versions.add(content);
        self.current_index.inc(&self.versions);
    }

    /// This is the underlying function that enables undo. It changes the current index to
    /// the previous index in the versions buffer.
    ///
    /// Once called, you can use [EditorHistory::next_content] to redo, as long as the
    /// current index is not at the end of the versions buffer.
    pub fn prev_content(&mut self) -> Option<EditorContent> {
        match self.current_index.is_at_start(&self.versions) {
            None => {
                // Is empty. Nothing to undo.
                None
            }
            Some(true) => {
                // At start of history. Nothing to undo.
                None
            }
            Some(false) => {
                // Decrement index.
                self.current_index.dec(&self.versions);

                // Return item at index.
                self.versions.get(self.current_index.as_usize()).cloned()
            }
        }
    }

    /// This is the underlying function that enables redo. It changes the current index to
    /// the next index in the versions buffer.
    ///
    /// You can call [EditorHistory::prev_content] to undo, as long as the current index
    /// is not at the start of the versions buffer.
    pub fn next_content(&mut self) -> Option<EditorContent> {
        match self.current_index.is_at_end(&self.versions) {
            None => {
                // Is empty. Nothing to redo.
                None
            }
            Some(true) => {
                // At end of history. Nothing to redo.
                None
            }
            Some(false) => {
                // Increment index.
                self.current_index.inc(&self.versions);

                // Return item at index.
                self.versions.get(self.current_index.as_usize()).cloned()
            }
        }
    }
}

impl Default for EditorHistory {
    fn default() -> Self {
        Self {
            versions: sizing::VersionHistory::new(),
            current_index: CurIndex::default(),
        }
    }
}

pub fn clear(buffer: &mut EditorBuffer) { buffer.history.clear(); }

pub fn add(buffer: &mut EditorBuffer) {
    // Invalidate the content cache, since the content just changed.
    cache::clear(buffer);

    // Normal history insertion.
    let content_copy = buffer.content.clone();
    buffer.history.add_content(content_copy);

    call_if_true!(DEBUG_TUI_COPY_PASTE, {
        tracing::debug!("🍎🍎🍎 add_content_to_undo_stack buffer: {:?}", buffer);
    });
}

pub fn undo(buffer: &mut EditorBuffer) {
    // Invalidate the content cache, since the content just changed.
    cache::clear(buffer);

    if let Some(content) = buffer.history.prev_content() {
        buffer.content = content;
    }

    call_if_true!(DEBUG_TUI_COPY_PASTE, {
        tracing::debug!("🍎🍎🍎 undo buffer: {:?}", buffer);
    });
}

pub fn redo(buffer: &mut EditorBuffer) {
    // Invalidate the content cache, since the content just changed.
    cache::clear(buffer);

    if let Some(content) = buffer.history.next_content() {
        buffer.content = content;
    }

    call_if_true!(DEBUG_TUI_COPY_PASTE, {
        tracing::debug!("🍎🍎🍎 redo buffer: {:?}", buffer);
    });
}

mod impl_debug_format {
    use super::*;

    impl Debug for EditorHistory {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
            "EditorHistory [index: {index:?} | versions: {len} | size: {size}]",
                len = self.versions.len(),
                size = format_as_kilobytes_with_commas(self.size_of().total_bytes()),
                index = self.current_index.0
            }
        }
    }
}

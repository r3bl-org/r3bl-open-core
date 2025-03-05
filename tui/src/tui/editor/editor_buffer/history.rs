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

use r3bl_core::{format_as_kilobytes_with_commas, RingBuffer as _};

use super::{cur_index::{CurIndex, CurIndexLoc},
            sizing,
            EditorContent};

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
    pub versions: super::sizing::HistoryBuffer,
    pub current_index: CurIndex,
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
    /// incremented. And [EditorHistory::undo()] can be called to undo.
    ///
    /// Any dangling redos are truncated when a new state is added to the buffer.
    pub fn add(&mut self, content: EditorContent) {
        match self.locate_current_index() {
            CurIndexLoc::Start(current_index)
            | CurIndexLoc::End(current_index)
            | CurIndexLoc::Middle(current_index) => {
                // Delete the history from the current version index + 1 to the end.
                self.versions.truncate(current_index.as_usize() + 1);
            }
            CurIndexLoc::EmptyHistory => {}
        }

        self.versions.add(content);
        CurIndexLoc::inc(&mut self.current_index, &self.versions);
    }

    /// This is the underlying function that enables undo. It changes the current index to
    /// the previous index in the versions buffer.
    ///
    /// Once called, you can use [EditorHistory::redo()] to redo, as long as the
    /// current index is not at the end of the versions buffer.
    pub fn undo(&mut self) -> Option<EditorContent> {
        match self.locate_current_index() {
            CurIndexLoc::EmptyHistory => {
                // Is empty. Nothing to undo.
                None
            }
            CurIndexLoc::Start(_) => {
                // At start of history. Nothing to undo.
                None
            }
            CurIndexLoc::End(_) | CurIndexLoc::Middle(_) => {
                // Decrement index.
                CurIndexLoc::dec(&mut self.current_index, &self.versions);

                // Return item at index.
                self.versions.get(self.current_index.as_usize()).cloned()
            }
        }
    }

    /// This is the underlying function that enables redo. It changes the current index to
    /// the next index in the versions buffer.
    ///
    /// You can call [EditorHistory::undo()] to undo, as long as the current index is not
    /// at the start of the versions buffer.
    pub fn redo(&mut self) -> Option<EditorContent> {
        match self.locate_current_index() {
            CurIndexLoc::EmptyHistory => {
                // Is empty. Nothing to redo.
                None
            }
            CurIndexLoc::End(_) => {
                // At end of history. Nothing to redo.
                None
            }
            CurIndexLoc::Start(_) | CurIndexLoc::Middle(_) => {
                // Increment index.
                CurIndexLoc::inc(&mut self.current_index, &self.versions);

                // Return item at index.
                self.versions.get(self.current_index.as_usize()).cloned()
            }
        }
    }

    /// Convenience method that calls [CurIndexLoc::locate()].
    pub fn locate_current_index(&self) -> CurIndexLoc {
        CurIndexLoc::locate(&self.current_index, &self.versions)
    }
}

impl Default for EditorHistory {
    fn default() -> Self {
        Self {
            versions: sizing::HistoryBuffer::new(),
            current_index: CurIndex::default(),
        }
    }
}

mod impl_debug_format {
    use super::*;

    impl Debug for EditorHistory {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            use r3bl_core::GetMemSize as _;
            let self_mem_size = self.get_mem_size();
            let size_fmt = format_as_kilobytes_with_commas(self_mem_size);

            write! {
                f,
            "EditorHistory [index: {index:?} | versions: {len} | size: {size}]",
                len = self.versions.len(),
                size = size_fmt,
                index = self.current_index.0
            }
        }
    }
}

#[cfg(test)]
mod tests_editor_history_struct {
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::cur_index::MIN_INDEX;

    #[test]
    fn test_editor_history_struct_one_item() {
        let mut history = EditorHistory::default();
        assert_eq2!(history.versions.len(), 0);
        assert_eq2!(history.current_index, CurIndex(MIN_INDEX));
        assert_eq2!(history.locate_current_index(), CurIndexLoc::EmptyHistory);
        assert!(history.is_empty());

        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_index, 0.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(0.into()));
        assert_eq!(history.locate_current_index(), CurIndexLoc::End(0.into()));

        // Can't redo, since there is only one version, can only undo.
        assert!(history.redo().is_none());
        assert_eq!(history.current_index, 0.into());
        assert_eq!(history.locate_current_index(), CurIndexLoc::End(0.into()));

        // Can undo, since there is only one version. And current_index is 0.
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, CurIndex(MIN_INDEX));
        assert_eq!(
            history.locate_current_index(),
            CurIndexLoc::Start(CurIndex(MIN_INDEX))
        );

        // Can redo, since there is only one version. And current_index is -1.
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 0.into());
        assert_eq!(history.locate_current_index(), CurIndexLoc::End(0.into()));
    }

    #[test]
    fn test_editor_history_struct_multiple_items() {
        let mut history = EditorHistory::default();

        // Add 3 items to the history.
        history.add(EditorContent::default());
        history.add(EditorContent::default());
        history.add(EditorContent::default());

        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.current_index, 2.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2.into()));

        // Can undo, since there are 3 versions. And current_index is 2.
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 0.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, CurIndex(-1));
        assert!(history.undo().is_none());

        // Can redo, 3 times.
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 0.into());
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 1.into());
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 2.into());
        assert!(history.redo().is_none());
    }

    #[test]
    fn test_editor_history_struct_truncate_dangling_redos() {
        let mut history = EditorHistory::default();

        // Add 3 items to the history.
        history.add(EditorContent::default());
        history.add(EditorContent::default());
        history.add(EditorContent::default());
        history.add(EditorContent::default());

        assert_eq!(history.versions.len(), 4);
        assert_eq!(history.current_index, 3.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(3.into()));

        // Undo twice. Can undo 4 times, since there are 4 versions. And current_index is
        // 3.
        assert!(history.undo().is_some());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1.into());
        assert_eq!(history.versions.len(), 4);

        // Add new content (+1) which should truncate the 2 dangling redos (-2).
        // So net change in versions.len() 4 - 2 + 1 = 3.
        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.current_index, 2.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2.into()));
    }
}

#[cfg(test)]
mod tests_history_functions {
    use r3bl_core::{assert_eq2, GCStringExt as _, RingBuffer as _};
    use smallvec::smallvec;

    use crate::{cur_index::CurIndex, EditorBuffer};

    #[test]
    fn test_push_default() {
        let mut buffer = EditorBuffer::default();
        let content = buffer.content.clone();

        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack.get(0).unwrap(), &content);
    }

    #[test]
    fn test_push_with_contents() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack.get(0).unwrap().lines.len(), 1);
        assert_eq2!(
            history_stack.get(0).unwrap().lines[0],
            "abc".grapheme_string()
        );
    }

    #[test]
    fn test_push_and_drop_future_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());

        buffer.content.lines = smallvec!["ghi".grapheme_string()];
        buffer.add();

        // 3 pushes, so the current index should be 2.
        assert_eq2!(buffer.history.current_index, 2.into());

        // Do two undos.
        buffer.undo();
        buffer.undo();
        // The current index should be 0.
        assert_eq!(buffer.history.current_index, 0.into());
        // There are two versions ahead of the current index.
        assert_eq!(buffer.history.versions.len(), 3);

        // Push new content. Should drop future redos (2 versions should be removed).
        buffer.content.lines = smallvec!["xyz".grapheme_string()];
        buffer.add();
        assert_eq!(buffer.history.current_index, 1.into());
        assert_eq!(buffer.history.versions.len(), 2);

        let history = buffer.history;
        assert_eq2!(history.current_index, 1.into());

        let history_stack = history.versions;
        assert_eq2!(history_stack.len(), 2);
        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "xyz".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_single_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.history.current_index, CurIndex(-1));
    }

    #[test]
    fn test_many_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());
        let copy_of_editor_content = buffer.content.clone();

        buffer.content.lines = smallvec!["ghi".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 2.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.history.current_index, 1.into());
        assert_eq2!(buffer.content, copy_of_editor_content);

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 3);

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".grapheme_string());
                }
                2 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "ghi".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_multiple_undos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());

        // Undo multiple times.
        buffer.undo();
        buffer.undo();
        buffer.undo();

        assert_eq2!(buffer.history.current_index, CurIndex(-1));
    }

    #[test]
    fn test_undo_and_multiple_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());
        let snapshot_content = buffer.content.clone();

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.history.current_index, 0.into());

        // Redo.
        buffer.redo();
        assert_eq2!(buffer.history.current_index, 1.into());

        // Current state.
        assert_eq2!(buffer.content, snapshot_content);

        // Redo.
        buffer.redo();

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 2);

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }
}

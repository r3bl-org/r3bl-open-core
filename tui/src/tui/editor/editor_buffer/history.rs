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

use super::{cur_index::{CurIndex, CurIndexLoc},
            EditorContent};
use crate::{format_as_kilobytes_with_commas, idx, Length, RingBuffer};

/// The `EditorHistory` struct manages the undo/redo functionality for the `EditorBuffer`.
///
/// - It uses a ring buffer (`versions`) to store the different states of the
///   `EditorContent`.
/// - It works hand in hand with the `current_index` field points to the current state in
///   the `versions` buffer. Please see the [`super::history::CurIndex`] for details on
///   how the `current_index` works.
///
/// # Pushing a new state [`self::EditorHistory::add`]
///
/// 1. If the `current_index` is not the last index in the `versions` buffer, the history
///    from `current_index + 1` to the end of the buffer is truncated (removed). This
///    discards any "future" states that were previously redone.
/// 2. A copy of the current `EditorContent` is added to the `versions` buffer.
/// 3. The `current_index` is incremented to point to the newly added state.
///
/// # Undoing [`self::EditorHistory::undo`]
///
/// 1. If the history is empty, return `None`.
/// 2. If the current index is at the start of the history, return `None`.
/// 3. Decrement the `current_index`.
/// 4. Return the `EditorContent` at the new `current_index` from the `versions` buffer.
///
/// # Redoing [`self::EditorHistory::redo`]
///
/// 1. If the history is empty, return `None`.
/// 2. If the current index is at the end of the history, return `None`.
/// 3. Increment the `current_index`.
/// 4. Return the `EditorContent` at the new `current_index` from the `versions` buffer.
///
/// # Notes
///
/// - The `versions` buffer has a maximum size (`MAX_UNDO_REDO_SIZE`). When the buffer is
///   full, adding a new state will overwrite the oldest state in the buffer.
/// - The caret position is retained during undo / redo operations. However, not all
///   editor events will trigger a new state to be added to the history buffer. See
///   [`crate::editor_engine::engine_public_api::apply_event()`] to see which events
///   actually get added to the editor history buffer.
#[derive(Clone, PartialEq, Default)]
pub struct EditorHistory {
    pub versions: super::sizing::HistoryBuffer,
    pub current_index: CurIndex,
}

impl EditorHistory {
    #[must_use]
    pub fn is_empty(&self) -> bool { self.versions.is_empty() }

    pub fn clear(&mut self) {
        self.versions.clear();
        self.current_index.clear();
    }

    /// Get the current index in the history buffer. If the buffer is empty, this will
    /// return `None`.
    #[must_use]
    pub fn current_index(&self) -> Option<CurIndex> {
        if self.is_empty() {
            None
        } else {
            Some(self.current_index)
        }
    }

    /// This function adds a state to the history buffer. It is called whenever the
    /// content of the editor changes. Once this is called, the current index is
    /// incremented. And [`EditorHistory::undo()`] can be called to undo.
    ///
    /// Any dangling redos are truncated when a new state is added to the buffer.
    pub fn add(&mut self, content: EditorContent) {
        match self.locate_current_index() {
            CurIndexLoc::End(current_index) | CurIndexLoc::Middle(current_index) => {
                // Delete the history from the current version index + 1 to the end.
                self.versions.truncate((current_index + idx(1)).as_usize());
            }
            CurIndexLoc::Start => {
                // Delete the entire history.
                self.versions.truncate(0);
            }
            CurIndexLoc::EmptyHistory => {}
        }

        self.versions.add(content);
        CurIndexLoc::inc(&mut self.current_index, Length::from(self.versions.len().as_usize()));
    }

    /// This is the underlying function that enables undo. It changes the current index to
    /// the previous index in the versions buffer.
    ///
    /// Once called, you can use [`EditorHistory::redo()`] to redo, as long as the
    /// current index is not at the end of the versions buffer.
    pub fn undo(&mut self) -> Option<EditorContent> {
        match self.locate_current_index() {
            CurIndexLoc::EmptyHistory => {
                // Is empty. Nothing to undo.
                None
            }
            CurIndexLoc::Start => {
                // Decrement index.
                CurIndexLoc::dec(&mut self.current_index, self.versions.len());
                // At start of history. Nothing to undo.
                None
            }
            CurIndexLoc::End(_) | CurIndexLoc::Middle(_) => {
                // Decrement index.
                CurIndexLoc::dec(&mut self.current_index, self.versions.len());

                // Return item at index.
                self.versions.get(self.current_index.as_index()).cloned()
            }
        }
    }

    /// This is the underlying function that enables redo. It changes the current index to
    /// the next index in the versions buffer.
    ///
    /// You can call [`EditorHistory::undo()`] to undo, as long as the current index is
    /// not at the start of the versions buffer.
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
            CurIndexLoc::Start | CurIndexLoc::Middle(_) => {
                // Increment index.
                CurIndexLoc::inc(&mut self.current_index, self.versions.len());

                // Return item at index.
                self.versions.get(self.current_index.as_index()).cloned()
            }
        }
    }

    /// Convenience method that calls [`CurIndexLoc::locate()`].
    #[must_use]
    pub fn locate_current_index(&self) -> CurIndexLoc {
        CurIndexLoc::locate(&self.current_index, Length::from(self.versions.len().as_usize()))
    }
}

mod impl_debug_format {
    use super::{format_as_kilobytes_with_commas, Debug, EditorHistory, Formatter,
                Result, RingBuffer};

    impl Debug for EditorHistory {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            use crate::GetMemSize;
            let self_mem_size = self.get_mem_size();
            let size_fmt = format_as_kilobytes_with_commas(self_mem_size);

            write!(
                f,
                "EditorHistory [index: {index:?} | versions.len(): {len} | size: {size}]",
                len = self.versions.len().as_usize(),
                size = size_fmt,
                index = self.current_index.0
            )
        }
    }
}

#[cfg(test)]
mod tests_editor_history_struct {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_editor_history_struct_one_item() {
        let mut history = EditorHistory::default();
        assert_eq2!(history.versions.len(), 0.into());
        assert_eq2!(history.current_index, CurIndex(None));
        assert_eq2!(history.locate_current_index(), CurIndexLoc::EmptyHistory);
        assert!(history.is_empty());

        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), 1.into());
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
        assert_eq!(history.current_index, CurIndex(None));
        assert_eq!(history.locate_current_index(), CurIndexLoc::Start);

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

        assert_eq!(history.versions.len(), 3.into());
        assert_eq!(history.current_index, 2.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2.into()));

        // Can undo, since there are 3 versions. And current_index is 2.
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 0.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, CurIndex(None));
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

        assert_eq!(history.versions.len(), 4.into());
        assert_eq!(history.current_index, 3.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(3.into()));

        // Undo twice. Can undo 4 times, since there are 4 versions. And current_index is
        // 3.
        assert!(history.undo().is_some());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1.into());
        assert_eq!(history.versions.len(), 4.into());

        // Add new content (+1) which should truncate the 2 dangling redos (-2).
        // So net change in versions.len() 4 - 2 + 1 = 3.
        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), 3.into());
        assert_eq!(history.current_index, 2.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2.into()));
    }
}

#[cfg(test)]
mod tests_history_functions {
    use smallvec::smallvec;

    use crate::{assert_eq2, cur_index::CurIndex, EditorBuffer,
                Length, RingBuffer};

    #[test]
    fn test_push_default() {
        let mut buffer = EditorBuffer::default();
        let content = buffer.content.clone();

        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), Length::from(1));
        assert_eq2!(history_stack.get(0).unwrap(), &content);
    }

    #[test]
    fn test_push_with_contents() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), Length::from(1));
        assert_eq2!(history_stack.get(0).unwrap().lines.len(), 1);
        assert_eq2!(
            history_stack.get(0).unwrap().lines[0],
            "abc".into()
        );
    }

    #[test]
    fn test_push_and_drop_future_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());

        buffer.content.lines = smallvec!["ghi".into()];
        buffer.add();

        // 3 pushes, so the current index should be 2.
        assert_eq2!(buffer.history.current_index, 2.into());

        // Do two undos.
        buffer.undo();
        buffer.undo();
        // The current index should be 0.
        assert_eq!(buffer.history.current_index, 0.into());
        // There are two versions ahead of the current index.
        assert_eq!(buffer.history.versions.len(), Length::from(3));

        // Push new content. Should drop future redos (2 versions should be removed).
        buffer.content.lines = smallvec!["xyz".into()];
        buffer.add();
        assert_eq!(buffer.history.current_index, 1.into());
        assert_eq!(buffer.history.versions.len(), Length::from(2));

        let history = buffer.history;
        assert_eq2!(history.current_index, 1.into());

        let history_stack = history.versions;
        assert_eq2!(history_stack.len(), Length::from(2));
        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".into());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "xyz".into());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_single_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.history.current_index, CurIndex(None));
    }

    #[test]
    fn test_many_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());
        let copy_of_editor_content = buffer.content.clone();

        buffer.content.lines = smallvec!["ghi".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 2.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.history.current_index, 1.into());
        assert_eq2!(buffer.content, copy_of_editor_content);

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), Length::from(3));

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".into());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".into());
                }
                2 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "ghi".into());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_multiple_undos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 1.into());

        // Undo multiple times.
        buffer.undo();
        buffer.undo();
        buffer.undo();

        assert_eq2!(buffer.history.current_index, CurIndex(None));
    }

    #[test]
    fn test_undo_and_multiple_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".into()];
        buffer.add();
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".into()];
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
        assert_eq2!(history_stack.len(), Length::from(2));

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".into());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".into());
                }
                _ => unreachable!(),
            }
        }
    }
}

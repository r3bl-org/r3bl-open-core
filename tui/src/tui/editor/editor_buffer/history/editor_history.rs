// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::history_cursor::{HistoryCursor, HistoryCursorLoc};
use crate::{EditorContent, GetMemSize, RingBuffer, RingBufferHeap,
            format_as_kilobytes_with_commas, get_mem_size};
use std::{fmt::{Debug, Formatter},
          mem::size_of};

/// This is the absolute maximum number of undo/redo steps that will ever be stored.
pub const MAX_UNDO_REDO_SIZE: usize = 16;

/// The version history is stored on the heap, as a ring buffer.
type HistoryBuffer = RingBufferHeap<EditorContent, MAX_UNDO_REDO_SIZE>;

/// Manages the undo/redo functionality for the [`EditorBuffer`].
///
/// - It uses a ring buffer (`versions`) to store the different states of the
///   [`EditorContent`].
/// - It works hand in hand with the [`current_index`] field pointing to the current state
///   in the [`versions`] buffer. Please see [`HistoryCursor`] for details on how the
///   [`current_index`] works.
///
/// [`current_index`]: EditorHistory::current_index
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`versions`]: EditorHistory::versions
#[derive(Clone, PartialEq, Default)]
pub struct EditorHistory {
    pub versions: HistoryBuffer,
    pub current_index: HistoryCursor,
}

impl GetMemSize for EditorHistory {
    fn get_mem_size(&self) -> usize {
        let versions_size = get_mem_size::ring_buffer_size(&self.versions);
        let current_index_field_size = size_of::<HistoryCursor>();
        versions_size + current_index_field_size
    }
}

impl EditorHistory {
    #[must_use]
    pub fn is_empty(&self) -> bool { self.versions.is_empty() }

    pub fn clear(&mut self) {
        self.versions.clear();
        self.current_index.clear();
    }

    /// Gets the current index in the history buffer. If the buffer is empty, this will
    /// return `None`.
    #[must_use]
    pub fn current_index(&self) -> Option<HistoryCursor> {
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
            HistoryCursorLoc::End(current_index)
            | HistoryCursorLoc::Middle(current_index) => {
                // Delete the history from the current version index + 1 to the end.
                self.versions.truncate(current_index + 1);
            }
            HistoryCursorLoc::Start => {
                // Delete the entire history.
                self.versions.truncate(0);
            }
            HistoryCursorLoc::EmptyHistory => {}
        }

        self.versions.add(content);
        HistoryCursorLoc::inc(&mut self.current_index, self.versions.len());
    }

    /// This is the underlying function that enables undo. It changes the current index to
    /// the previous index in the versions buffer.
    ///
    /// Once called, you can use [`EditorHistory::redo()`] to redo, as long as the
    /// current index is not at the end of the versions buffer.
    pub fn undo(&mut self) -> Option<EditorContent> {
        match self.locate_current_index() {
            HistoryCursorLoc::EmptyHistory => {
                // Is empty. Nothing to undo.
                None
            }
            HistoryCursorLoc::Start => {
                // Decrement index.
                HistoryCursorLoc::dec(&mut self.current_index, self.versions.len());
                // At start of history. Nothing to undo.
                None
            }
            HistoryCursorLoc::End(_) | HistoryCursorLoc::Middle(_) => {
                // Decrement index.
                HistoryCursorLoc::dec(&mut self.current_index, self.versions.len());

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
            HistoryCursorLoc::EmptyHistory => {
                // Is empty. Nothing to redo.
                None
            }
            HistoryCursorLoc::End(_) => {
                // At end of history. Nothing to redo.
                None
            }
            HistoryCursorLoc::Start | HistoryCursorLoc::Middle(_) => {
                // Increment index.
                HistoryCursorLoc::inc(&mut self.current_index, self.versions.len());

                // Return item at index.
                self.versions.get(self.current_index.as_index()).cloned()
            }
        }
    }

    /// Convenience method that calls [`HistoryCursorLoc::locate()`].
    #[must_use]
    pub fn locate_current_index(&self) -> HistoryCursorLoc {
        HistoryCursorLoc::locate(&self.current_index, self.versions.len())
    }
}

mod impl_debug_format {
    use super::{Debug, EditorHistory, Formatter, RingBuffer,
                format_as_kilobytes_with_commas};
    use crate::GetMemSize;

    impl Debug for EditorHistory {
        /// # Implementation Note: Intentional Use of Raw `usize`
        ///
        /// Uses `.as_usize()` for Debug formatting output only.
        /// Type-safe bounds checking not needed for debug display.
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    use crate::{assert_eq2, c_index, c_len};

    #[test]
    fn test_editor_history_struct_one_item() {
        let mut history = EditorHistory::default();
        assert_eq2!(history.versions.len(), c_len(0));
        assert_eq2!(history.current_index, HistoryCursor(None));
        assert_eq2!(
            history.locate_current_index(),
            HistoryCursorLoc::EmptyHistory
        );
        assert!(history.is_empty());

        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), c_len(1));
        assert_eq!(history.current_index, 0usize.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(0usize.into()));
        assert_eq!(
            history.locate_current_index(),
            HistoryCursorLoc::End(c_index(0))
        );

        // Can't redo, since there is only one version, can only undo.
        assert!(history.redo().is_none());
        assert_eq!(history.current_index, 0usize.into());
        assert_eq!(
            history.locate_current_index(),
            HistoryCursorLoc::End(c_index(0))
        );

        // Can undo, since there is only one version. And current_index is 0.
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, HistoryCursor(None));
        assert_eq!(history.locate_current_index(), HistoryCursorLoc::Start);

        // Can redo, since there is only one version. And current_index is -1.
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 0usize.into());
        assert_eq!(
            history.locate_current_index(),
            HistoryCursorLoc::End(c_index(0))
        );
    }

    #[test]
    fn test_editor_history_struct_multiple_items() {
        let mut history = EditorHistory::default();

        // Add 3 items to the history.
        history.add(EditorContent::default());
        history.add(EditorContent::default());
        history.add(EditorContent::default());

        assert_eq!(history.versions.len(), c_len(3));
        assert_eq!(history.current_index, 2usize.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2usize.into()));

        // Can undo, since there are 3 versions. And current_index is 2.
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1usize.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 0usize.into());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, HistoryCursor(None));
        assert!(history.undo().is_none());

        // Can redo, 3 times.
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 0usize.into());
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 1usize.into());
        assert!(history.redo().is_some());
        assert_eq!(history.current_index, 2usize.into());
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

        assert_eq!(history.versions.len(), c_len(4));
        assert_eq!(history.current_index, 3usize.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(3usize.into()));

        // Undo twice. Can undo 4 times, since there are 4 versions. And current_index is
        // 3.
        assert!(history.undo().is_some());
        assert!(history.undo().is_some());
        assert_eq!(history.current_index, 1usize.into());
        assert_eq!(history.versions.len(), c_len(4));

        // Add new content (+1) which should truncate the 2 dangling redos (-2).
        // So net change in versions.len() 4 - 2 + 1 = 3.
        history.add(EditorContent::default());
        assert_eq!(history.versions.len(), c_len(3));
        assert_eq!(history.current_index, 2usize.into());
        assert!(!history.is_empty());
        assert_eq!(history.current_index(), Some(2usize.into()));
    }
}

#[cfg(test)]
mod tests_history_functions {
    use crate::{EditorBuffer, HistoryCursor, RingBuffer, VPSize, assert_eq2, c_len,
                c_row};

    fn set_single_line(buffer: &mut EditorBuffer, line: &str) {
        let buffer_mut = buffer.get_mut(VPSize::default());
        buffer_mut.inner.lines.clear();
        buffer_mut.inner.lines.push_line(line);
    }

    #[test]
    fn test_push_default() {
        let mut buffer = EditorBuffer::new_empty(());

        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        let history_stack = &buffer.get_history().versions;
        assert_eq2!(history_stack.len(), c_len(1));
        assert_eq2!(
            history_stack
                .get(0)
                .expect("conversion error")
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            ""
        );
    }

    #[test]
    fn test_push_with_contents() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        let history_stack = &buffer.get_history().versions;
        assert_eq2!(history_stack.len(), c_len(1));
        assert_eq2!(
            history_stack
                .get(0)
                .expect("conversion error")
                .get_lines()
                .get_c_len(),
            c_len(1)
        );
        assert_eq2!(
            history_stack
                .get(0)
                .expect("conversion error")
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "abc"
        );
    }

    #[test]
    fn test_push_and_drop_future_redos() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        set_single_line(&mut buffer, "def");
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());

        set_single_line(&mut buffer, "ghi");
        buffer.add();

        // 3 pushes, so the current index should be 2.
        assert_eq2!(buffer.get_history().current_index, 2usize.into());

        // Do two undos.
        buffer.undo();
        buffer.undo();
        // The current index should be 0.
        assert_eq!(buffer.get_history().current_index, 0usize.into());
        // There are two versions ahead of the current index.
        assert_eq!(buffer.get_history().versions.len(), c_len(3));

        // Push new content. Should drop future redos (2 versions should be removed).
        set_single_line(&mut buffer, "xyz");
        buffer.add();
        assert_eq!(buffer.get_history().current_index, 1usize.into());
        assert_eq!(buffer.get_history().versions.len(), c_len(2));

        let history = buffer.get_history();
        assert_eq2!(history.current_index, 1usize.into());

        let history_stack = &history.versions;
        assert_eq2!(history_stack.len(), c_len(2));
        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "abc"
                    );
                }
                1 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "xyz"
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_single_undo() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.get_history().current_index, HistoryCursor(None));
    }

    #[test]
    fn test_many_undo() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        set_single_line(&mut buffer, "def");
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());
        let copy_of_line = buffer.get_line_at_row_index(c_row(0)).unwrap().to_string();

        set_single_line(&mut buffer, "ghi");
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 2usize.into());

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());
        assert_eq2!(
            buffer.get_line_at_row_index(c_row(0)).unwrap(),
            copy_of_line.as_str()
        );

        let history_stack = &buffer.get_history().versions;
        assert_eq2!(history_stack.len(), c_len(3));

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "abc"
                    );
                }
                1 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "def"
                    );
                }
                2 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "ghi"
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_multiple_undos() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        set_single_line(&mut buffer, "def");
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());

        // Undo multiple times.
        buffer.undo();
        buffer.undo();
        buffer.undo();

        assert_eq2!(buffer.get_history().current_index, HistoryCursor(None));
    }

    #[test]
    fn test_undo_and_multiple_redos() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["abc"]);
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        set_single_line(&mut buffer, "def");
        buffer.add();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());
        let snapshot_line = buffer.get_line_at_row_index(c_row(0)).unwrap().to_string();

        // Undo.
        buffer.undo();
        assert_eq2!(buffer.get_history().current_index, 0usize.into());

        // Redo.
        buffer.redo();
        assert_eq2!(buffer.get_history().current_index, 1usize.into());

        // Current state.
        assert_eq2!(
            buffer.get_line_at_row_index(c_row(0)).unwrap(),
            snapshot_line.as_str()
        );

        // Redo.
        buffer.redo();

        let history_stack = &buffer.get_history().versions;
        assert_eq2!(history_stack.len(), c_len(2));

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "abc"
                    );
                }
                1 => {
                    assert_eq2!(content.get_lines().get_c_len(), c_len(1));
                    assert_eq2!(
                        content
                            .get_lines()
                            .get_line_content(c_row(0))
                            .expect("conversion error"),
                        "def"
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}

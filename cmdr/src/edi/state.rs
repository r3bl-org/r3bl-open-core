// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnalyticsAction, edi::Id, report_analytics};
use r3bl_tui::{DEBUG_TUI_MOD, DEFAULT_SYN_HI_FILE_EXT, DialogBuffer, DocumentStorage,
               EditorBuffer, FileExtensionToken, FilePathToken, FlexBoxId,
               HasDialogBuffers, HasEditorBuffers, InlineString, TinyInlineString,
               fg_green, fg_red, inline_string, into_existing, ok};
use rustc_hash::FxHashMap;
use std::{ffi::OsStr,
          fmt::{Debug, Display, Formatter, Result},
          path::Path};

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: FxHashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: FxHashMap<FlexBoxId, DialogBuffer>,
}

pub mod constructor {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for State {
        fn default() -> Self {
            Self {
                editor_buffers: create_hash_map_of_editor_buffers(None),
                dialog_buffers: FxHashMap::default(),
            }
        }
    }

    #[must_use]
    pub fn new(maybe_file_path: Option<&str>) -> State {
        match maybe_file_path {
            Some(_) => State {
                editor_buffers: create_hash_map_of_editor_buffers(maybe_file_path),
                dialog_buffers: FxHashMap::default(),
            },
            None => State::default(),
        }
    }

    fn create_hash_map_of_editor_buffers(
        maybe_file_path: Option<&str>,
    ) -> FxHashMap<FlexBoxId, EditorBuffer> {
        let editor_buffer = {
            let file_ext = &file_utils::get_file_extension(maybe_file_path);

            let mut editor_buffer = match maybe_file_path {
                Some(file_path) => EditorBuffer::new_empty(
                    FileExtensionToken(file_ext) + FilePathToken(file_path),
                ),
                None => EditorBuffer::new_empty(()),
            };

            let content = file_utils::read_file_into_storage(maybe_file_path);
            editor_buffer.init_with(content.lines());
            editor_buffer
        };

        {
            let mut it = FxHashMap::default();
            it.insert(FlexBoxId::from(Id::ComponentEditor), editor_buffer);
            it
        }
    }
}

pub mod file_utils {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn get_file_extension(maybe_file_path: Option<&str>) -> TinyInlineString {
        if let Some(file_path) = maybe_file_path {
            let maybe_extension =
                Path::new(file_path).extension().and_then(OsStr::to_str);
            if let Some(extension) = maybe_extension {
                if extension.is_empty() {
                    return DEFAULT_SYN_HI_FILE_EXT.into();
                }
                return extension.into();
            }
        }

        DEFAULT_SYN_HI_FILE_EXT.into()
    }

    /// This is just a wrapper around
    /// [`into_existing::read_from_file::try_read_file_path_into_inline_string()`].
    pub fn read_file_into_storage(maybe_file_path: Option<&str>) -> DocumentStorage {
        // Create an empty document storage.
        let mut acc = DocumentStorage::new();

        // Read the file contents into acc if possible (file exists, have read
        // permissions, etc).
        if let Some(file_path) = maybe_file_path {
            match into_existing::read_from_file::try_read_file_path_into_inline_string(
                &mut acc, file_path,
            ) {
                Ok(()) => {
                    DEBUG_TUI_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "💾💾💾✅ Successfully read file",
                            file_path = ?file_path,
                            details = %fg_green(&inline_string!("{file_path:?}"))
                        );
                    });
                    return acc;
                }
                Err(error) => {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = "💾💾💾❌ Failed to read file",
                        file_path = ?file_path,
                        error = %fg_red(&inline_string!("{error:?}"))
                    );
                }
            }
        }

        acc
    }

    pub fn save_content_to_file(file_path: &str, content: &str) {
        let file_path = InlineString::from_str(file_path);
        let content = InlineString::from_str(content);

        tokio::spawn(async move {
            report_analytics::start_task_to_generate_event(
                String::new(),
                AnalyticsAction::EdiFileSave,
            );
            let result_file_write = std::fs::write(&*file_path, &content);
            match result_file_write {
                Ok(()) => {
                    DEBUG_TUI_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "💾💾💾❌ Successfully saved file",
                            file_path = %fg_green(&inline_string!("{file_path:?}"))
                        );
                    });
                }
                Err(error) => {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = "💾💾💾✅ Failed to save file",
                        file_path = %fg_red(&inline_string!("{error:?}"))
                    );
                }
            }
        });
    }
}

mod impl_editor_support {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl HasEditorBuffers for State {
        fn get_mut_editor_buffer(&mut self, id: FlexBoxId) -> Option<&mut EditorBuffer> {
            if let Some(buffer) = self.editor_buffers.get_mut(&id) {
                Some(buffer)
            } else {
                None
            }
        }

        fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer) {
            self.editor_buffers.insert(id, buffer);
        }

        fn contains_editor_buffer(&self, id: FlexBoxId) -> bool {
            self.editor_buffers.contains_key(&id)
        }
    }
}

mod impl_dialog_support {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl HasDialogBuffers for State {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
            self.dialog_buffers.get_mut(&id)
        }
    }
}

mod impl_debug {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(
                f,
                "State [
  - dialog_buffers:\n{:?}
  - editor_buffers:\n{:?}
]",
                self.dialog_buffers, self.editor_buffers,
            )
        }
    }
}

/// Efficient Display implementation for telemetry logging.
mod impl_display {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Display for State {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffers and dialog buffers. This is used for telemetry
        /// reporting, and it is expected to be fast, since it is called in a hot loop,
        /// on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Build compact telemetry format.
            write!(
                f,
                "State[editors={}, dialogs={}]",
                self.editor_buffers.len(),
                self.dialog_buffers.len()
            )?;

            // Add detailed buffer info if needed (with line breaks and indentation).
            if !self.editor_buffers.is_empty() {
                write!(f, "\n  editors=[")?;
                for (i, (id, buffer)) in self.editor_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\n    {id}:{buffer}")?;
                }
                write!(f, "\n  ]")?;
            }

            if !self.dialog_buffers.is_empty() {
                write!(f, "\n  dialogs=[")?;
                for (i, (id, buffer)) in self.dialog_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\n    {id}:{buffer}")?;
                }
                write!(f, "\n  ]")?;
            }

            ok!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{State, constructor, file_utils};
    use crate::edi::Id;
    use r3bl_tui::{DialogBuffer, EditorBuffer, FlexBoxId, HasDialogBuffers,
                   HasEditorBuffers, InlineString, InlineVec, assert_eq2,
                   friendly_random_id};

    #[test]
    fn test_file_extension() {
        let file_path = Some("foo.rs");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "rs");

        let file_path = Some("foo");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "md");

        let file_path = Some("foo.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "md");

        let file_path = Some("foo.bar.rs");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "rs");

        let file_path = Some("foo.bar");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "bar");

        let file_path = Some("foo.bar.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "md");

        let file_path = Some("foo.bar.baz");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "baz");

        let file_path = Some("foo.bar.baz.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "md");

        let file_path = None;
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq2!(file_ext, "md");
    }

    #[test]
    fn test_read_file_content() {
        // Make up a file name.
        let filename = &format!(
            "/tmp/{}_file.md",
            friendly_random_id::generate_friendly_random_id()
        );
        println!("🍍🍎🍏filename: {filename}");

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        let expected = file_utils::read_file_into_storage(Some(filename));
        assert_eq2!(expected, content);

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_state_constructor_with_file() {
        // Make up a file name.
        let filename = format!(
            "/tmp/{}_file.md",
            friendly_random_id::generate_friendly_random_id()
        );
        let maybe_file_path = Some(filename.as_str());
        println!("🍍🍎🍏filename: {filename}");

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        // Create a state.
        let state = constructor::new(maybe_file_path);

        // Check the state.
        assert_eq2!(state.editor_buffers.len(), 1);
        assert_eq2!(state.dialog_buffers.len(), 0);
        assert!(
            state
                .editor_buffers
                .contains_key(&FlexBoxId::from(Id::ComponentEditor))
        );

        let editor_buffer = state
            .editor_buffers
            .get(&FlexBoxId::from(Id::ComponentEditor))
            .unwrap();

        // Verify buffer metadata.
        assert_eq2!(
            editor_buffer.get_file_path(),
            Some(&InlineString::from_str(&filename))
        );
        assert_eq2!(editor_buffer.get_maybe_file_extension(), Some("md"));

        // Verify buffer content.
        assert_eq2!(editor_buffer.get_lines().get_line_count().as_usize(), 2);
        assert_eq2!(
            editor_buffer
                .get_lines()
                .iter_lines()
                .map(|line| line.content())
                .collect::<InlineVec<&str>>()
                .join("\n"),
            content
        );

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_state_constructor_empty() {
        // Test constructor::new(None) and State::default().
        let state_from_none = constructor::new(None);
        let state_default = State::default();
        assert_eq2!(state_from_none, state_default);

        assert_eq2!(state_from_none.editor_buffers.len(), 1);
        assert_eq2!(state_from_none.dialog_buffers.len(), 0);
        assert!(
            state_from_none
                .editor_buffers
                .contains_key(&FlexBoxId::from(Id::ComponentEditor))
        );

        let buffer = state_from_none
            .editor_buffers
            .get(&FlexBoxId::from(Id::ComponentEditor))
            .unwrap();

        assert_eq2!(buffer.get_file_path(), None);
        assert_eq2!(buffer.get_maybe_file_extension(), None);
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_has_editor_buffers_trait() {
        let mut state = State::default();
        let editor_id = FlexBoxId::from(Id::ComponentEditor);
        let non_existent_id = FlexBoxId::from(99);

        // Test contains_editor_buffer.
        assert!(state.contains_editor_buffer(editor_id));
        assert!(!state.contains_editor_buffer(non_existent_id));

        // Test get_mut_editor_buffer.
        assert!(state.get_mut_editor_buffer(editor_id).is_some());
        assert!(state.get_mut_editor_buffer(non_existent_id).is_none());

        // Test mutating via get_mut_editor_buffer.
        if let Some(buf) = state.get_mut_editor_buffer(editor_id) {
            buf.set_file_path(InlineString::from_str("updated.rs"));
        }
        assert_eq2!(
            state
                .editor_buffers
                .get(&editor_id)
                .unwrap()
                .get_file_path(),
            Some(&InlineString::from_str("updated.rs"))
        );

        // Test insert_editor_buffer.
        state.insert_editor_buffer(non_existent_id, EditorBuffer::new_empty(()));
        assert!(state.contains_editor_buffer(non_existent_id));
        assert_eq2!(state.editor_buffers.len(), 2);
    }

    #[test]
    fn test_has_dialog_buffers_trait() {
        let mut state = State::default();
        let dialog_id = FlexBoxId::from(100);

        // Initially no dialog buffer.
        assert!(state.get_mut_dialog_buffer(dialog_id).is_none());

        // Insert a dialog buffer and verify get_mut_dialog_buffer.
        state
            .dialog_buffers
            .insert(dialog_id, DialogBuffer::new_empty());
        assert!(state.get_mut_dialog_buffer(dialog_id).is_some());
    }

    #[test]
    fn test_state_display_and_debug() {
        let state = State::default();

        let display_str = format!("{state}");
        assert!(display_str.contains("State[editors=1, dialogs=0]"));
        assert!(display_str.contains("editors=["));

        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("State ["));
        assert!(debug_str.contains("editor_buffers:"));
    }
}

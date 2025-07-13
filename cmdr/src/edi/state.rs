/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::{collections::HashMap,
          ffi::OsStr,
          fmt::{Debug, Display, Formatter, Result},
          path::Path};

use r3bl_tui::{DEBUG_TUI_MOD, DEFAULT_SYN_HI_FILE_EXT, DialogBuffer, DocumentStorage,
               EditorBuffer, FlexBoxId, HasDialogBuffers, HasEditorBuffers,
               InlineString, TinyInlineString, fg_green, fg_red, inline_string,
               into_existing};

use crate::{AnalyticsAction, edi::Id, report_analytics};

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

#[cfg(test)]
mod state_tests {
    use r3bl_tui::{FlexBoxId, InlineVec, friendly_random_id};

    use super::{constructor, file_utils};
    use crate::edi::Id;

    #[test]
    fn test_file_extension() {
        let file_path = Some("foo.rs");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.rs");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo.bar");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "bar");

        let file_path = Some("foo.bar.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.baz");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "baz");

        let file_path = Some("foo.bar.baz.");
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "md");

        let file_path = None;
        let file_ext = file_utils::get_file_extension(file_path);
        assert_eq!(file_ext, "md");
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
        assert_eq!(expected, content);

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_state_constructor() {
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
        assert_eq!(state.editor_buffers.len(), 1);
        assert_eq!(state.dialog_buffers.len(), 0);
        assert!(
            state
                .editor_buffers
                .contains_key(&FlexBoxId::from(Id::ComponentEditor))
        );
        assert_eq!(
            state
                .editor_buffers
                .get(&FlexBoxId::from(Id::ComponentEditor))
                .unwrap()
                .content
                .lines
                .len(),
            2
        );
        assert_eq!(
            state
                .editor_buffers
                .get(&FlexBoxId::from(Id::ComponentEditor))
                .unwrap()
                .content
                .lines
                .iter()
                .map(|it| it.string.as_str())
                .collect::<InlineVec<&str>>()
                .join("\n"),
            content
        );

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }
}

pub mod constructor {
    use super::{EditorBuffer, FlexBoxId, HashMap, Id, State, file_utils};

    impl Default for State {
        fn default() -> Self {
            Self {
                editor_buffers: create_hash_map_of_editor_buffers(None),
                dialog_buffers: HashMap::default(),
            }
        }
    }

    #[must_use]
    pub fn new(maybe_file_path: Option<&str>) -> State {
        match maybe_file_path {
            Some(_) => State {
                editor_buffers: create_hash_map_of_editor_buffers(maybe_file_path),
                dialog_buffers: HashMap::default(),
            },
            None => State::default(),
        }
    }

    fn create_hash_map_of_editor_buffers(
        maybe_file_path: Option<&str>,
    ) -> HashMap<FlexBoxId, EditorBuffer> {
        let editor_buffer = {
            let file_ext = file_utils::get_file_extension(maybe_file_path);

            let mut editor_buffer =
                EditorBuffer::new_empty(Some(&file_ext), maybe_file_path);

            let content = file_utils::read_file_into_storage(maybe_file_path);
            editor_buffer.set_lines(content.lines());
            editor_buffer
        };

        {
            let mut it = HashMap::new();
            it.insert(FlexBoxId::from(Id::ComponentEditor), editor_buffer);
            it
        }
    }
}

pub mod file_utils {
    use super::{AnalyticsAction, DEBUG_TUI_MOD, DEFAULT_SYN_HI_FILE_EXT,
                DocumentStorage, InlineString, OsStr, Path, TinyInlineString, fg_green,
                fg_red, inline_string, into_existing, report_analytics};

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
    use super::{EditorBuffer, FlexBoxId, HasEditorBuffers, State};

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
    use super::{DialogBuffer, FlexBoxId, HasDialogBuffers, State};

    impl HasDialogBuffers for State {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
            self.dialog_buffers.get_mut(&id)
        }
    }
}

mod impl_debug {
    use super::{Debug, Formatter, Result, State};

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
    use super::{Display, Formatter, Result, State};

    impl Display for State {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffers and dialog buffers. This is used for telemetry
        /// reporting, and it is expected to be fast, since it is called in a hot loop,
        /// on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Build compact telemetry format
            write!(
                f,
                "State[editors={}, dialogs={}]",
                self.editor_buffers.len(),
                self.dialog_buffers.len()
            )?;

            // Add detailed buffer info if needed (more compact format)
            if !self.editor_buffers.is_empty() {
                write!(f, " editors=[")?;
                for (i, (id, buffer)) in self.editor_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{id}:{buffer}")?;
                }
                write!(f, "]")?;
            }

            if !self.dialog_buffers.is_empty() {
                write!(f, " dialogs=[")?;
                for (i, (id, buffer)) in self.dialog_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{id}:{buffer}")?;
                }
                write!(f, "]")?;
            }

            Ok(())
        }
    }
}

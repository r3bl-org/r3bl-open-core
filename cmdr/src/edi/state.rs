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
          fmt::{Debug, Formatter, Result},
          path::Path};

use crossterm::style::Stylize;
use r3bl_core::{CharStorage,
                DocumentStorage,
                StringStorage,
                call_if_true,
                into_existing::read_from_file::try_read_file_path_into_small_string,
                string_storage,
                style_error,
                style_primary};
use r3bl_tui::{DEBUG_TUI_MOD,
               DEFAULT_SYN_HI_FILE_EXT,
               DialogBuffer,
               EditorBuffer,
               FlexBoxId,
               HasDialogBuffers,
               HasEditorBuffers};

use crate::{AnalyticsAction, edi::Id, report_analytics};

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

#[cfg(test)]
mod state_tests {
    use r3bl_core::{VecArray, friendly_random_id};
    use r3bl_tui::FlexBoxId;

    use super::*;
    use crate::edi::Id;

    #[test]
    fn test_file_extension() {
        let file_path = Some("foo.rs");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.rs");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "rs");

        let file_path = Some("foo.bar");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "bar");

        let file_path = Some("foo.bar.");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = Some("foo.bar.baz");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "baz");

        let file_path = Some("foo.bar.baz.");
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");

        let file_path = None;
        let file_ext = file_utils::get_file_extension(&file_path);
        assert_eq!(file_ext, "md");
    }

    #[test]
    fn test_read_file_content() {
        // Make up a file name.
        let filename = &format!(
            "/tmp/{}_file.md",
            friendly_random_id::generate_friendly_random_id()
        );
        println!("🍍🍎🍏filename: {}", filename);

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        let expected = file_utils::read_file_into_storage(&Some(filename));
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
        println!("🍍🍎🍏filename: {}", filename);

        // Write some content to this file.
        let content = "This is a test.\nThis is only a test.";
        std::fs::write(filename.clone(), content).unwrap();

        // Create a state.
        let state = constructor::new(&maybe_file_path);

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
                .collect::<VecArray<&str>>()
                .join("\n"),
            content
        );

        // Delete the file.
        std::fs::remove_file(filename).unwrap();
    }
}

pub mod constructor {
    use super::*;

    impl Default for State {
        fn default() -> Self {
            Self {
                editor_buffers: create_hash_map_of_editor_buffers(&None),
                dialog_buffers: Default::default(),
            }
        }
    }

    pub fn new(maybe_file_path: &Option<&str>) -> State {
        match maybe_file_path {
            Some(_) => State {
                editor_buffers: create_hash_map_of_editor_buffers(maybe_file_path),
                dialog_buffers: Default::default(),
            },
            None => State::default(),
        }
    }

    fn create_hash_map_of_editor_buffers(
        maybe_file_path: &Option<&str>,
    ) -> HashMap<FlexBoxId, EditorBuffer> {
        let editor_buffer = {
            let file_ext = file_utils::get_file_extension(maybe_file_path);
            let mut editor_buffer =
                EditorBuffer::new_empty(&Some(&file_ext), maybe_file_path);
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
    use super::*;

    pub fn get_file_extension(maybe_file_path: &Option<&str>) -> CharStorage {
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

    /// This is just a wrapper around [try_read_file_path_into_small_string()].
    pub fn read_file_into_storage(maybe_file_path: &Option<&str>) -> DocumentStorage {
        // Create an empty document storage.
        let mut acc = DocumentStorage::new();

        // Read the file contents into acc if possible (file exists, have read
        // permissions, etc).
        if let Some(file_path) = maybe_file_path {
            match try_read_file_path_into_small_string(&mut acc, file_path) {
                Ok(_) => {
                    call_if_true!(DEBUG_TUI_MOD, {
                        let message = "\n💾💾💾✅ Successfully read file";
                        let details = string_storage!("{file_path:?}");
                        let details_fmt = style_primary(&details);
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = %message,
                            file_path = ?file_path,
                            details = %details_fmt
                        );
                    });
                    return acc;
                }
                Err(error) => {
                    let message = "\n💾💾💾❌ Failed to read file";
                    let details = string_storage!("{error:?}");
                    let details_fmt = style_error(&details);
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = %message,
                        file_path = ?file_path,
                        details = %details_fmt
                    );
                }
            }
        }

        acc
    }

    pub fn save_content_to_file(file_path: &str, content: &str) {
        let file_path = StringStorage::from_str(file_path);
        let content = StringStorage::from_str(content);

        tokio::spawn(async move {
            report_analytics::start_task_to_generate_event(
                "".to_string(),
                AnalyticsAction::EdiFileSave,
            );
            let result_file_write = std::fs::write(&*file_path, &content);
            match result_file_write {
                Ok(_) => {
                    call_if_true!(DEBUG_TUI_MOD, {
                        tracing::debug!(
                            "\n💾💾💾❌ Successfully saved file: {}",
                            format!("{file_path:?}").green()
                        );
                    });
                }
                Err(error) => {
                    tracing::error!(
                        "\n💾💾💾✅ Failed to save file: {}",
                        format!("{error:?}").red()
                    );
                }
            }
        });
    }
}

mod impl_editor_support {
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
    use super::*;

    impl HasDialogBuffers for State {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
            self.dialog_buffers.get_mut(&id)
        }
    }
}

mod impl_debug_format {
    use super::*;

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
"State [
  - dialog_buffers:\n{:?}
  - editor_buffers:\n{:?}
]",
                self.dialog_buffers, self.editor_buffers,
            }
        }
    }
}

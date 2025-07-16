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
          fmt::{Debug, Display, Formatter, Result}};

use r3bl_tui::{DEFAULT_SYN_HI_FILE_EXT, DialogBuffer, EditorBuffer, FlexBoxId,
               HasDialogBuffers, HasEditorBuffers};

use crate::ex_editor::Id;

/// Provides default content for the editor example.
///
/// This function loads real-world markdown content that demonstrates various
/// markdown features including metadata, headings with emojis, lists, code blocks,
/// and formatting. The content is shared between this example and the parser tests
/// to ensure consistency.
///
/// The content is loaded from the `r3bl_tui::editor::EX_EDITOR_CONTENT` constant,
/// which [`include_str!`] the content from an external markdown file, ensuring a
/// single source of truth for this example data.
///
/// # Returns
///
/// A vector of string slices, where each slice represents one line of the markdown
/// content. This format is suitable for initializing an `EditorBuffer`.
fn get_default_editor_content() -> Vec<&'static str> {
    r3bl_tui::editor::EX_EDITOR_CONTENT.lines().collect()
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

mod constructor {
    use super::{DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, FlexBoxId, HashMap, Id, State,
                constructor, get_default_editor_content};

    impl Default for State {
        fn default() -> Self { constructor::get_initial_state() }
    }

    pub fn get_initial_state() -> State {
        let editor_buffers: HashMap<FlexBoxId, EditorBuffer> = {
            let editor_buffer = {
                let mut editor_buffer =
                    EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
                let iter = get_default_editor_content().into_iter();
                editor_buffer.set_lines(iter);
                editor_buffer
            };
            let mut it = HashMap::new();
            it.insert(FlexBoxId::from(Id::Editor), editor_buffer);
            it
        };

        State {
            editor_buffers,
            dialog_buffers: HashMap::default(),
        }
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
- dialog_buffers_map: {dialog:?}
- editor_buffers_map: {editor:?}
]",
                dialog = self.dialog_buffers,
                editor = self.editor_buffers,
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

            // Add detailed buffer info if needed (with line breaks and indentation)
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

            Ok(())
        }
    }
}

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

use r3bl_tui::{format_as_kilobytes_with_commas, get_real_world_editor_content,
               DialogBuffer, EditorBuffer, FlexBoxId, HasDialogBuffers,
               HasEditorBuffers, DEFAULT_SYN_HI_FILE_EXT};

use crate::ex_editor::Id;

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

mod constructor {
    use super::{constructor, get_real_world_editor_content, EditorBuffer, FlexBoxId,
                HashMap, Id, State, DEFAULT_SYN_HI_FILE_EXT};

    impl Default for State {
        fn default() -> Self { constructor::get_initial_state() }
    }

    pub fn get_initial_state() -> State {
        let editor_buffers: HashMap<FlexBoxId, EditorBuffer> = {
            let editor_buffer = {
                let mut editor_buffer =
                    EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
                let iter = get_real_world_editor_content().iter().copied();
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
    use super::{format_as_kilobytes_with_commas, Display, Formatter, Result, State};

    impl Display for State {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffers and dialog buffers. This is used for telemetry
        /// reporting, and it is expected to be fast, since it is called in a hot loop,
        /// on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Efficient telemetry logging format - no deep traversal.
            let editor_count = self.editor_buffers.len();
            let dialog_count = self.dialog_buffers.len();

            // Calculate total memory size only if caches are available.
            let mut total_cached_size = 0usize;
            let mut uncached_count = 0usize;

            // Sum up cached sizes from editor buffers.
            for buffer in self.editor_buffers.values() {
                if let Some(size) = buffer.get_memory_size_calc_cached() {
                    total_cached_size += size;
                } else {
                    uncached_count += 1;
                }
            }

            // Format the state summary.
            write!(f, "State[editors={editor_count}, dialogs={dialog_count}")?;

            // Add editor buffers info if available. The EditorBuffer's Display impl is
            // fast.
            if !self.editor_buffers.is_empty() {
                write!(f, "\n  editor_buffers=[")?;
                for (i, (id, buffer)) in self.editor_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, "\n    ")?;
                    }
                    write!(f, "{id}:{buffer}")?;
                }
                write!(f, "]")?;
            }

            // Add dialog buffers info if available. The DialogBuffer's Display impl is
            // fast.
            if !self.dialog_buffers.is_empty() {
                write!(f, "\n  dialog_buffers=[")?;
                for (i, (id, buffer)) in self.dialog_buffers.iter().enumerate() {
                    if i > 0 {
                        write!(f, "\n    ")?;
                    }
                    write!(f, "{id}:{buffer}")?;
                }
                write!(f, "]")?;
            }

            // Add memory info if available.
            if uncached_count == 0 && editor_count > 0 {
                let memory_str = format_as_kilobytes_with_commas(total_cached_size);
                write!(f, ", total_size={memory_str}")?;
            }

            write!(f, "]")?;

            Ok(())
        }
    }
}

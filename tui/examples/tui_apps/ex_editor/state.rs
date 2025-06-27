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
          fmt::{Debug, Formatter, Result}};

use r3bl_tui::{get_real_world_editor_content,
               DialogBuffer,
               EditorBuffer,
               FlexBoxId,
               HasDialogBuffers,
               HasEditorBuffers,
               DEFAULT_SYN_HI_FILE_EXT};

use crate::ex_editor::Id;

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

mod constructor {
    use super::*;

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
            dialog_buffers: Default::default(),
        }
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
            write! { f,
"State [
- dialog_buffers_map: {dialog:?}
- editor_buffers_map: {editor:?}
]",
                    dialog = self.dialog_buffers,
                    editor = self.editor_buffers,
            }
        }
    }
}

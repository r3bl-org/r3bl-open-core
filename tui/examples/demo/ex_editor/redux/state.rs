/*
 *   Copyright (c) 2022 R3BL LLC
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

use r3bl_tui::*;

#[derive(Clone, PartialEq, Default)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

impl HasEditorBuffers for State {
    fn get_editor_buffer(&self, id: FlexBoxId) -> Option<&EditorBuffer> {
        if let Some(buffer) = self.editor_buffers.get(&id) {
            Some(buffer)
        } else {
            None
        }
    }
}

impl HasDialogBuffers for State {
    fn get_dialog_buffer(&self, id: FlexBoxId) -> Option<&DialogBuffer> {
        self.dialog_buffers.get(&id)
    }
}

mod debug_format_helpers {
    use super::*;

    fn fmt(this: &State, f: &mut Formatter<'_>) -> Result {
        write! { f,
            "\nState [\n\
            - dialog_buffers:\n{:?}\n\
            - editor_buffers:\n{:?}\n\
            ]",
            this.dialog_buffers,
            this.editor_buffers,
        }
    }

    impl Display for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
    }

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
    }
}

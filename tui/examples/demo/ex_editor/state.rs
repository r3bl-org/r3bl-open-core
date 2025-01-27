/*
 *   Copyright (c) 2023 R3BL LLC
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

use r3bl_tui::{DialogBuffer,
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
                    EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
                let iter = get_default_content().iter().copied();
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

    pub fn get_default_content<'a>() -> &'a [&'a str] {
        &[
"0         1         2         3         4         5         6",
"0123456789012345678901234567890123456789012345678901234567890",
"@title: untitled",
"@tags: foo, bar, baz",
"@authors: xyz, abc",
"@date: 12-12-1234",
"",
"# This approach will not be easy. You are required to fly straight😀",
"## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀",
"",
"1. line 1 of 2",
"2. line 2 of 2",
"",
"This is _not italic [link](https://r3bl.com) not bold* etc.",
"",
"```ts",
"let a=1;",
"```",
"",
"`foo`",
"",
"*bar*",
"**baz**",
"",
"```rs",
"let a=1;",
"```",
"",
"- [x] done",
"- [ ] todo",
"",
"# Random writing from star wars text lorem ipsum generator",
"",
"1. A hyperlink [link](https://forcemipsum.com/)",
"   inline code `code`",
"    2. Did you hear that?",
"       They've shut down the main reactor.",
"       We'll be destroyed for sure.",
"       This is madness!",
"       We're doomed!",
"",
"## Random writing from star trek text lorem ipsum generator",
"",
"- Logic is the beginning of wisdom, not the end. ",
"  A hyperlink [link](https://fungenerators.com/lorem-ipsum/startrek/)",
"  I haven't faced death. I've cheated death. ",
"  - I've tricked my way out of death and patted myself on the back for my ingenuity; ",
"    I know nothing. It's not safe out here. ",
"    - Madness has no purpose. Or reason. But it may have a goal.",
"      Without them to strengthen us, we will weaken and die. ",
"      You remove those obstacles.",
"      - But one man can change the present!  Without freedom of choice there is no creativity. ",
"        I object to intellect without discipline; I object to power without constructive purpose. ",
"        - Live Long and Prosper. To Boldly Go Where No Man Has Gone Before",
"          It’s a — far, far better thing I do than I have ever done before",
"          - A far better resting place I go to than I have ever know",
"            Something Spock was trying to tell me on my birthday",
"",
        ]
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

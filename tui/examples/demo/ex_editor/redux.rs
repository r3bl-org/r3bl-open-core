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
          fmt::{Debug, Display, Formatter, Result}};

use async_trait::async_trait;
use r3bl_redux::*;
use r3bl_tui::{DialogBuffer, *};

// FIXME: clean up action names so they make sense & update reducer & app (state?) to match

pub async fn create_store() -> Store<State, Action> {
    let mut store: Store<State, Action> = Store::default();
    store.add_reducer(Reducer::new()).await;
    store
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// Best practices for naming actions: <https://redux.js.org/style-guide/#write-action-types-as-domaineventname>
pub enum Action {
    Noop,
    UpdateEditorBufferById(FlexBoxId /* id */, EditorBuffer),
    SetDialogBufferTitleAndTextById(
        FlexBoxId, /* id */
        String,    /* title */
        String,    /* text */
    ),
    UpdateDialogBufferById(FlexBoxId /* id */, EditorBuffer),
}

mod action_impl {
    use super::*;

    impl Default for Action {
        fn default() -> Self { Action::Noop }
    }

    impl Display for Action {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
    }
}

#[derive(Default)]
pub struct Reducer;

mod reducer_impl {
    use super::*;

    #[async_trait]
    impl AsyncReducer<State, Action> for Reducer {
        async fn run(&self, action: &Action, state: &State) -> State {
            match action {
                Action::Noop => state.clone(),

                Action::UpdateEditorBufferById(id, buffer) => {
                    let mut new_state = state.clone();
                    new_state.editor_buffers.insert(*id, buffer.clone());
                    new_state
                }

                Action::SetDialogBufferTitleAndTextById(id, title, text) => {
                    let mut new_state = state.clone();

                    let dialog_buffer = {
                        let mut it = DialogBuffer::new_empty();
                        it.title = title.into();
                        it.editor_buffer.set_lines(vec![text.into()]);
                        it
                    };

                    new_state.dialog_buffers.insert(*id, dialog_buffer);

                    new_state
                }

                Action::UpdateDialogBufferById(id, editor_buffer) => {
                    let mut new_state = state.clone();

                    new_state
                        .dialog_buffers
                        .entry(*id)
                        .and_modify(|it| it.editor_buffer = editor_buffer.clone())
                        .or_insert_with(
                            // This code path should never execute, since to update the buffer given an
                            // id, it should have already existed in the first place (created by
                            // SetDialogBufferTitleAndTextById action).
                            || {
                                let mut it = DialogBuffer::new_empty();
                                it.editor_buffer = editor_buffer.clone();
                                it
                            },
                        );

                    new_state
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

mod state_impl {
    use super::*;

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
}

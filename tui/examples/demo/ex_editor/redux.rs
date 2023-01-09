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

pub async fn create_store() -> Store<State, Action> {
    let mut store: Store<State, Action> = Store::default();
    store.add_reducer(Reducer::new()).await;
    store
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
#[non_exhaustive]
/// Best practices for naming actions: <https://redux.js.org/style-guide/#write-action-types-as-domaineventname>
pub enum Action {
    Noop,

    /// Domain: EditorComponent, Event: UpdateContent.
    EditorComponentUpdateContent(FlexBoxId /* id */, EditorBuffer),

    /// Domain: SimpleDialogComponent, Event: InitializeFocused.
    SimpleDialogComponentInitializeFocused(
        FlexBoxId, /* id */
        String,    /* title */
        String,    /* text */
    ),
    /// Domain: SimpleDialogComponent, Event: UpdateContent.
    SimpleDialogComponentUpdateContent(FlexBoxId /* id */, EditorBuffer),

    /// Domain: AutocompleteDialogComponent, Event: InitializeFocused.
    AutocompleteDialogComponentInitializeFocused(
        FlexBoxId, /* id */
        String,    /* title */
        String,    /* text */
    ),
    /// Domain: AutocompleteDialogComponent, Event: UpdateContent.
    AutocompleteDialogComponentUpdateContent(FlexBoxId /* id */, EditorBuffer),

    /// Domain: AutocompleteDialogComponent, Event: SetResults.
    AutocompleteDialogComponentSetResults(FlexBoxId /* id */, Vec<String>),
}

mod action_impl {
    use super::*;

    impl Default for Action {
        fn default() -> Self { Action::Noop }
    }

    impl Display for Action {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
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

                Action::EditorComponentUpdateContent(id, buffer) => {
                    Self::editor_component_update_content(state, id, buffer)
                }

                Action::SimpleDialogComponentInitializeFocused(id, title, text) => {
                    Self::dialog_component_initialize_focused(state, id, title, text)
                }

                Action::SimpleDialogComponentUpdateContent(id, editor_buffer) => {
                    Self::dialog_component_update_content(state, id, editor_buffer)
                }

                Action::AutocompleteDialogComponentInitializeFocused(id, title, text) => {
                    Self::dialog_component_initialize_focused(state, id, title, text)
                }

                Action::AutocompleteDialogComponentUpdateContent(id, editor_buffer) => {
                    Self::dialog_component_update_content(state, id, editor_buffer)
                }

                Action::AutocompleteDialogComponentSetResults(id, results) => {
                    Self::dialog_component_set_results(state, id, results)
                }
            }
        }
    }

    impl Reducer {
        fn dialog_component_initialize_focused(
            state: &State,
            id: &FlexBoxId,
            title: &String,
            text: &String,
        ) -> State {
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

        fn dialog_component_update_content(
            state: &State,
            id: &FlexBoxId,
            editor_buffer: &EditorBuffer,
        ) -> State {
            let mut new_state = state.clone();

            // This is Some only if the content has changed (ignoring caret movements).
            let results_have_changed: Option<Vec<String>> = {
                match new_state.dialog_buffers.get_mut(id) {
                    Some(dialog_buffer)
                        if dialog_buffer.editor_buffer.get_lines() != editor_buffer.get_lines() =>
                    {
                        Some({
                            let editor_buffer_str = editor_buffer.get_as_string();
                            let start_rand_num = rand::random::<u8>() as usize;
                            let max = 10;
                            let mut it = Vec::with_capacity(max);
                            for index in start_rand_num..(start_rand_num + max) {
                                it.push(format!("{editor_buffer_str}{index}"));
                            }
                            it
                        })
                    }
                    _ => None,
                }
            };

            new_state
                .dialog_buffers
                .entry(*id)
                .and_modify(|it| {
                    it.editor_buffer = editor_buffer.clone();
                    if let Some(results) = results_have_changed.clone() {
                        it.maybe_results = Some(results);
                    }
                })
                .or_insert_with(
                    // This code path should never execute, since to update the buffer given an id,
                    // it should have already existed in the first place (created by
                    // SetDialogBufferTitleAndTextById action).
                    || {
                        let mut it = DialogBuffer::new_empty();
                        it.editor_buffer = editor_buffer.clone();
                        if let Some(results) = results_have_changed {
                            it.maybe_results = Some(results);
                        }
                        it
                    },
                );

            // Content is empty.
            if editor_buffer.get_as_string() == "" {
                if let Some(it) = new_state.dialog_buffers.get_mut(id) {
                    it.maybe_results = None;
                }
                return new_state;
            }

            new_state
        }

        fn dialog_component_set_results(
            state: &State,
            id: &FlexBoxId,
            results: &[String],
        ) -> State {
            let mut new_state = state.clone();

            new_state
                .dialog_buffers
                .entry(*id)
                .and_modify(|it| it.maybe_results = Some(results.to_vec()))
                .or_insert_with(
                    // This code path should never execute, since to update the buffer given an id,
                    // it should have already existed in the first place (created by
                    // SetDialogBufferTitleAndTextById action).
                    || {
                        let mut it = DialogBuffer::new_empty();
                        it.maybe_results = Some(results.to_vec());
                        it
                    },
                );

            new_state
        }

        fn editor_component_update_content(
            state: &State,
            id: &FlexBoxId,
            buffer: &EditorBuffer,
        ) -> State {
            let mut new_state = state.clone();
            new_state.editor_buffers.insert(*id, buffer.clone());
            new_state
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

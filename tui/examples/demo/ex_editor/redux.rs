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

#[derive(Default, Clone, Debug)]
#[allow(dead_code)]
#[non_exhaustive]
/// Best practices for naming actions: <https://redux.js.org/style-guide/#write-action-types-as-domaineventname>
pub enum Action {
    #[default]
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

    impl Display for Action {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
    }
}

#[derive(Default)]
pub struct Reducer;

mod reducer_impl {
    use super::*;
    use crate::ex_editor::ComponentId;

    pub fn get_default_content() -> Vec<String> {
        vec![
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
].iter().map(|s| s.to_string()).collect()
    }

    // 00: this is how init state is created
    pub fn get_initial_state() -> State {
        let editor_buffers = {
            let editor_buffer = {
                let mut editor_buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
                editor_buffer.set_lines(get_default_content());
                editor_buffer
            };
            let mut it = HashMap::new();
            it.insert(ComponentId::Editor as u8, editor_buffer);
            it
        };

        State {
            editor_buffers,
            dialog_buffers: Default::default(),
        }
    }

    #[async_trait]
    impl AsyncReducer<State, Action> for Reducer {
        async fn run(&self, action: &Action, state: &mut State) {
            match action {
                Action::Noop => {}

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
            };
        }
    }

    impl Reducer {
        fn dialog_component_initialize_focused(
            state: &mut State,
            id: &FlexBoxId,
            title: &String,
            text: &String,
        ) {
            let dialog_buffer = {
                let mut it = DialogBuffer::new_empty();
                it.title = title.into();
                it.editor_buffer.set_lines(vec![text.into()]);
                it
            };
            state.dialog_buffers.insert(*id, dialog_buffer);
        }

        fn dialog_component_update_content(
            state: &mut State,
            id: &FlexBoxId,
            editor_buffer: &EditorBuffer,
        ) {
            // This is Some only if the content has changed (ignoring caret movements).
            let results_have_changed: Option<Vec<String>> = {
                match state.dialog_buffers.get_mut(id) {
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

            state
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
                if let Some(it) = state.dialog_buffers.get_mut(id) {
                    it.maybe_results = None;
                }
            }
        }

        fn dialog_component_set_results(state: &mut State, id: &FlexBoxId, results: &[String]) {
            state
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
        }

        fn editor_component_update_content(
            state: &mut State,
            id: &FlexBoxId,
            buffer: &EditorBuffer,
        ) {
            state.editor_buffers.insert(*id, buffer.clone());
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
}

mod state_impl {
    use super::*;

    impl Default for State {
        fn default() -> Self { reducer_impl::get_initial_state() }
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
}

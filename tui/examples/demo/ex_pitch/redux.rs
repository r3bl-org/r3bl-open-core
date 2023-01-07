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
use r3bl_tui::*;

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

    /// Domain: SlideControl, Event: NextSlide.
    SlideControlNextSlide,

    /// Domain: SlideControl, Event: PreviousSlide.
    SlideControlPreviousSlide,
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

pub const LINES_ARRAY: [&str; 11] = [
    include_str!("slide1.md"),
    include_str!("slide2.md"),
    include_str!("slide3.md"),
    include_str!("slide4.md"),
    include_str!("slide5.md"),
    include_str!("slide6.md"),
    include_str!("slide7.md"),
    include_str!("slide8.md"),
    include_str!("slide9.md"),
    include_str!("slide10.md"),
    include_str!("slide11.md"),
];

mod reducer_impl {
    use int_enum::IntEnum;

    use super::*;
    use crate::ex_pitch::ComponentId;

    #[async_trait]
    impl AsyncReducer<State, Action> for Reducer {
        async fn run(&self, action: &Action, state: &State) -> State {
            match action {
                Action::Noop => state.clone(),
                Action::EditorComponentUpdateContent(id, buffer) => {
                    Self::editor_component_update_content(state, id, buffer)
                }
                Action::SlideControlNextSlide => Self::next_slide(state),
                Action::SlideControlPreviousSlide => Self::prev_slide(state),
            }
        }
    }

    impl Reducer {
        fn next_slide(state: &State) -> State {
            let mut new_state = state.clone();

            if state.current_slide_index < LINES_ARRAY.len() - 1 {
                new_state.current_slide_index += 1;
                new_state
                    .editor_buffers
                    .entry(ComponentId::Editor.int_value())
                    .and_modify(|it| {
                        it.set_lines(reducer_impl::get_slide_content(
                            new_state.current_slide_index,
                        ));
                    });
            }

            new_state
        }

        fn prev_slide(state: &State) -> State {
            let mut new_state = state.clone();

            if state.current_slide_index > 0 {
                new_state.current_slide_index -= 1;
                new_state
                    .editor_buffers
                    .entry(ComponentId::Editor.int_value())
                    .and_modify(|it| {
                        it.set_lines(reducer_impl::get_slide_content(
                            new_state.current_slide_index,
                        ));
                    });
            }

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

    pub fn get_slide_content(arg: usize) -> Vec<String> {
        let slide_content = LINES_ARRAY[arg];
        let mut it = Vec::new();
        for line in slide_content.lines() {
            it.push(line.to_string());
        }
        it
    }

    pub fn get_initial_state() -> State {
        let editor_buffer = {
            let mut editor_buffer = EditorBuffer::new_empty(DEFAULT_SYN_HI_FILE_EXT.to_string());
            editor_buffer.set_lines(reducer_impl::get_slide_content(0));
            editor_buffer
        };

        let mut editor_buffers = HashMap::new();
        editor_buffers.insert(ComponentId::Editor.int_value(), editor_buffer);

        State {
            editor_buffers,
            current_slide_index: 0,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub current_slide_index: usize,
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

    mod debug_format_helpers {
        use super::*;

        fn fmt(this: &State, f: &mut Formatter<'_>) -> Result {
            write! { f,
                "\nState [\n\
                - current_slide_index:\n{:?}\n\
                - editor_buffers:\n{:?}\n\
                ]",
                this.current_slide_index,
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

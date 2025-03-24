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

use r3bl_tui::{editor_buffer::EditorBuffer,
               ComponentRegistryMap,
               FlexBoxId,
               HasEditorBuffers,
               DEFAULT_SYN_HI_FILE_EXT};

use crate::ex_pitch::Id;

pub const FILE_CONTENT_ARRAY: [&str; 13] = [
    include_str!("slide0.md"),
    include_str!("slide1.md"),
    include_str!("slide2.md"),
    include_str!("slide3.md"),
    include_str!("slide3_1.md"),
    include_str!("slide4.md"),
    include_str!("slide5.md"),
    include_str!("slide6.md"),
    include_str!("slide7.md"),
    include_str!("slide8.md"),
    include_str!("slide9.md"),
    include_str!("slide10.md"),
    include_str!("slide11.md"),
];

#[derive(Clone, PartialEq)]
pub struct State {
    pub editor_buffers: HashMap<FlexBoxId, EditorBuffer>,
    pub current_slide_index: usize,
}

pub mod state_mutator {
    use super::*;

    pub fn reset_editor_engine_ast_cache(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        // Reset the editor component to the current state.
        let id = FlexBoxId::from(Id::Editor);
        if let Some(editor_component) = component_registry_map.get_mut(&id) {
            editor_component.reset();
        }
    }

    pub fn next_slide(
        state: &mut State,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        if state.current_slide_index < FILE_CONTENT_ARRAY.len() - 1 {
            state.current_slide_index += 1;
            state
                .editor_buffers
                .entry(FlexBoxId::from(Id::Editor as u8))
                .and_modify(|it| {
                    it.set_lines(get_slide_content(state.current_slide_index));
                    reset_editor_engine_ast_cache(component_registry_map);
                });
        }
    }

    pub fn prev_slide(
        state: &mut State,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        if state.current_slide_index > 0 {
            state.current_slide_index -= 1;
            state
                .editor_buffers
                .entry(FlexBoxId::from(Id::Editor as u8))
                .and_modify(|it| {
                    it.set_lines(get_slide_content(state.current_slide_index));
                    reset_editor_engine_ast_cache(component_registry_map);
                });
        }
    }

    pub fn get_slide_content<'a>(arg: usize) -> Vec<&'a str> {
        let slide_content = FILE_CONTENT_ARRAY[arg];
        let mut it = vec![];
        for line in slide_content.lines() {
            it.push(line);
        }
        it
    }

    pub fn get_initial_state() -> State {
        let editor_buffer = {
            let mut it = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
            it.set_lines(get_slide_content(0));
            it
        };

        let editor_buffers = {
            let mut it = HashMap::new();
            let id = FlexBoxId::from(Id::Editor);
            it.insert(id, editor_buffer);
            it
        };

        State {
            editor_buffers,
            current_slide_index: 0,
        }
    }
}

mod state_impl {
    use super::*;

    impl Default for State {
        fn default() -> Self { state_mutator::get_initial_state() }
    }

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

#[derive(Default, Clone, Debug)]
#[allow(dead_code)]
#[non_exhaustive]
pub enum AppSignal {
    #[default]
    Noop,
    NextSlide,
    PrevSlide,
}

mod debug_format_helpers {
    use super::*;

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {f,
"State [
  - current_slide_index:\n{:?}
  - editor_buffers:\n{:?}
]",
                    self.current_slide_index,
                    self.editor_buffers,
            }
        }
    }
}

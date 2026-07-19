// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ex_pitch::Id;
use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, CIndex, ComponentRegistryMap,
               DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, FileExtensionToken, FlexBoxId,
               HasEditorBuffers, NumericValue, c_index, c_len};
use rustc_hash::FxHashMap;
use std::fmt::{Debug, Display, Formatter, Result};

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
    pub editor_buffers: FxHashMap<FlexBoxId, EditorBuffer>,
    pub current_slide_index: CIndex,
}

pub mod state_mutator {
    #[allow(clippy::wildcard_imports)]
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
        let total_slides = c_len(FILE_CONTENT_ARRAY.len());
        let next_slide_index: CIndex = state.current_slide_index + 1;
        if next_slide_index.overflows(total_slides) == ArrayOverflowResult::Within {
            state.current_slide_index = next_slide_index;
            state
                .editor_buffers
                .entry(FlexBoxId::from(Id::Editor))
                .and_modify(|it| {
                    it.init_with(get_slide_content(state.current_slide_index));
                    reset_editor_engine_ast_cache(component_registry_map);
                });
        }
    }

    pub fn prev_slide(
        state: &mut State,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        if !state.current_slide_index.is_zero() {
            state.current_slide_index -= 1;
            state
                .editor_buffers
                .entry(FlexBoxId::from(Id::Editor))
                .and_modify(|it| {
                    it.init_with(get_slide_content(state.current_slide_index));
                    reset_editor_engine_ast_cache(component_registry_map);
                });
        }
    }

    pub fn get_slide_content<'a>(arg: impl Into<CIndex>) -> Vec<&'a str> {
        let slide_content = FILE_CONTENT_ARRAY[arg.into().as_usize()];
        let mut it = vec![];
        for line in slide_content.lines() {
            it.push(line);
        }
        it
    }

    pub fn get_initial_state() -> State {
        let editor_buffer = {
            let mut it =
                EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
            it.init_with(get_slide_content(c_index(0)));
            it
        };

        let editor_buffers = {
            let mut it = FxHashMap::default();
            let id = FlexBoxId::from(Id::Editor);
            it.insert(id, editor_buffer);
            it
        };

        State {
            editor_buffers,
            current_slide_index: c_index(0),
        }
    }
}

mod impl_state {
    use super::{EditorBuffer, FlexBoxId, HasEditorBuffers, State, state_mutator};

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

mod debug_format_helper {
    use super::{Debug, Formatter, Result, State};

    impl Debug for State {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(
                f,
                "State [
  - current_slide_index:\n{:?}
  - editor_buffers:\n{:?}
]",
                self.current_slide_index, self.editor_buffers,
            )
        }
    }
}

/// Efficient Display implementation for telemetry logging.
mod impl_display {
    use super::{Display, Formatter, Result, State};
    use r3bl_tui::{IndexOps, c_len, ok};

    impl Display for State {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffers. This is used for telemetry reporting, and it is expected
        /// to be fast, since it is called in a hot loop, on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Efficient telemetry logging format - no deep traversal.
            let editor_count = self.editor_buffers.len();
            let slide_num = self.current_slide_index.convert_to_length();
            let total_slides = c_len(super::FILE_CONTENT_ARRAY.len());

            // Note: We can't calculate total memory size here because Display
            // requires &self not &mut self. The EditorBuffer's Display impl will
            // show memory size for each buffer individually.

            // Format the state summary.
            write!(
                f,
                "State[slide={}/{}, editors={}",
                slide_num.as_usize(),
                total_slides.as_usize(),
                editor_count
            )?;

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

            // Memory info is shown per-buffer in the EditorBuffer's Display impl.

            write!(f, "]")?;

            ok!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_initial_state() {
        let state = state_mutator::get_initial_state();
        assert_eq2!(state.current_slide_index, c_index(0));
        assert_eq2!(state.editor_buffers.len(), 1);
        let id = FlexBoxId::from(Id::Editor);
        assert!(state.editor_buffers.contains_key(&id));
    }

    #[test]
    fn test_get_slide_content() {
        let lines = state_mutator::get_slide_content(c_index(0));
        assert!(!lines.is_empty());
        let last_slide_index = c_index(FILE_CONTENT_ARRAY.len() - 1);
        let last_lines = state_mutator::get_slide_content(last_slide_index);
        assert!(!last_lines.is_empty());
    }

    #[test]
    fn test_next_and_prev_slide_navigation() {
        let mut state = state_mutator::get_initial_state();
        let mut map: ComponentRegistryMap<State, AppSignal> =
            ComponentRegistryMap::default();

        // Already at slide 0, prev_slide should be a no-op.
        state_mutator::prev_slide(&mut state, &mut map);
        assert_eq2!(state.current_slide_index, c_index(0));

        // Advance to slide 1.
        state_mutator::next_slide(&mut state, &mut map);
        assert_eq2!(state.current_slide_index, c_index(1));

        // Step back to slide 0.
        state_mutator::prev_slide(&mut state, &mut map);
        assert_eq2!(state.current_slide_index, c_index(0));

        // Advance to the last slide.
        let total_slides = FILE_CONTENT_ARRAY.len();
        for _ in 0..total_slides + 5 {
            state_mutator::next_slide(&mut state, &mut map);
        }
        assert_eq2!(state.current_slide_index, c_index(total_slides - 1));

        // At last slide, next_slide should not overflow.
        state_mutator::next_slide(&mut state, &mut map);
        assert_eq2!(state.current_slide_index, c_index(total_slides - 1));
    }

    #[test]
    fn test_state_display() {
        let state = state_mutator::get_initial_state();
        let display_str = format!("{state}");
        let total_slides = FILE_CONTENT_ARRAY.len();
        let expected_prefix = format!("State[slide=1/{total_slides}, editors=1");
        assert!(display_str.starts_with(&expected_prefix));
    }
}

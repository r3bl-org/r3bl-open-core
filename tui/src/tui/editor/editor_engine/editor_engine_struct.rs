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

use std::fmt::Debug;

use r3bl_core::Dim;
use syntect::{highlighting::Theme, parsing::SyntaxSet};

use crate::{load_default_theme, try_load_r3bl_theme, PartialFlexBox};

/// Do not create this struct directly. Please use [new()](EditorEngine::new) instead.
///
/// Holds data related to rendering in between render calls.
/// 1. This is not stored in the [crate::EditorBuffer] struct, which lives in the app's
///    state. The state provides the underlying document or buffer struct that holds the
///    actual document.
/// 2. Rather, this struct is stored in the [crate::EditorComponent] struct, which lives
///    inside of the [crate::App].
///
/// In order to change the document, you can use the
/// [EditorEngineApi::apply_event](crate::EditorEngineApi::apply_event) method which takes
/// [crate::InputEvent] and tries to convert it to an [crate::EditorEvent] and then execute them
/// against this buffer.
#[derive(Clone, Debug)]
pub struct EditorEngine {
    /// Set by [EditorEngineApi::render_engine](crate::EditorEngineApi::render_engine).
    pub current_box: PartialFlexBox,
    pub config_options: EditorEngineConfig,
    /// Syntax highlighting support. This is a very heavy object to create, re-use it.
    pub syntax_set: SyntaxSet,
    /// Syntax highlighting support. This is a very heavy object to create, re-use it.
    pub theme: Theme,
}

impl Default for EditorEngine {
    fn default() -> Self { EditorEngine::new(Default::default()) }
}

impl EditorEngine {
    /// Syntax highlighting support - [SyntaxSet] and [Theme] are a very expensive objects to
    /// create, so re-use them.
    pub fn new(config_options: EditorEngineConfig) -> Self {
        Self {
            current_box: Default::default(),
            config_options,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: try_load_r3bl_theme().unwrap_or_else(|_| load_default_theme()),
        }
    }

    pub fn viewport(&self) -> Dim { self.current_box.style_adjusted_bounds_size }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorEngineConfig {
    pub multiline_mode: LineMode,
    pub syntax_highlight: SyntaxHighlightMode,
    pub edit_mode: EditMode,
}

mod editor_engine_config_options_impl {
    use super::*;

    impl Default for EditorEngineConfig {
        fn default() -> Self {
            Self {
                multiline_mode: LineMode::MultiLine,
                syntax_highlight: SyntaxHighlightMode::Enable,
                edit_mode: EditMode::ReadWrite,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineMode {
    SingleLine,
    MultiLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxHighlightMode {
    Disable,
    Enable,
}

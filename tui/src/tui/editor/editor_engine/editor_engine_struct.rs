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

use r3bl_rs_utils_core::*;
use serde::*;
use syntect::{highlighting::Theme, parsing::SyntaxSet};

use crate::*;

/// Do not create this struct directly. Please use [new()](EditorEngine::new) instead.
///
/// Holds data related to rendering in between render calls.
/// 1. This is not stored in the [EditorBuffer] struct, which lives in the [r3bl_redux::Store]. The
///    store provides the underlying document or buffer struct that holds the actual document.
/// 2. Rather, this struct is stored in the [EditorComponent] struct, which lives inside of the
///    [App] and outside of the Redux store. Keep in mind that both the [App] and
///    [r3bl_redux::Store] are passed into the [main_event_loop] function to launch an app.
///
/// In order to change the document, you can use the
/// [EditorEngineApi::apply_event](EditorEngineApi::apply_event) method which takes [InputEvent] and
/// tries to convert it to an [EditorEvent] and then execute them against this buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorEngine {
    /// Set by [EditorEngineApi::render_engine](EditorEngineApi::render_engine).
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

    pub fn viewport_width(&self) -> ChUnit { self.current_box.style_adjusted_bounds_size.col_count }

    pub fn viewport_height(&self) -> ChUnit {
        self.current_box.style_adjusted_bounds_size.row_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
                syntax_highlight: SyntaxHighlightMode::Enable(DEFAULT_SYN_HI_FILE_EXT.to_string()),
                edit_mode: EditMode::ReadWrite,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineMode {
    SingleLine,
    MultiLine,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxHighlightMode {
    Disable,
    /// This is only the default for buffers that are created by the component when not passed in
    /// via state. The String represents the `file_extension_for_new_empty_buffer` which is used as
    /// the argument to [EditorBuffer::new_empty()](EditorBuffer::new_empty()) when creating a new
    /// editor buffer.
    Enable(String),
}

mod syntax_highlight_config_impl {
    use super::*;

    impl SyntaxHighlightMode {
        pub fn get_file_extension_for_new_empty_buffer(&self) -> Option<&str> {
            match self {
                SyntaxHighlightMode::Disable => None,
                SyntaxHighlightMode::Enable(ref ext) => Some(ext.as_str()),
            }
        }
    }
}

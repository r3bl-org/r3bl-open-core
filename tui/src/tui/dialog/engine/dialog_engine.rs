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

use crate::*;

/// Please do not construct this struct directly, and use [new](DialogEngine::new) instead.
///
/// Holds data related to rendering in between render calls. This is not stored in the
/// [DialogBuffer] struct, which lives in the [r3bl_redux::Store]. The store provides the underlying
/// document or buffer struct that holds the actual document.
///
/// In order to change the document, you can use the
/// [apply_event](DialogEngine::apply_event) method which takes [InputEvent] and tries to
/// execute it against this buffer.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DialogEngine {
    pub dialog_options: DialogEngineConfigOptions,
    pub editor_engine: EditorEngine,
    pub lolcat: Lolcat,
    /// This is evaluated and saved when [render_engine]( DialogEngine::render_engine) is called.
    /// The dialog box is rendered outside of any layout [FlexBox] or [Surface], so it just paints
    /// itself to the screen on top of everything else.
    pub maybe_flex_box: Option<(
        /* window size: */ Size,
        /* flex box calculated by render_engine(): */ PartialFlexBox,
    )>,
    pub maybe_surface_bounds: Option<SurfaceBounds>,
    pub selected_row_index: ChUnit,
    pub scroll_offset_row_index: ChUnit,
}

mod dialog_engine_impl {
    use super::*;

    impl DialogEngine {
        pub fn new(
            dialog_options: DialogEngineConfigOptions,
            editor_options: EditorEngineConfigOptions,
        ) -> Self {
            Self {
                dialog_options,
                editor_engine: EditorEngine::new(editor_options),
                ..Default::default()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEngineConfigOptions {
    pub mode: DialogEngineMode,
    pub result_panel_row_count: ChUnit,
    pub maybe_style_border: Option<Style>,
    pub maybe_style_title: Option<Style>,
    pub maybe_style_editor: Option<Style>,
    pub maybe_style_results_panel: Option<Style>,
}

mod dialog_engine_config_options_impl {
    use int_enum::IntEnum;

    use super::*;

    impl Default for DialogEngineConfigOptions {
        fn default() -> Self {
            Self {
                mode: DialogEngineMode::ModalSimple,
                result_panel_row_count: ch!(
                    DisplayConstants::DefaultResultsPanelRowCount.int_value()
                ),
                maybe_style_border: None,
                maybe_style_editor: None,
                maybe_style_title: None,
                maybe_style_results_panel: None,
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogEngineMode {
    ModalSimple,
    ModalAutocomplete,
}

/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use smallvec::smallvec;

use crate::{get_terminal_width_no_default,
            row,
            u8,
            width,
            Ansi256GradientIndex,
            ColorWheel,
            ColorWheelConfig,
            ColorWheelSpeed,
            DisplayConstants,
            EditorEngine,
            EditorEngineConfig,
            PartialFlexBox,
            RowHeight,
            RowIndex,
            Size,
            SurfaceBounds,
            TuiStyle};

/// Please do not construct this struct directly, and use [new](DialogEngine::new)
/// instead.
///
/// Holds data related to rendering in between render calls. This is not stored in the
/// [crate::DialogBuffer] struct, which lives in the app's state. The store provides the
/// underlying document or buffer struct that holds the actual document.
///
/// In order to change the document, you can use the
/// [DialogEngineApi::apply_event](crate::DialogEngineApi::apply_event) method which takes
/// [crate::InputEvent] and tries to execute it against this buffer.
#[derive(Clone, Default, Debug)]
pub struct DialogEngine {
    pub dialog_options: DialogEngineConfigOptions,
    pub editor_engine: EditorEngine,
    /// This [ColorWheel] is used to render the dialog box. It is created when
    /// [new()](DialogEngine::new) is called.
    /// - The colors it cycles through are "stable" meaning that once constructed via the
    ///   [ColorWheel::new()](ColorWheel::new) (which sets the options that determine
    ///   where the color wheel starts when it is used). For eg, between repeated calls
    ///   to [DialogEngineApi::render_engine](crate::DialogEngineApi::render_engine)
    ///   which uses the same [ColorWheel] instance, the generated colors will be the
    ///   same.
    /// - If you want to change where the color wheel "begins", you have to change
    ///   [ColorWheelConfig] options used to create this instance.
    pub color_wheel: ColorWheel,
    /// This is evaluated and saved when
    /// [DialogEngineApi::render_engine](crate::DialogEngineApi::render_engine) is
    /// called. The dialog box is rendered outside of any layout [crate::FlexBox] or
    /// [crate::Surface], so it just paints itself to the screen on top of everything
    /// else.
    pub maybe_flex_box: Option<(
        /* window size: */ Size,
        /* mode: */ DialogEngineMode,
        /* flex box calculated by render_engine(): */ PartialFlexBox,
    )>,
    pub maybe_surface_bounds: Option<SurfaceBounds>,
    pub selected_row_index: RowIndex,
    pub scroll_offset_row_index: RowIndex,
}

impl DialogEngine {
    pub fn new(
        dialog_options: DialogEngineConfigOptions,
        editor_options: EditorEngineConfig,
    ) -> Self {
        // The col_count has to be large enough to fit the terminal width so that the
        // gradient doesn't flicker. If for some reason the terminal width is not
        // available, then we default to 250.
        let width_col_count = *get_terminal_width_no_default().unwrap_or(width(200));

        Self {
            dialog_options,
            editor_engine: EditorEngine::new(editor_options),
            color_wheel: ColorWheel::new(smallvec![
                // Truecolor gradient.
                ColorWheelConfig::Rgb(
                    smallvec::smallvec![
                        "#00ffff".into(), /* cyan */
                        "#ff00ff".into(), /* magenta */
                        "#0000ff".into(), /* blue */
                        "#00ff00".into(), /* green */
                        "#ffff00".into(), /* yellow */
                        "#ff0000".into(), /* red */
                    ],
                    ColorWheelSpeed::Fast,
                    u8(width_col_count + 50),
                ),
                // Ansi256 gradient.
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightGreenToLightBlue,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            ..Default::default()
        }
    }

    /// Clean up any state in the engine, eg: selected_row_index or
    /// scroll_offset_row_index.
    pub fn reset(&mut self) {
        self.selected_row_index = row(0);
        self.scroll_offset_row_index = row(0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct DialogEngineConfigOptions {
    pub mode: DialogEngineMode,
    /// Max height of the results panel.
    pub result_panel_display_row_count: RowHeight,
    pub maybe_style_border: Option<TuiStyle>,
    pub maybe_style_title: Option<TuiStyle>,
    pub maybe_style_editor: Option<TuiStyle>,
    pub maybe_style_results_panel: Option<TuiStyle>,
}

mod dialog_engine_config_options_impl {
    use super::*;
    use crate::height;

    impl Default for DialogEngineConfigOptions {
        fn default() -> Self {
            Self {
                mode: DialogEngineMode::ModalSimple,
                result_panel_display_row_count: height(
                    DisplayConstants::DefaultResultsPanelRowCount as u16,
                ),
                maybe_style_border: None,
                maybe_style_editor: None,
                maybe_style_title: None,
                maybe_style_results_panel: None,
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DialogEngineMode {
    ModalSimple,
    ModalAutocomplete,
}

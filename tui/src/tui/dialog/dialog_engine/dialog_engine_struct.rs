// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{Ansi256GradientIndex, CPos, CRow, CanvasCameraExt, ColorWheel,
            ColorWheelConfig, ColorWheelSpeed, DisplayConstants, EditorEngine,
            EditorEngineConfig, PartialFlexBox, TuiStyle, VPBoundingBox, VPHeight,
            VPSize, Viewport, WideningCastToU16, c_pos, c_row,
            get_terminal_width_no_default, u8, vp_height, vp_width};
use smallvec::smallvec;
use std::fmt::Debug;

/// Please do not construct this struct directly, and use [new] instead.
///
/// Holds data related to rendering in between render calls. This is not stored in the
/// [`DialogBuffer`] struct, which lives in the app's state. The store provides the
/// underlying document or buffer struct that holds the actual document.
///
/// In order to change the document, you can use the [`DialogEngineApi::apply_event`]
/// method which takes [`InputEvent`] and tries to execute it against this buffer.
///
/// [`DialogBuffer`]: crate::DialogBuffer
/// [`DialogEngineApi::apply_event`]: crate::DialogEngineApi::apply_event
/// [`InputEvent`]: crate::InputEvent
/// [new]: DialogEngine::new
#[derive(Default, Debug)]
pub struct DialogEngine {
    pub dialog_options: DialogEngineConfig,

    pub editor_engine: EditorEngine,

    /// This [`ColorWheel`] is used to render the dialog box. It is created when
    /// [`new()`] is called.
    /// - The colors it cycles through are "stable" meaning that once constructed via the
    ///   [`ColorWheel::new()`] (which sets the options that determine where the color
    ///   wheel starts when it is used). For eg, between repeated calls to
    ///   [`DialogEngineApi::render_engine`] which uses the same [`ColorWheel`] instance,
    ///   the generated colors will be the same.
    /// - If you want to change where the color wheel "begins", you have to change
    ///   [`ColorWheelConfig`] options used to create this instance.
    ///
    /// [`ColorWheel::new()`]: ColorWheel::new
    /// [`DialogEngineApi::render_engine`]: crate::DialogEngineApi::render_engine
    /// [`new()`]: DialogEngine::new
    pub color_wheel: ColorWheel,

    /// This is evaluated and saved when
    /// [`DialogEngineApi::render_engine`] is
    /// called. The dialog box is rendered outside of any layout [`crate::FlexBox`] or
    /// [`crate::Surface`], so it just paints itself to the screen on top of everything
    /// else.
    ///
    /// [`DialogEngineApi::render_engine`]: crate::DialogEngineApi::render_engine
    pub maybe_flex_box: Option<(
        /* window size: */ VPSize,
        /* mode: */ DialogEngineMode,
        /* flex box calculated by render_engine(): */ PartialFlexBox,
    )>,

    pub maybe_surface_bounds: Option<VPBoundingBox>,

    pub selected_row_index: CRow,

    pub vp_origin: CPos,
}

impl DialogEngine {
    #[must_use]
    pub fn new(
        dialog_options: DialogEngineConfig,
        editor_options: EditorEngineConfig,
    ) -> Self {
        // The col_count has to be large enough to fit the terminal width so that the
        // gradient doesn't flicker. If for some reason the terminal width is not
        // available, then we default to 250.
        let width_col_count = *get_terminal_width_no_default().unwrap_or(vp_width(200));

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

    /// Clean up any state in the engine, eg: `selected_row_index` or
    /// `vp_origin`.
    pub fn reset(&mut self) {
        self.selected_row_index = c_row(0);
        self.vp_origin = c_pos(0, 0);
    }

    pub fn pan_viewport_to_keep_row_in_view(
        &mut self,
        target_row: CRow,
        vp_height: VPHeight,
    ) {
        self.vp_origin = {
            // We create a synthetic viewport with the current origin and the given
            // height, and then pan it to keep the target row in view. After that, we
            // update the actual viewport origin to match the synthetic viewport's origin.
            let fake_width = vp_width(u16::MAX);
            let real_height = vp_height;
            let real_vp_origin = self.vp_origin;
            let mut synthetic_viewport: Viewport =
                (real_vp_origin, fake_width + real_height).into();
            // We only care about the row index, and we ignore the col index.
            synthetic_viewport.pan_to_keep_coord_in_view(target_row);
            synthetic_viewport.get_origin_pos()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct DialogEngineConfig {
    pub mode: DialogEngineMode,
    /// Max height of the results panel.
    pub result_panel_display_row_count: VPHeight,
    pub maybe_style_border: Option<TuiStyle>,
    pub maybe_style_title: Option<TuiStyle>,
    pub maybe_style_editor: Option<TuiStyle>,
    pub maybe_style_results_panel: Option<TuiStyle>,
}

mod impl_dialog_engine_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for DialogEngineConfig {
        fn default() -> Self {
            Self {
                mode: DialogEngineMode::ModalSimple,
                result_panel_display_row_count: vp_height(
                    DisplayConstants::DefaultResultsPanelRowCount.as_u16_widening(),
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

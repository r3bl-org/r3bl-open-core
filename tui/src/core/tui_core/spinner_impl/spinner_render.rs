/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
use std::ops::Not;

use crossterm::style::{self, Stylize};

use crate::{contains_ansi_escape_sequence,
            convert_from_tui_color_to_crossterm_color,
            inline_string,
            spinner_render::style::style,
            width,
            ColWidth,
            GCStringExt,
            InlineString,
            SpinnerColor,
            SpinnerStyle,
            SpinnerTemplate,
            BLOCK_DOTS,
            BRAILLE_DOTS};

pub fn render_tick(
    style: &mut SpinnerStyle,
    message: &str,
    count: usize,
    display_width: ColWidth,
) -> InlineString {
    debug_assert!(contains_ansi_escape_sequence(message).not());

    match style.template {
        SpinnerTemplate::Braille => {
            // Translate count into the index of the BRAILLE_DOTS array.
            let output_symbol = get_next_tick_glyph(style, count);
            let output_symbol = apply_color(&output_symbol, &mut style.color);

            let text = message.grapheme_string();
            let text_trunc = text.trunc_end_to_fit(
                display_width -
                width(3) /* 1 for symbol, 1 for space, 1 empty for last display col */
            );
            let text_trunc_fmt = apply_color(text_trunc, &mut style.color);

            inline_string!("{output_symbol} {text_trunc_fmt}")
        }
        SpinnerTemplate::Block => {
            // Translate count into the index of the BLOCK_DOTS array.
            let output_symbol = get_next_tick_glyph(style, count);
            let output_symbol = apply_color(&output_symbol, &mut style.color);

            let text = message.grapheme_string();
            let text_trunc = text.trunc_end_to_fit(
                display_width -
                width(3) /* 1 for symbol, 1 for space, 1 empty for last display col */
            );
            let text_trunc_fmt = apply_color(text_trunc, &mut style.color);

            inline_string!("{output_symbol} {text_trunc_fmt}")
        }
    }
}

pub fn render_final_tick(
    style: &SpinnerStyle,
    final_message: &str,
    display_width: ColWidth,
) -> InlineString {
    let text = final_message.grapheme_string();
    let text_trunc = text.trunc_end_to_fit(display_width);
    match style.template {
        SpinnerTemplate::Braille => text_trunc.into(),
        SpinnerTemplate::Block => text_trunc.into(),
    }
}

pub fn get_next_tick_glyph(style: &SpinnerStyle, count: usize) -> InlineString {
    match style.template {
        SpinnerTemplate::Braille => {
            let index_to_use = count % BRAILLE_DOTS.len();
            BRAILLE_DOTS[index_to_use].into()
        }
        SpinnerTemplate::Block => {
            let index_to_use = count % BLOCK_DOTS.len();
            BLOCK_DOTS[index_to_use].into()
        }
    }
}
fn apply_color(output: &str, color: &mut SpinnerColor) -> InlineString {
    if let SpinnerColor::ColorWheel(color_wheel) = color {
        let maybe_next_color = color_wheel.next_color();
        if let Some(next_color) = maybe_next_color {
            let color = convert_from_tui_color_to_crossterm_color(next_color);
            let styled_content = style(output).with(color);
            return inline_string!("{styled_content}");
        }
    }
    InlineString::from(output)
}

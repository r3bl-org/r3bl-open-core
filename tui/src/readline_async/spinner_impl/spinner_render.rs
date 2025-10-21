// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::ops::Not;

use crate::{contains_ansi_escape_sequence, fg_color, GCStringOwned, inline_string, width,
            ColWidth, InlineString, SpinnerColor, SpinnerStyle, SpinnerTemplate, BLOCK_DOTS,
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

            let text = GCStringOwned::from(message);
            let text_trunc = text.trunc_end_to_fit(
                display_width - width(3), /* 1 for symbol, 1 for space, 1 empty for
                                           * last display col */
            );
            let text_trunc_fmt = apply_color(text_trunc, &mut style.color);

            inline_string!("{output_symbol} {text_trunc_fmt}")
        }
        SpinnerTemplate::Block => {
            // Translate count into the index of the BLOCK_DOTS array.
            let output_symbol = get_next_tick_glyph(style, count);
            let output_symbol = apply_color(&output_symbol, &mut style.color);

            let text = GCStringOwned::from(message);
            let text_trunc = text.trunc_end_to_fit(
                display_width - width(3), /* 1 for symbol, 1 for space, 1 empty for
                                           * last display col */
            );
            let text_trunc_fmt = apply_color(text_trunc, &mut style.color);

            inline_string!("{output_symbol} {text_trunc_fmt}")
        }
    }
}

#[must_use]
pub fn render_final_tick(
    style: &SpinnerStyle,
    final_message: &str,
    display_width: ColWidth,
) -> InlineString {
    let text = GCStringOwned::from(final_message);
    let text_trunc = text.trunc_end_to_fit(display_width);
    match style.template {
        SpinnerTemplate::Braille | SpinnerTemplate::Block => text_trunc.into(),
    }
}

#[must_use]
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
    if let SpinnerColor::ColorWheel(color_wheel) = color
        && let Some(tui_color) = color_wheel.next_color() {
            // Use CliText to apply the color
            let styled_text = fg_color(tui_color, output);
            return inline_string!("{styled_text}");
        }
    InlineString::from(output)
}

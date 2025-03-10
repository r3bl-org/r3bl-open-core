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

use crossterm::{cursor::{MoveToColumn, MoveUp},
                style::{self, Print, Stylize},
                terminal::{Clear, ClearType},
                QueueableCommand};
use miette::IntoDiagnostic as _;
use r3bl_core::{pad_fmt,
                string_storage,
                width,
                ColWidth,
                GCStringExt,
                SendRawTerminal,
                StringStorage,
                ELLIPSIS_GLYPH};

use crate::{convert_from_tui_color_to_crossterm_color,
            spinner_render::style::style,
            SpinnerColor,
            SpinnerStyle,
            SpinnerTemplate,
            BLOCK_DOTS,
            BRAILLE_DOTS};

pub fn get_next_tick_glyph(style: &SpinnerStyle, count: usize) -> StringStorage {
    match style.template {
        SpinnerTemplate::Braille => {
            let index_to_use = count % BRAILLE_DOTS.len();
            BRAILLE_DOTS[index_to_use].into()
        }
        SpinnerTemplate::Block => {
            let index_to_use = count % BLOCK_DOTS.len();
            BLOCK_DOTS[index_to_use].into()
        }
        SpinnerTemplate::Dots => {
            let mut acc = StringStorage::with_capacity(count);
            pad_fmt!(fmt: acc, pad_str: ELLIPSIS_GLYPH, repeat_count: count);
            acc
        }
    }
}

pub fn render_tick(
    style: &mut SpinnerStyle,
    message: &str,
    count: usize,
    display_width: ColWidth,
) -> StringStorage {
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

            string_storage!("{output_symbol} {text_trunc_fmt}")
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

            string_storage!("{output_symbol} {text_trunc_fmt}")
        }
        SpinnerTemplate::Dots => {
            let padding_right = get_next_tick_glyph(style, count);

            let text = message.grapheme_string();
            let text_trunc = text.trunc_end_to_fit({
                display_width - width(padding_right.len()) -
                /* last display col is empty */ width(1)
            });
            let text_trunc_with_padding = string_storage!("{text_trunc}{padding_right}");

            apply_color(&text_trunc_with_padding, &mut style.color)
        }
    }
}

pub fn print_tick(
    style: &SpinnerStyle,
    output: &str,
    writer: &mut SendRawTerminal,
) -> miette::Result<()> {
    match style.template {
        SpinnerTemplate::Dots => {
            // Print the output. And make sure to terminate w/ a newline, so that the
            // output is printed.
            writer
                .queue(MoveToColumn(0))
                .into_diagnostic()?
                .queue(Print(format!("{}\n", output)))
                .into_diagnostic()?
                .queue(MoveUp(1))
                .into_diagnostic()?;
        }

        SpinnerTemplate::Braille => {
            // Print the output. And make sure to terminate w/ a newline, so that the
            // output is printed.
            writer
                .queue(MoveToColumn(0))
                .into_diagnostic()?
                .queue(Clear(ClearType::CurrentLine))
                .into_diagnostic()?
                .queue(Print(format!("{}\n", output)))
                .into_diagnostic()?
                .queue(MoveUp(1))
                .into_diagnostic()?;
        }

        SpinnerTemplate::Block => {
            // Print the output. And make sure to terminate w/ a newline, so that the
            // output is printed.
            writer
                .queue(MoveToColumn(0))
                .into_diagnostic()?
                .queue(Clear(ClearType::CurrentLine))
                .into_diagnostic()?
                .queue(Print(format!("{}\n", output)))
                .into_diagnostic()?
                .queue(MoveUp(1))
                .into_diagnostic()?;
        }
    }

    writer.flush().into_diagnostic()?;

    Ok(())
}

pub fn render_final_tick(
    style: &SpinnerStyle,
    final_message: &str,
    display_width: ColWidth,
) -> StringStorage {
    let text = final_message.grapheme_string();
    let text_trunc = text.trunc_end_to_fit(display_width);
    match style.template {
        SpinnerTemplate::Dots => text_trunc.into(),
        SpinnerTemplate::Braille => text_trunc.into(),
        SpinnerTemplate::Block => text_trunc.into(),
    }
}

pub fn print_final_tick(
    style: &SpinnerStyle,
    output: &str,
    writer: &mut SendRawTerminal,
) -> miette::Result<()> {
    match style.template {
        SpinnerTemplate::Dots | SpinnerTemplate::Braille | SpinnerTemplate::Block => {
            writer
                .queue(MoveToColumn(0))
                .into_diagnostic()?
                .queue(Print(Clear(ClearType::CurrentLine)))
                .into_diagnostic()?
                .queue(Print(format!("{}\n", output)))
                .into_diagnostic()?
        }
    };

    writer.flush().into_diagnostic()?;

    Ok(())
}

fn apply_color(output: &str, color: &mut SpinnerColor) -> StringStorage {
    if let SpinnerColor::ColorWheel(color_wheel) = color {
        let maybe_next_color = color_wheel.next_color();
        if let Some(next_color) = maybe_next_color {
            let color = convert_from_tui_color_to_crossterm_color(next_color);
            let styled_content = style(output).with(color);
            return string_storage!("{styled_content}");
        }
    }
    StringStorage::from(output)
}

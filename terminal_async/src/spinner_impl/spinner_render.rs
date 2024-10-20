/*
 *   Copyright (c) 2024 R3BL LLC
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
use r3bl_core::ch;
use r3bl_tui::convert_from_tui_color_to_crossterm_color;
use r3bl_tuify::clip_string_to_width_with_ellipsis;

use crate::{spinner_render::style::style,
            SendRawTerminal,
            SpinnerColor,
            SpinnerStyle,
            SpinnerTemplate,
            BLOCK_DOTS,
            BRAILLE_DOTS};

pub fn render_tick(
    style: &mut SpinnerStyle,
    message: &str,
    count: usize,
    display_width: usize,
) -> String {
    match style.template {
        SpinnerTemplate::Dots => {
            let padding_right = ".".repeat(count);
            let clipped_message = clip_string_to_width_with_ellipsis(
                message.to_string(),
                ch!(display_width) - ch!(padding_right.len()),
            );
            let output_message = format!("{clipped_message}{padding_right}");
            let clipped_message =
                clip_string_to_width_with_ellipsis(output_message, ch!(display_width));
            apply_color(clipped_message.as_str(), &mut style.color)
        }
        SpinnerTemplate::Braille => {
            // Translate count into the index of the BRAILLE_DOTS array.
            let index_to_use = count % BRAILLE_DOTS.len();
            let output_symbol = BRAILLE_DOTS[index_to_use];
            let output_symbol = apply_color(output_symbol, &mut style.color);
            let clipped_message = clip_string_to_width_with_ellipsis(
                message.to_string(),
                ch!(display_width) - ch!(2),
            );
            let clipped_message = apply_color(&clipped_message, &mut style.color);
            format!("{output_symbol} {clipped_message}")
        }
        SpinnerTemplate::Block => {
            // Translate count into the index of the BLOCK_DOTS array.
            let index_to_use = count % BLOCK_DOTS.len();
            let output_symbol = BLOCK_DOTS[index_to_use];
            let output_symbol = apply_color(output_symbol, &mut style.color);
            let clipped_message = clip_string_to_width_with_ellipsis(
                message.to_string(),
                ch!(display_width) - ch!(2),
            );
            let clipped_message = apply_color(&clipped_message, &mut style.color);
            format!("{output_symbol} {clipped_message}")
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
    display_width: usize,
) -> String {
    let clipped_final_message =
        clip_string_to_width_with_ellipsis(final_message.to_string(), ch!(display_width));
    match style.template {
        SpinnerTemplate::Dots => clipped_final_message.to_string(),
        SpinnerTemplate::Braille => clipped_final_message.to_string(),
        SpinnerTemplate::Block => clipped_final_message.to_string(),
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

fn apply_color(output: &str, color: &mut SpinnerColor) -> String {
    let mut return_it = output.to_string();
    if let SpinnerColor::ColorWheel(ref mut color_wheel) = color {
        let maybe_next_color = color_wheel.next_color();
        if let Some(next_color) = maybe_next_color {
            let color = convert_from_tui_color_to_crossterm_color(next_color);
            let styled_content = style(output).with(color);
            return_it = styled_content.to_string()
        }
    }
    return_it
}

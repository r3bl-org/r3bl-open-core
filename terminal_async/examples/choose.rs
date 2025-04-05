/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::io::Write as _;

use r3bl_core::{InputDevice,
                ItemsBorrowed,
                OutputDevice,
                fg_rgb_color,
                ok,
                rgb_value,
                try_initialize_logging_global};
use r3bl_terminal_async::{Header, HowToChoose, ReadlineAsync, StyleSheet, choose_async};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> miette::Result<()> {
    // Initialize tracing w/ file writer.
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();

    without_readline_async().await?;
    with_readline_async().await?;

    ok!()
}

async fn without_readline_async() -> miette::Result<()> {
    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    let chosen = choose_async(
        Header::SingleLine("Choose one:".into()),
        ItemsBorrowed(&["one", "two", "three"]).into(),
        None,
        None,
        HowToChoose::Single,
        StyleSheet::sea_foam_style(),
        (&mut output_device, &mut input_device, None),
    )
    .await;

    let message = format!(
        ">>> Chosen {:<25}: {:?}",
        "(without readline_async)", chosen
    );
    ReadlineAsync::print_exit_message(&message)?;

    ok!()
}

async fn with_readline_async() -> miette::Result<()> {
    // If the terminal is not fully interactive, then return early.
    let Some(mut readline_async) = ReadlineAsync::try_new({
        // Generate prompt.
        let fg = rgb_value!(slate_grey);
        let bg = rgb_value!(moonlight_blue);
        let prompt_seg_1 = fg_rgb_color(fg, "╭>╮").bg_rgb_color(bg);
        let prompt_seg_2 = " ";
        Some(format!("{}{}", prompt_seg_1, prompt_seg_2))
    })?
    else {
        return ok!();
    };

    let mut sw_1 = readline_async.clone_shared_writer();
    let sw_2 = readline_async.clone_shared_writer();
    let mut output_device = readline_async.clone_output_device();
    let input_device = readline_async.mut_input_device();

    // Start a task to write some output to the shared writer. This output should be
    // paused (as long as choose() is active).
    tokio::spawn({
        tracing::debug!(">>> Starting task to write to shared writer");
        async move {
            for i in 0..5 {
                _ = writeln!(sw_1, ">>> {i}");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    let chosen = choose_async(
        Header::SingleLine("Choose one:".into()),
        ItemsBorrowed(&["one", "two", "three"]).into(),
        None,
        None,
        HowToChoose::Single,
        StyleSheet::hot_pink_style(),
        (&mut output_device, input_device, Some(sw_2)),
    )
    .await;

    // The output to the shared writer should be visible now. Kill the task that was
    // writing to the shared writer.

    let message = format!(">>> Chosen {:<25}: {:?}", "(with readline_async)", chosen);
    ReadlineAsync::print_exit_message(&message)?;

    // Pause for a moment to let the output flush.
    ReadlineAsync::pause_for_output_to_flush().await;

    ok!()
}

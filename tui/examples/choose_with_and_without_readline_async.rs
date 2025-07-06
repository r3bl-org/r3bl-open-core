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

use r3bl_tui::{fg_slate_gray, ok,
               readline_async::{choose, Header, HowToChoose, ReadlineAsyncContext,
                                StyleSheet},
               set_jemalloc_in_main, try_initialize_logging_global, InputDevice,
               OutputDevice};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> miette::Result<()> {
    set_jemalloc_in_main!();

    // Initialize tracing w/ file writer.
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();

    without_readline_async().await?;
    with_readline_async().await?;

    ok!()
}

async fn without_readline_async() -> miette::Result<()> {
    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    let chosen = choose(
        Header::SingleLine("Choose one:".into()),
        &["one", "two", "three"],
        None,
        None,
        HowToChoose::Single,
        StyleSheet::sea_foam_style(),
        (&mut output_device, &mut input_device, None),
    )
    .await;

    println!(
        ">>> Chosen {:<25}: {:?}",
        "(without readline_async)", chosen
    );

    ok!()
}

async fn with_readline_async() -> miette::Result<()> {
    // If the terminal is not fully interactive, then return early.
    let Some(mut rl_ctx) = ReadlineAsyncContext::try_new({
        // Generate prompt.
        let prompt_seg_1 = fg_slate_gray("╭>╮").bg_moonlight_blue();
        let prompt_seg_2 = " ";
        Some(format!("{prompt_seg_1}{prompt_seg_2}"))
    })
    .await?
    else {
        return ok!();
    };

    let mut sw_1 = rl_ctx.clone_shared_writer();
    let sw_2 = rl_ctx.clone_shared_writer();
    let mut output_device = rl_ctx.clone_output_device();
    let input_device = rl_ctx.mut_input_device();

    // Start a task to write some output to the shared writer. This output should be
    // paused (as long as choose() is active).
    tokio::spawn({
        async move {
            // Wait a moment to write to the shared writer. Give the main thread a chance
            // to start the choose() task, which will pause the shared writer output.
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            tracing::debug!(">>> Starting task to write to shared writer");
            for i in 0..5 {
                // We don't care about the result of this operation.
                writeln!(sw_1, ">>> {i}").ok();
            }
        }
    });

    let chosen = choose(
        Header::SingleLine("Choose one:".into()),
        &["one", "two", "three"],
        None,
        None,
        HowToChoose::Single,
        StyleSheet::hot_pink_style(),
        (&mut output_device, input_device, Some(sw_2)),
    )
    .await;

    let message = format!(">>> Chosen {:<25}: {:?}", "(with readline_async)", chosen);
    rl_ctx.request_shutdown(Some(message.as_str())).await?;
    rl_ctx.await_shutdown().await;

    // The output to the shared writer should be visible now.

    ok!()
}

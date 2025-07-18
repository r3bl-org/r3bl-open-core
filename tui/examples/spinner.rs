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

use std::{io::Write, time::Duration};

use r3bl_tui::{CommonResult, OutputDevice, SpinnerColor, SpinnerStyle, SpinnerTemplate,
               readline_async::{ReadlineAsyncContext, Spinner},
               set_mimalloc_in_main,
               spinner_constants::{ARTIFICIAL_UI_DELAY, DELAY_MS, DELAY_UNIT},
               underline};
use tokio::{spawn,
            task::JoinError,
            time::{Instant, interval, sleep}};

macro_rules! println_with_flush {
    ($($tt:tt)*) => {
        println!($($tt)*);
        std::io::stdout().flush().unwrap();
    };
}

#[tokio::main]
#[allow(clippy::needless_return)]
pub async fn main() -> CommonResult<()> {
    set_mimalloc_in_main!();

    // Without readline.
    {
        println_with_flush!("{}", underline("❌ WITHOUT READLINE ASYNC").bold());

        println_with_flush!(
            "-------------> Example with concurrent output: Braille <-------------"
        );
        example_with_concurrent_output_no_readline_async(SpinnerStyle {
            template: SpinnerTemplate::Braille,
            color: SpinnerColor::default_color_wheel(),
        })
        .await?;

        println_with_flush!(
            "-------------> Example with concurrent output: Block <-------------"
        );
        example_with_concurrent_output_no_readline_async(SpinnerStyle {
            template: SpinnerTemplate::Block,
            color: SpinnerColor::default_color_wheel(),
        })
        .await?;
    }

    // With readline async.
    {
        println_with_flush!("{}", underline("✅ WITH READLINE ASYNC").bold());

        println_with_flush!(
            "-------------> Example with concurrent output: Braille <-------------"
        );
        example_with_concurrent_output(SpinnerStyle {
            template: SpinnerTemplate::Braille,
            color: SpinnerColor::default_color_wheel(),
        })
        .await?;

        println_with_flush!(
            "-------------> Example with concurrent output: Block <-------------"
        );
        example_with_concurrent_output(SpinnerStyle {
            template: SpinnerTemplate::Block,
            color: SpinnerColor::default_color_wheel(),
        })
        .await?;
    }

    Ok(())
}

#[allow(unused_assignments)]
async fn example_with_concurrent_output(style: SpinnerStyle) -> miette::Result<()> {
    let maybe_rl_ctx = ReadlineAsyncContext::try_new(Some("$ ")).await?;
    let rl_ctx = maybe_rl_ctx.expect("terminal is not fully interactive");
    let address = "127.0.0.1:8000";
    let message_trying_to_connect = format!(
        "This is a sample indeterminate progress message: trying to connect to server on {}",
        &address
    );

    let mut shared_writer = rl_ctx.clone_shared_writer();

    // Start spinner. Automatically pauses the terminal.
    let mut maybe_spinner = Spinner::try_start(
        message_trying_to_connect,
        "Sample FINAL message for the spinner: Connected to server",
        DELAY_UNIT,
        style,
        OutputDevice::default(),
        Some(shared_writer.clone()),
    )
    .await?;

    // Start another task to simulate some async work that uses an interval to display
    // output for a fixed amount of time, using the shared writer.
    // Wait for the spawned task to complete.
    let _unused: Result<(), JoinError> = spawn(async move {
        // To calculate the delay.
        let duration = ARTIFICIAL_UI_DELAY;
        let start = Instant::now();

        // Dropping the interval cancels it.
        let mut interval = interval(Duration::from_millis(DELAY_MS * 5));

        loop {
            interval.tick().await;
            let elapsed = start.elapsed();
            if elapsed >= duration {
                break;
            }
            // We don't care about the result of this operation.
            writeln!(shared_writer, "⏳foo").ok();
        }
    })
    .await;

    // Stop spinner. Automatically resumes the terminal.
    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    sleep(ARTIFICIAL_UI_DELAY).await;

    Ok(())
}

#[allow(unused_assignments)]
async fn example_with_concurrent_output_no_readline_async(
    style: SpinnerStyle,
) -> miette::Result<()> {
    let address = "127.0.0.1:8000";
    let message_trying_to_connect = format!(
        "This is a sample indeterminate progress message: trying to connect to server on {}",
        &address
    );

    // Start spinner.
    let mut maybe_spinner = Spinner::try_start(
        message_trying_to_connect,
        "Sample FINAL message for the spinner: Connected to server",
        DELAY_UNIT,
        style,
        OutputDevice::default(),
        None,
    )
    .await?;

    // Simulate some async work.
    sleep(ARTIFICIAL_UI_DELAY).await;

    // Stop spinner.
    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    sleep(ARTIFICIAL_UI_DELAY).await;

    Ok(())
}

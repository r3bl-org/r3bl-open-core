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

use r3bl_terminal_async::{
    Spinner, SpinnerColor, SpinnerStyle, SpinnerTemplate, StdMutex, TerminalAsync,
    ARTIFICIAL_UI_DELAY, DELAY_MS, DELAY_UNIT,
};
use std::{io::stderr, time::Duration};
use std::{io::Write, sync::Arc};
use tokio::{time::Instant, try_join};

#[tokio::main]
pub async fn main() -> miette::Result<()> {
    println!("-------------> Example with concurrent output: Braille <-------------");
    example_with_concurrent_output(SpinnerStyle {
        template: SpinnerTemplate::Braille,
        color: SpinnerColor::default_color_wheel(),
    })
    .await?;

    println!("-------------> Example with concurrent output: Block <-------------");
    example_with_concurrent_output(SpinnerStyle {
        template: SpinnerTemplate::Block,
        color: SpinnerColor::default_color_wheel(),
    })
    .await?;

    println!("-------------> Example with concurrent output: Dots <-------------");
    example_with_concurrent_output(SpinnerStyle {
        template: SpinnerTemplate::Dots,
        color: SpinnerColor::default_color_wheel(),
    })
    .await?;

    Ok(())
}

#[allow(unused_assignments)]
async fn example_with_concurrent_output(style: SpinnerStyle) -> miette::Result<()> {
    let terminal_async = TerminalAsync::try_new("$ ").await?;
    let terminal_async = terminal_async.expect("terminal is not fully interactive");
    let address = "127.0.0.1:8000";
    let message_trying_to_connect = format!(
        "This is a sample indeterminate progress message: trying to connect to server on {}",
        &address
    );

    let mut shared_writer = terminal_async.clone_shared_writer();

    // Start spinner. Automatically pauses the terminal.
    let mut maybe_spinner = Spinner::try_start(
        message_trying_to_connect.clone(),
        DELAY_UNIT,
        style,
        Arc::new(StdMutex::new(stderr())),
        shared_writer.clone(),
    )
    .await?;

    // Start another task, to simulate some async work, that uses a interval to display
    // output, for a fixed amount of time, using `terminal_async.println_prefixed()`.
    let _ = try_join!(tokio::spawn(async move {
        // To calculate the delay.
        let duration = ARTIFICIAL_UI_DELAY;
        let start = Instant::now();

        // Dropping the interval cancels it.
        let mut interval = tokio::time::interval(Duration::from_millis(DELAY_MS * 5));

        loop {
            interval.tick().await;
            let elapsed = start.elapsed();
            if elapsed >= duration {
                break;
            }
            let _ = writeln!(shared_writer, "⏳foo");
        }
    }));

    // Stop spinner. Automatically resumes the terminal.
    if let Some(spinner) = maybe_spinner.as_mut() {
        spinner
            .stop("This is a sample final message for the spinner component: Connected to server")
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{io::Write, time::Duration};

use r3bl_tui::{CommonResult, OutputDevice, SpinnerColor, SpinnerStyle, SpinnerTemplate,
               readline_async::{ReadlineAsyncContext, SafeInlineString, Spinner},
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

        println_with_flush!(
            "-------------> Example with message updates: Braille <-------------"
        );
        example_with_message_updates(SpinnerStyle {
            template: SpinnerTemplate::Braille,
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

/// Example showing how to update spinner messages dynamically.
/// This demonstrates the new `update_message()` functionality.
async fn example_with_message_updates(style: SpinnerStyle) -> miette::Result<()> {
    let maybe_rl_ctx = ReadlineAsyncContext::try_new(Some("$ ")).await?;
    let rl_ctx = maybe_rl_ctx.expect("terminal is not fully interactive");

    let shared_writer = rl_ctx.clone_shared_writer();

    // Start spinner with initial message
    let mut maybe_spinner = Spinner::try_start(
        "Starting installation...",
        "Installation complete!",
        DELAY_UNIT,
        style,
        OutputDevice::default(),
        Some(shared_writer.clone()),
    )
    .await?;

    if let Some(ref spinner) = maybe_spinner {
        // Simulate different phases of work with updated messages
        let phases = [
            ("Downloading packages...", 1000),
            ("Verifying checksums...", 800),
            ("Installing dependencies...", 1200),
            ("Configuring package...", 600),
            ("Finalizing installation...", 400),
        ];

        for (message, delay_ms) in phases {
            sleep(Duration::from_millis(delay_ms)).await;
            spinner.update_message(message);
        }

        // Demonstrate direct access to SafeInlineString field
        // (alternative to using update_message() method)
        sleep(Duration::from_millis(500)).await;
        let safe_message: &SafeInlineString = &spinner.interval_message;
        *safe_message.lock().unwrap() =
            "Direct field access via SafeInlineString!".into();

        sleep(Duration::from_millis(800)).await;

        // Test ANSI code stripping
        spinner.update_message("\x1b[31mCleaning up (ANSI codes stripped)...\x1b[0m");

        sleep(Duration::from_millis(800)).await;
    }

    // Stop spinner
    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    sleep(ARTIFICIAL_UI_DELAY).await;

    Ok(())
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Multi-line async output example for readline.
//!
//! Demonstrates proper rendering of multiple log lines via [`SharedWriter`] in a
//! readline REPL loop. Each line should:
//! 1. Start at column 1 (proper carriage return after line feed)
//! 2. Not create extra blank lines before the prompt
//!
//! This example validates the fix for issue #442:
//! <https://github.com/r3bl-org/r3bl-open-core/issues/442>
//!
//! Run with: `cargo run -p r3bl_tui --example readline_async_multiline`
//!
//! ## Expected behavior
//!
//! ```text
//! banner
//! line 1
//! line 2
//! >
//! ```
//!
//! ## Bug behavior (before fix)
//!
//! Lines would either:
//! - Not start at column 1 (missing carriage return)
//! - Have extra blank line before prompt
//!
//! [`SharedWriter`]: r3bl_tui::SharedWriter

use miette::IntoDiagnostic;
use r3bl_tui::{readline_async::{ReadlineAsyncContext, ReadlineEvent}, rla_println};
use std::io::Write;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let maybe_rl_ctx = ReadlineAsyncContext::try_new(Some("> "), None).await?;

    let Some(mut rl_ctx) = maybe_rl_ctx else {
        println!("Not an interactive terminal.");
        return Ok(());
    };

    // Get the shared writer for logging.
    let mut shared_writer = rl_ctx.clone_shared_writer();

    // Print banner (simulates user code printing before REPL loop).
    rla_println!(rl_ctx, "banner");

    loop {
        // Simulate async log output (this is where the bug manifests).
        writeln!(shared_writer, "line 1").into_diagnostic()?;
        writeln!(shared_writer, "line 2").into_diagnostic()?;

        let event = rl_ctx.read_line().await?;

        match event {
            ReadlineEvent::Line(line) => {
                let trimmed = line.trim();

                if trimmed.eq_ignore_ascii_case("exit") {
                    rla_println!(rl_ctx, "Exiting...");
                    break;
                }

                rla_println!(rl_ctx, "You entered: {}", trimmed);

                // Log more lines after input to test the fix persists.
                writeln!(shared_writer, "After input: line A").into_diagnostic()?;
                writeln!(shared_writer, "After input: line B").into_diagnostic()?;
            }

            ReadlineEvent::Resized => {
                writeln!(shared_writer, "Terminal resized").into_diagnostic()?;
            }

            ReadlineEvent::Eof | ReadlineEvent::Interrupted => break,
        }
    }

    rl_ctx
        .request_shutdown(Some("Shutting down test..."))
        .await?;
    rl_ctx.await_shutdown().await;

    Ok(())
}

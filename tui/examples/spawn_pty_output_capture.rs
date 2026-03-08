// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Binary for capturing and displaying [`OSC`] progress sequences from cargo builds.
//!
//! This program demonstrates how to capture [`OSC`] (Operating System Command) sequences
//! emitted by cargo when running in a terminal that supports progress reporting. It uses
//! a pseudoterminal ([`PTY`]) to make cargo think it's running in an interactive
//! terminal, which triggers the emission of **`OSC 9;4`** progress sequences.
//!
//! # [`OSC`] Sequence Format
//!
//! Cargo emits [`OSC`] sequences in the format:
//! ```text
//! ESC ] 9 ; 4 ; {state} ; {progress} ESC \ (ST)
//! ```
//!
//! Where:
//! - `state` 0: Clear/remove progress
//! - `state` 1: Set specific progress (0-100%)
//! - `state` 2: Build error occurred
//! - `state` 3: Indeterminate progress
//!
//! # Usage
//!
//! Run this binary to see cargo build progress in real-time:
//! ```bash
//! cargo run --example spawn_pty_output_capture
//! ```
//!
//! See [`PtySessionConfigOption::CaptureOsc`] for the environment variables
//! required to trigger [`OSC`] emission from cargo.
//!
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtySessionConfigOption::CaptureOsc`]:
//!     crate::core::pty::PtySessionConfigOption::CaptureOsc

use miette::IntoDiagnostic;
use r3bl_tui::{OscEvent, SGR_FG_BRIGHT_GREEN_STR, SGR_FG_BRIGHT_RED_STR,
               SGR_FG_BRIGHT_YELLOW_STR, SGR_RESET_STR,
               core::pty::{DefaultPtySessionConfig, PtyOutputEvent, PtySessionBuilder,
                           PtySessionConfigOption},
               set_mimalloc_in_main};

// ANSI color constants for terminal output.

const YELLOW: &str = SGR_FG_BRIGHT_YELLOW_STR;
const GREEN: &str = SGR_FG_BRIGHT_GREEN_STR;
const RED: &str = SGR_FG_BRIGHT_RED_STR;
const RESET: &str = SGR_RESET_STR;

/// Runs cargo clean using the generic [`PTY`] command.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
async fn run_cargo_clean() -> miette::Result<()> {
    println!("{YELLOW}🧹 Running 'cargo clean' to ensure a fresh build...{RESET}");

    let mut session = PtySessionBuilder::new("cargo")
        .cli_args(["clean", "-q"])
        .with_config(DefaultPtySessionConfig + PtySessionConfigOption::NoCaptureOutput)
        .start()?;

    // Wait for completion.
    tokio::select! {
        result = &mut session.orchestrator_task_handle => {
            let status = result.into_diagnostic()??;
            if status.success() {
                println!("{GREEN}✓ Cargo clean completed successfully{RESET}\n");
            } else {
                return Err(miette::miette!("Cargo clean failed"));
            }
        }
        Some(event) = session.rx_output_event.recv() => {
            if let PtyOutputEvent::Exit(status) = event
                && !status.success() {
                return Err(miette::miette!("Cargo clean failed"));
            }
        }
    }

    Ok(())
}

/// Runs a single cargo build with [`OSC`] capture.
///
/// [`OSC`]: crate::osc_codes::OscSequence
async fn run_build_with_osc_capture(run_number: u32) -> miette::Result<()> {
    println!("{YELLOW}========================================");
    println!("{YELLOW}Starting Cargo build #{run_number} with OSC capture...");
    println!("{YELLOW}========================================{RESET}");
    // Configure cargo build command with OSC sequences enabled.
    let mut session = PtySessionBuilder::new("cargo")
        .cli_args(["build"])
        .with_config(DefaultPtySessionConfig + PtySessionConfigOption::CaptureOsc)
        .start()?;

    // Track if we saw any progress updates.
    let mut saw_progress = false;

    // Handle events as they arrive until cargo completes.
    loop {
        tokio::select! {
            // Handle cargo build completion.
            result = &mut session.orchestrator_task_handle => {
                let status = result.into_diagnostic()??;

                // Print summary.
                if saw_progress {
                    println!(
                        "{GREEN}✅ Build #{run_number} completed with progress tracking (status: {status:?}){RESET}"
                    );
                } else {
                    println!(
                        "{GREEN}✅ Build #{run_number} completed (no progress - everything cached) (status: {status:?}){RESET}"
                    );
                }
                break;
            }
            // Handle incoming PTY events.
            Some(event) = session.rx_output_event.recv() => {
                match event {
                    PtyOutputEvent::Osc(osc_event) => {
                        match osc_event {
                            OscEvent::ProgressUpdate(percentage) => {
                                saw_progress = true;
                                println!("{GREEN}📊 Build #{run_number} progress: {percentage}%{RESET}");
                            }
                            OscEvent::ProgressCleared => {
                                println!("{GREEN}✓ Progress tracking cleared{RESET}");
                            }
                            OscEvent::BuildError => {
                                println!("{RED}❌ Build error occurred{RESET}");
                            }
                            OscEvent::IndeterminateProgress => {
                                println!("{GREEN}⏳ Build in progress (indeterminate){RESET}");
                            }
                            OscEvent::Hyperlink { uri, text } => {
                                println!("{GREEN}🔗 Hyperlink detected: {text} -> {uri}{RESET}");
                            },
                            OscEvent::SetTitleAndTab(title) => {
                                println!("{GREEN}📝 Terminal title/tab update: {title}{RESET}");
                            }
                        }
                    }
                    PtyOutputEvent::Exit(_) | PtyOutputEvent::Output(_) | PtyOutputEvent::UnexpectedExit(_) | PtyOutputEvent::WriteError(_) | PtyOutputEvent::CursorModeChange(_) => {
                        // These events are ignored or handled by the completion logic.
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    println!(
        "\
        {YELLOW}╔═══════════════════════════════════════════════════════════════╗\n\
        {YELLOW}║  Demo: Cargo Build OSC Progress Sequences with Generic API    ║\n\
        {YELLOW}╚═══════════════════════════════════════════════════════════════╝{RESET}"
    );

    // Step 1: Run cargo clean to ensure the following build generates OSC sequences.
    println!("\n{YELLOW}▶ Step 1: Running cargo clean to ensure fresh build{RESET}");
    run_cargo_clean().await?;

    // Step 2: Run cargo build - should generate OSC sequences.
    println!("\n{YELLOW}▶ Step 2: First cargo build (expect progress updates){RESET}");
    run_build_with_osc_capture(1).await?;

    // Step 3: Run cargo build again - should NOT generate OSC sequences (cached).
    println!(
        "\n{YELLOW}▶ Step 3: Second cargo build (expect no progress - cached){RESET}"
    );
    run_build_with_osc_capture(2).await?;

    println!(
        "\n{GREEN}✨ Demo complete! The generic spawn_pty_command successfully captured OSC sequences.{RESET}"
    );

    Ok(())
}

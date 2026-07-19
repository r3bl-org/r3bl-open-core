// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Demo of [`OSC`] sequence handling in [`PTY`] Mux.
//!
//! This example demonstrates how [`PTY`] Mux can handle [`OSC`] sequences from processes
//! to dynamically update the terminal title.
//!
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use r3bl_tui::{DefaultIoDevices, IntoErr, TuiAvailability, TuiAvailabilityChooseExt,
               assert_terminal_is_interactive, choose,
               core::pty_mux::PTYMux,
               ok,
               readline_async::{HowToChoose, style::StyleSheet},
               set_mimalloc_in_main};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize tracing for debugging.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Create the multiplexer.
    let mux = match PTYMux::builder()
        .add_process("bash", "bash", vec![])
        .add_process(
            "OSC Demo",
            "bash",
            vec![
                "-c".to_string(),
                "echo 'This process will change the terminal title'; \
                  sleep 1; \
                  printf '\\033]0;Dynamic Title 1\\007'; \
                  echo 'Title changed to: Dynamic Title 1'; \
                  sleep 2; \
                  printf '\\033]2;Dynamic Title 2\\007'; \
                  echo 'Title changed to: Dynamic Title 2'; \
                  sleep 2; \
                  printf '\\033]1;Dynamic Title 3\\007'; \
                  echo 'Title changed to: Dynamic Title 3'; \
                  sleep 2; \
                  echo 'Demo complete'; \
                  exec bash"
                    .to_string(),
            ],
        )
        .add_process("htop", "htop", vec![])
        .build()
    {
        TuiAvailability::Available(mux) => mux,
        it => return it.into_err(),
    };

    println!("PTY Mux OSC Demo");
    println!("================");
    println!("Press F1-F3 to switch between processes");
    println!("Process 2 (F2) will demonstrate dynamic title changes");
    println!("Press Ctrl+Q to exit");
    println!();

    // Ask user for confirmation before taking over screen.
    let maybe_user_choice = {
        let mut default_io_devices = DefaultIoDevices::default();
        choose(
            "🚀 Ready to launch the PTY Mux OSC Demo?",
            &["Yes, launch OSC Demo", "No, exit"],
            None,
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .get_first_result()
        .await?
    };

    match maybe_user_choice {
        Some(ref choice) if choice.starts_with("Yes") => {}
        _ => {
            println!("👋 Exiting without running demo.");
            return ok!();
        }
    }

    mux.run().await?;

    ok!()
}

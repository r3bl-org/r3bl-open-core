// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Demo of OSC sequence handling in PTY Mux.
//!
//! This example demonstrates how PTY Mux can handle OSC sequences from processes
//! to dynamically update the terminal title.

use r3bl_tui::{Size,
               core::pty_mux::{PTYMux, Process},
               height, width};

#[tokio::main]
async fn main() -> miette::Result<()> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Get terminal size
    let (cols, rows) = crossterm::terminal::size()
        .map_err(|e| miette::miette!("Failed to get terminal size: {}", e))?;
    let terminal_size = Size {
        col_width: width(cols),
        row_height: height(rows),
    };

    // Create processes - one of them will emit OSC sequences
    let processes = vec![
        Process::new("bash", "bash", vec![], terminal_size),
        Process::new(
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
            terminal_size,
        ),
        Process::new("htop", "htop", vec![], terminal_size),
    ];

    // Create and run the multiplexer
    let mux = PTYMux::builder().processes(processes).build()?;

    println!("PTY Mux OSC Demo");
    println!("================");
    println!("Press F1-F3 to switch between processes");
    println!("Process 2 (F2) will demonstrate dynamic title changes");
    println!("Press Ctrl+Q to exit");
    println!();
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    mux.run().await?;

    Ok(())
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! `PTYMux` terminal multiplexer example.
//!
//! This example demonstrates how to use the `pty_mux` module to create a terminal
//! multiplexer similar to tmux, allowing you to run multiple TUI processes in a
//! single terminal window and switch between them using keyboard shortcuts.
//!
//! ## Features
//!
//! - **Multiple TUI processes**: Spawns multiple terminal applications
//! - **Dynamic process switching**: Use F1 through F9 to switch between processes
//! - **Live status bar**: Shows process status and available shortcuts
//! - **Terminal title updates**: Uses OSC sequences to update terminal title
//! - **Fake resize technique**: Ensures proper TUI app repainting when switching
//!
//! ## Usage
//!
//! Run this example with:
//! ```bash
//! cargo run --example pty_mux_example
//! ```
//!
//! Once running:
//! - `F1` to switch to claude (AI assistant)
//! - `F2` to switch to less (file viewer)
//! - `F3` to switch to htop (process monitor)
//! - `F4` to switch to gitui (git TUI)
//! - `Ctrl+Q` to quit
//! - The status bar shows live process status and available shortcuts
//!
//! ## Configured Processes
//!
//! This example is configured to run the following TUI applications:
//! - `less /etc/adduser.conf` - File pager for viewing configuration
//! - `htop` - Process monitor
//! - `claude` - Claude AI assistant CLI
//! - `gitui` - Git terminal user interface
//!
//! Note: All processes are started immediately at startup for fast switching.
//! All applications are proper TUI applications that respond to SIGWINCH
//! and will repaint correctly when switching between them.

use r3bl_tui::{core::{pty_mux::{PTYMux, Process},
                      term::{TTYResult, is_fully_interactive_terminal},
                      try_initialize_logging_global},
               set_mimalloc_in_main};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Initialize logging to log.txt
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    tracing::debug!("Starting PTYMux Example");

    // Check if running in interactive terminal
    if is_fully_interactive_terminal() == TTYResult::IsNotInteractive {
        eprintln!("❌ This example requires an interactive terminal to run.");
        eprintln!(
            "   Please run directly in a terminal, not through pipes or non-TTY environments."
        );
        std::process::exit(1);
    }

    println!("🚀 Starting PTYMux Example");
    println!("📋 Configured processes: claude, less, htop, gitui");
    println!("⌨️  Controls:");
    println!("   • F1: claude");
    println!("   • F2: less");
    println!("   • F3: htop");
    println!("   • F4: gitui");
    println!("   • Ctrl+Q: Quit");
    println!("📊 Status bar will show live process status and shortcuts");
    println!("📝 Debug output will be written to log.txt");
    println!();

    // TUI processes only - all are proper TUI applications that respond to SIGWINCH
    let processes = vec![
        Process::new("claude", "/home/nazmul/.claude/local/claude", vec![]),
        Process::new("less", "less", vec!["/etc/adduser.conf".to_string()]),
        Process::new("htop", "htop", vec![]),
        Process::new("gitui", "gitui", vec![]),
    ];

    println!(
        "🔧 Building multiplexer with {} processes...",
        processes.len()
    );

    // Build and run multiplexer using the pty_mux module
    let multiplexer = PTYMux::builder().processes(processes).build()?;

    println!("▶️  Starting multiplexer event loop...");
    println!("   (All processes will be started immediately for fast switching)");
    println!("   Press F1-F4 to switch processes, Ctrl+Q to quit");
    println!();

    // Run the multiplexer event loop
    tracing::debug!("About to start multiplexer.run()");
    let run_result = multiplexer.run().await;
    tracing::debug!("multiplexer.run() completed with result: {:?}", run_result);

    // Check for any errors from the run
    run_result?;

    println!("👋 PTYMux session ended. Goodbye!");

    tracing::debug!("Main function completing successfully");

    // Allow a brief moment for any final cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}

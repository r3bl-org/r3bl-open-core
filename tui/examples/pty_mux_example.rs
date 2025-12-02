// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! `PTYMux` terminal multiplexer example with universal process compatibility.
//!
//! This example demonstrates how to use the `pty_mux` module to create a terminal
//! multiplexer similar to tmux, but with enhanced support for truecolor and TUI apps that
//! frequently re-render their UI, with support for ALL types of programs:
//! interactive shells, TUI applications, and CLI tools.
//!
//! ## Features
//!
//! - **Universal compatibility**: Supports bash, TUI apps, and CLI tools
//! - **Per-process virtual terminals**: Each process maintains its own buffer
//! - **Instant switching**: No delays or fake resize hacks needed
//! - **Dynamic process switching**: Use F1 through F9 to switch between processes
//! - **Live status bar**: Shows process status and available shortcuts
//! - **Terminal title updates**: Uses OSC sequences to update terminal title
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
//! - `F5` to switch to bash (interactive shell)
//! - `Ctrl+Q` to quit
//! - The status bar shows live process status and available shortcuts
//!
//! ## Configured Processes
//!
//! This example demonstrates universal compatibility with different process types:
//! - `claude` - Claude AI assistant CLI (interactive TUI app)
//! - `less /etc/adduser.conf` - File pager for viewing configuration
//! - `htop` - Process monitor (full-screen TUI)
//! - `gitui` - Git terminal user interface (interactive TUI)
//! - `bash` - Interactive shell (demonstrates universal compatibility)
//!
//! Note: All processes are started immediately at startup for fast switching.
//! All applications are proper TUI applications that respond to SIGWINCH
//! and will repaint correctly when switching between them.

use r3bl_tui::{core::{get_size,
                      pty_mux::{PTYMux, Process},
                      term::{TTYResult, is_stdin_interactive},
                      try_initialize_logging_global},
               set_mimalloc_in_main};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Initialize logging to log.txt.
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    tracing::debug!("Starting PTYMux Example");

    // Check if running in interactive terminal.
    if is_stdin_interactive() == TTYResult::IsNotInteractive {
        eprintln!("❌ This example requires an interactive terminal to run.");
        eprintln!(
            "   Please run directly in a terminal, not through pipes or non-TTY environments."
        );
        std::process::exit(1);
    }

    println!("🚀 Starting PTYMux Example - Universal Process Compatibility");
    println!("📋 Configured processes: claude, less, htop, gitui, bash");
    println!("🌟 Demonstrates universal compatibility:");
    println!("   • AI assistant (claude) with interactive chat");
    println!("   • TUI applications (less, htop, gitui) with proper ANSI handling");
    println!("   • Interactive shells (bash) with persistent command history");
    println!("   • Per-process virtual terminals for instant switching");
    println!("⌨️  Controls:");
    println!("   • F1: claude (AI assistant)");
    println!("   • F2: less (file viewer)");
    println!("   • F3: htop (process monitor)");
    println!("   • F4: gitui (git TUI)");
    println!("   • F5: bash (interactive shell)");
    println!("   • Ctrl+Q: Quit");
    println!("📊 Status bar shows live process status and shortcuts");
    println!("📝 Debug output will be written to log.txt");
    println!();

    // Get terminal size for process creation.
    let terminal_size = get_size()?;

    // Mixed process types demonstrating universal compatibility:
    // - claude: AI assistant (existing TUI app)
    // - TUI apps: less, htop, gitui (proper TUI applications)
    // - bash: Interactive shell (universal compatibility demonstration)
    let processes = vec![
        Process::new(
            "claude",
            "/home/nazmul/.claude/local/claude",
            vec![],
            terminal_size,
        ),
        Process::new(
            "less",
            "less",
            vec!["/etc/adduser.conf".to_string()],
            terminal_size,
        ),
        Process::new("htop", "htop", vec![], terminal_size),
        Process::new("gitui", "gitui", vec![], terminal_size),
        Process::new("bash", "bash", vec![], terminal_size),
    ];

    println!(
        "🔧 Building multiplexer with {} processes...",
        processes.len()
    );

    // Build and run multiplexer using the pty_mux module.
    let multiplexer = PTYMux::builder().processes(processes).build()?;

    println!("▶️  Starting multiplexer event loop...");
    println!("   (All processes will be started immediately for fast switching)");
    println!("   Press F1-F4 to switch processes, Ctrl+Q to quit");
    println!();

    // Run the multiplexer event loop.
    tracing::debug!("About to start multiplexer.run()");
    let run_result = multiplexer.run().await;
    tracing::debug!("multiplexer.run() completed with result: {:?}", run_result);

    // Check for any errors from the run.
    run_result?;

    println!("👋 PTYMux session ended. Goodbye!");

    tracing::debug!("Main function completing successfully");

    // Allow a brief moment for any final cleanup.
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}

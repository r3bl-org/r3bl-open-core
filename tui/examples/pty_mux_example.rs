// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words adduser

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
//! - **Terminal title updates**: Uses [`OSC`] sequences to update terminal title
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
//!
//! [`OSC`]: crate::osc_codes::OscSequence

use r3bl_tui::{IntoErr, TuiAvailability, assert_terminal_is_interactive,
               core::pty_mux::PTYMux, ok, set_mimalloc_in_main,
               try_initialize_logging_global};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize logging to /tmp/r3bl_tui/log.txt.
    let _log_guard = try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    tracing::debug!("Starting PTYMux Example");

    // Mixed process types demonstrating universal compatibility:
    // - claude: AI assistant (existing TUI app)
    // - TUI apps: less, htop, gitui (proper TUI applications)
    // - bash: Interactive shell (universal compatibility demonstration)
    let processes = vec![
        ("claude", "claude", vec![]),
        ("less", "less", vec!["/etc/adduser.conf".to_string()]),
        ("htop", "htop", vec![]),
        ("gitui", "gitui", vec![]),
        ("bash", "bash", vec![]),
    ];

    println!("🚀 Starting PTYMux Example - Universal Process Compatibility");

    // List available processes
    println!("📋 Available processes:");
    let mut current_f_key = 1;
    for (name, command, _args) in &processes {
        if r3bl_tui::is_command_available(command) {
            println!("   • F{current_f_key}: {name} ({command})");
            current_f_key += 1;
        }
    }
    println!("   • Ctrl+Q: Quit");
    println!("📊 Status bar shows live process status and shortcuts");
    println!("📝 Debug output will be written to /tmp/r3bl_tui/log.txt");
    println!();

    let mut builder = PTYMux::builder();
    let mut added_count = 0;

    for (name, command, args) in processes {
        if r3bl_tui::is_command_available(command) {
            builder = builder.add_process(name, command, args);
            added_count += 1;
        }
    }

    if added_count == 0 {
        miette::bail!(
            "No configured processes are available on this system. Please ensure at least one of (claude, less, htop, gitui, bash) is installed and in PATH."
        );
    }

    let multiplexer = match builder.build() {
        TuiAvailability::Available(mux) => mux,
        it => return it.into_err(),
    };

    println!("🛫 Starting multiplexer event loop...");
    println!("   (All processes will be started immediately for fast switching)");
    println!("   Press F1-F{added_count} to switch processes, Ctrl+Q to quit");
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

    ok!()
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words adduser

//! [`PTYMux`] terminal multiplexer example with universal process compatibility.
//!
//! This example demonstrates how to use the [`pty_mux`] module to create a terminal
//! multiplexer similar to tmux, but with enhanced support for truecolor and TUI apps that
//! frequently re-render their UI, with support for ALL types of programs: interactive
//! shells, TUI applications, and CLI tools.
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
//! - `F1` to switch to hx (Helix text editor)
//! - `F2` to switch to less (file viewer)
//! - `F3` to switch to htop (process monitor)
//! - `F4` to switch to gitui (git TUI)
//! - `F5` to switch to bash (interactive shell)
//! - `F6` to switch to fish (interactive shell)
//! - `Ctrl+Q` to quit
//! - The status bar shows live process status and available shortcuts
//!
//! ## Configured Processes
//!
//! This example demonstrates universal compatibility with different process types:
//! - `hx` - Helix modal text editor (interactive TUI app)
//! - `less /etc/adduser.conf` - File pager for viewing configuration
//! - `htop` - Process monitor (full-screen TUI)
//! - `gitui` - Git terminal user interface (interactive TUI)
//! - `bash` - Interactive shell (demonstrates universal compatibility)
//! - `fish` - Interactive shell (demonstrates universal compatibility without timeouts)
//!
//! Note: All processes are started immediately at startup for fast switching. All
//! applications are proper TUI applications that respond to [`SIGWINCH`] and will repaint
//! correctly when switching between them.
//!
//! [`OSC`]: r3bl_tui::core::ansi::osc_codes::OscSequence
//! [`pty_mux`]: r3bl_tui::core::pty_mux
//! [`PTYMux`]: r3bl_tui::core::pty_mux::PTYMux
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH

use r3bl_tui::{EventPropagation, InputEvent, IntoErr, Key, KeyPress, KeyState,
               ModifierKeysMask, TuiAvailability, assert_terminal_is_interactive,
               core::pty_mux::{PTYMux, ProcessManager},
               is_command_available, ok, set_mimalloc_in_main,
               show_notification_non_blocking, try_initialize_logging_global};
use tracing_core::LevelFilter;

const ENABLE_NOTIFICATIONS: bool = false;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize logging to /tmp/r3bl_tui/log.txt.
    let _log_guard = try_initialize_logging_global(LevelFilter::DEBUG).ok();
    tracing::debug!("Starting PTYMux Example");

    // Mixed process types demonstrating universal compatibility:
    // - hx: Helix text editor (existing TUI app)
    // - TUI apps: less, htop, gitui (proper TUI applications)
    // - bash, fish: Interactive shells (universal compatibility demonstration)
    let processes = vec![
        ("hx", "hx", vec![]),
        ("less", "less", vec!["Cargo.toml".to_string()]),
        ("htop", "htop", vec![]),
        ("gitui", "gitui", vec![]),
        ("bash", "bash", vec![]),
        ("fish", "fish", vec![]),
    ];

    println!("🚀 Starting PTYMux Example - Universal Process Compatibility");

    // List available processes
    println!("📋 Available processes:");
    let mut current_f_key = 1;
    for (name, command, _args) in &processes {
        if is_command_available(command) {
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
        if is_command_available(command) {
            builder = builder.add_process(name, command, args);
            added_count += 1;
        }
    }

    if added_count == 0 {
        miette::bail!(
            "No configured processes are available on this system. \
            Please ensure at least one of (hx, less, htop, gitui, bash, fish) \
            is installed and in PATH."
        );
    }

    builder = builder.input_interceptor_fn(Box::new(interceptor_fn));

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

fn interceptor_fn(
    input_event: &InputEvent,
    process_manager: &mut ProcessManager,
) -> EventPropagation {
    match input_event {
        // 1. Handle F1-F12 keys to switch processes.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::FunctionKey(fn_key),
        }) => {
            let fn_number = u8::from(*fn_key);
            let process_index = (fn_number - 1) as usize;

            if process_index < process_manager.processes().len() {
                let old_index = process_manager.active_index();
                if old_index != process_index {
                    process_manager.switch_to(process_index);

                    if ENABLE_NOTIFICATIONS {
                        let process_name =
                            &process_manager.processes()[process_index].command;
                        show_notification_non_blocking(
                            "PTY Mux - Process Switch",
                            &format!("Switching to {process_name}"),
                        );
                    }
                }
                return EventPropagation::ConsumedRender;
            }
        }

        // 2. Handle Ctrl+Q to exit.
        InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('q'),
            mask:
                ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    ..
                },
        }) => {
            if ENABLE_NOTIFICATIONS {
                show_notification_non_blocking("PTY Mux - Exit", "Exiting PTY Mux");
            }
            return EventPropagation::ExitMainEventLoop;
        }

        // 3. Log other unhandled keyboard events.
        InputEvent::Keyboard(key) => {
            if ENABLE_NOTIFICATIONS {
                show_notification_non_blocking(
                    "PTY Mux - Key Press",
                    &format!("Key pressed: {key:?}"),
                );
            }
        }

        // 4. Log other non-mouse input events.
        other_event
            if ENABLE_NOTIFICATIONS && !matches!(other_event, InputEvent::Mouse(_)) =>
        {
            show_notification_non_blocking(
                "PTY Mux - Input Event",
                &format!("Input event received: {other_event:?}"),
            );
        }

        // 5. Ignore everything else.
        _ => {}
    }

    EventPropagation::Propagate
}

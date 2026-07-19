// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words adduser trackpads

//! [`PTYMux`] 2D panning and infinite canvas showcase.
//!
//! This example demonstrates the infinite 2D canvas capabilities of the [`pty_mux`]
//! module. It showcases vertical scrolling (scrollback history) and horizontal panning,
//! allowing the user to navigate the virtual terminal buffer beyond the physical bounds
//! of the screen.
//!
//! ## Features
//!
//! - **Vertical Scrolling**: Standard scrollback history using Mouse Wheel or Up/Down.
//! - **Horizontal Panning**: Pan the virtual viewport left and right using Shift+Mouse
//!   Wheel, or horizontal scroll gestures on trackpads.
//! - **Infinite 2D Canvas**: The underlying virtual buffer stores content beyond the
//!   physical viewport.
//!
//! ## Usage
//!
//! Run this example with:
//! ```bash
//! cargo run --example pty_2d_panning_example
//! ```
//!
//! Once running:
//! - `F1` to switch to bash (try running a command with long output like `dmesg` or
//!   `tree`)
//! - `F2` to switch to htop
//! - `Mouse Wheel Up/Down` to scroll vertically into the history.
//! - `Shift + Mouse Wheel Up/Down` to pan horizontally left/right.
//! - `Ctrl+Q` to quit
//!
//! [`OSC`]: r3bl_tui::core::ansi::osc_codes::OscSequence
//! [`pty_mux`]: r3bl_tui::core::pty_mux
//! [`PTYMux`]: r3bl_tui::core::pty_mux::PTYMux

use r3bl_tui::{DefaultIoDevices, EventPropagation, InputEvent, IntoErr, Key, KeyPress,
               KeyState, ModifierKeysMask, TuiAvailability, TuiAvailabilityChooseExt,
               assert_terminal_is_interactive, choose,
               core::pty_mux::{PTYMux, ProcessManager},
               is_command_available, ok,
               readline_async::{HowToChoose, style::StyleSheet},
               set_mimalloc_in_main, show_notification_non_blocking,
               try_initialize_logging_global, vp_width};
use tracing_core::LevelFilter;

const ENABLE_NOTIFICATIONS: bool = false;
const VIRTUAL_TERMINAL_WIDTH: usize = 1000;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize logging to /tmp/r3bl_tui/log.txt.
    let _log_guard = try_initialize_logging_global(LevelFilter::DEBUG).ok();
    tracing::debug!("Starting 2D Panning Example");

    // Processes optimized for demonstrating panning
    let processes = vec![("bash", "bash", vec![]), ("htop", "htop", vec![])];

    println!("🚀 Starting 2D Panning Example - Infinite Canvas Showcase");

    // List available processes
    println!("📋 Available processes:");
    let mut current_f_key = 1;
    for (name, command, _args) in &processes {
        if is_command_available(command) {
            println!("   • F{current_f_key}: {name} ({command})");
            current_f_key += 1;
        }
    }
    println!("   • Mouse Wheel: Vertical scrollback");
    println!(
        "   • Shift+Mouse Wheel: Horizontal panning (across {VIRTUAL_TERMINAL_WIDTH}-column virtual canvas)"
    );
    println!("   • Ctrl+Q: Quit");
    println!("📊 Status bar shows live process status and shortcuts");
    println!("📝 Debug output will be written to /tmp/r3bl_tui/log.txt");
    println!();

    // Ask user for confirmation before taking over screen.
    let maybe_user_choice = {
        let mut default_io_devices = DefaultIoDevices::default();
        choose(
            "🚀 Ready to launch the 2D Panning multiplexer demo?",
            &["Yes, launch 2D Panning demo (Infinite Canvas)", "No, exit"],
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

    let mut builder =
        PTYMux::builder().virtual_terminal_width(vp_width(VIRTUAL_TERMINAL_WIDTH));
    let mut added_count = 0;

    for (name, command, args) in processes {
        if is_command_available(command) {
            builder = builder.add_process(name, command, args);
            added_count += 1;
        }
    }

    if added_count == 0 {
        return Err(miette::miette!(
            "No configured processes are available on this system. \
            Please ensure at least one of (hx, less, htop, gitui, bash, fish) \
            is installed and in PATH."
        ));
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
            let process_index = usize::from(fn_number.saturating_sub(1));

            if process_index < process_manager.processes().len() {
                let old_index = process_manager.focused_index();
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

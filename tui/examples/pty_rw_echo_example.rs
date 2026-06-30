// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Super minimal [`PTY`] test - just echo raw bytes to verify data flow
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use r3bl_tui::{InputEvent, Key, KeyPress, KeyState, ModifierKeysMask,
               TerminalModeController, assert_terminal_is_interactive, col,
               core::{get_size,
                      pty::{ControlSequence, CursorKeyMode, DefaultPtySessionConfig,
                            PtyInputEvent, PtyOutputEvent, PtySessionBuilder,
                            PtySessionConfigOption},
                      terminal_io::{InputDevice, OutputDevice},
                      try_initialize_logging_global},
               ok, row, set_mimalloc_in_main};
use std::io::Write;

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize logging.
    let _log_guard = try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();

    println!("🚀 Starting Echo Test");
    println!("📋 Running 'cat' - it will echo whatever you type");
    println!("⌨️  Type anything, Ctrl+Q to quit");
    println!();

    let terminal_size = get_size()?;

    let output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::default();

    // Start raw mode and full screen TUI
    let _raw_mode_guard = output_device.enter_raw_mode()?;
    let _fullscreen_tui_mode_guard = output_device.setup_full_screen_tui()?;

    // Clear screen.
    output_device.write(|out| {
        let _unused = out
            .write_all(r3bl_tui::ansi_output::screen_clearing::clear_screen().as_bytes());
        let _unused = out.write_all(
            r3bl_tui::ansi_output::cursor_movement::cursor_position(row(0), col(0))
                .as_bytes(),
        );
        let _unused = out.flush();
    });

    // Spawn cat process (simple echo).
    let mut session = PtySessionBuilder::new("cat")
        .with_config(
            DefaultPtySessionConfig + PtySessionConfigOption::Size(terminal_size),
        )
        .start()?;

    println!("Type something and press Enter to see it echo back:");

    // Simple event loop.
    loop {
        tokio::select! {
            // Handle PTY output.
            Some(event) = session.rx_output_event.recv() => {
                match event {
                    PtyOutputEvent::Output(data) => {
                        // Just write raw bytes directly to terminal.
                        print!("{}", String::from_utf8_lossy(&data));
                        std::io::stdout().flush().unwrap();
                    }
                    PtyOutputEvent::Exit(_) => {
                        break;
                    }
                    _ => {}
                }
            }

            // Handle user input.
            Some(input_event) = input_device.next() => {
                if let InputEvent::Keyboard(key) = input_event {
                    // Check for Ctrl+Q.
                    if let KeyPress::WithModifiers {
                        key: Key::Character('q'),
                        mask: ModifierKeysMask {
                            ctrl_key_state: KeyState::Pressed,
                            shift_key_state: KeyState::NotPressed,
                            alt_key_state: KeyState::NotPressed,
                        },
                    } = key {
                        // Send Ctrl+D to cat.
                        let _unused = session.tx_input_event.send(PtyInputEvent::SendControl(ControlSequence::CtrlD, CursorKeyMode::default()));
                        break;
                    }

                    // Convert key to PTY event and send.
                    if let Some(event) = Option::<PtyInputEvent>::from(key) {
                        let _unused = session.tx_input_event.send(event);
                    }
                }
            }
        }
    }

    // Cleanup.
    // `_fullscreen_tui_mode_guard` and `_raw_mode_guard` are dropped here.
    output_device.flush()?;

    println!("\n👋 Goodbye!");
    ok!()
}

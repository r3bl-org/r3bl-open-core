// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Super minimal PTY test - just echo raw bytes to verify data flow

use portable_pty::PtySize;
use r3bl_tui::{clear_screen_and_home_cursor,
               core::{get_size,
                      pty::{ControlSequence, CursorKeyMode, PtyCommandBuilder,
                            PtyInputEvent, PtyReadWriteOutputEvent},
                      terminal_io::{InputDevice, OutputDevice},
                      try_initialize_logging_global},
               lock_output_device_as_mut, set_mimalloc_in_main,
               tui::terminal_lib_backends::{InputEvent, Key, KeyPress, KeyState,
                                            ModifierKeysMask, RawMode}};
use std::io::Write;

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Initialize logging.
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();

    println!("🚀 Starting Echo Test");
    println!("📋 Running 'cat' - it will echo whatever you type");
    println!("⌨️  Type anything, Ctrl+Q to quit");
    println!();

    let terminal_size = get_size()?;
    let output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::new_event_stream();

    // Start raw mode.
    RawMode::start(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );

    // Clear screen.
    clear_screen_and_home_cursor(&output_device);

    // Spawn cat process (simple echo).
    let pty_size = PtySize {
        rows: terminal_size.row_height.into(),
        cols: terminal_size.col_width.into(),
        pixel_width: 0,
        pixel_height: 0,
    };

    let mut session = PtyCommandBuilder::new("cat").spawn_read_write(pty_size)?;

    println!("Type something and press Enter to see it echo back:");

    // Simple event loop.
    loop {
        tokio::select! {
            // Handle PTY output.
            Some(event) = session.output_event_receiver_half.recv() => {
                match event {
                    PtyReadWriteOutputEvent::Output(data) => {
                        // Just write raw bytes directly to terminal.
                        print!("{}", String::from_utf8_lossy(&data));
                        std::io::stdout().flush().unwrap();
                    }
                    PtyReadWriteOutputEvent::Exit(_) => {
                        break;
                    }
                    _ => {}
                }
            }

            // Handle user input.
            Ok(event) = input_device.next() => {
                let Ok(input_event) = InputEvent::try_from(event) else { continue };

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
                        let _unused = session.input_event_ch_tx_half.send(PtyInputEvent::SendControl(ControlSequence::CtrlD, CursorKeyMode::default()));
                        break;
                    }

                    // Convert key to PTY event and send.
                    if let Some(event) = Option::<PtyInputEvent>::from(key) {
                        let _unused = session.input_event_ch_tx_half.send(event);
                    }
                }
            }
        }
    }

    // Cleanup.
    RawMode::end(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );

    println!("\n👋 Goodbye!");
    Ok(())
}

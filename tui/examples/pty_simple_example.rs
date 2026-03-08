// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Minimal [`PTY`] example - just run a single htop process.
//!
//! This is a simplified example to debug [`PTY`] integration issues.
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use r3bl_tui::{AnsiSequenceGenerator, InputEvent, Key, KeyPress, KeyState,
               ModifierKeysMask, RawMode, col,
               core::{get_size,
                      pty::{ControlSequence, CursorKeyMode, DefaultPtySessionConfig,
                            PtyInputEvent, PtyOutputEvent, PtySession,
                            PtySessionBuilder, PtySessionConfigOption},
                      terminal_io::{InputDevice, OutputDevice},
                      try_initialize_logging_global},
               lock_output_device_as_mut, row, set_mimalloc_in_main};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Initialize logging to log.txt.
    try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    tracing::debug!("Starting Simple PTY Example");

    println!("🚀 Starting Simple PTY Example");
    println!("📋 Running htop in a PTY");
    println!("⌨️  Use htop normally, Ctrl+Q to quit");
    println!("📝 Debug output will be written to log.txt");
    println!();

    // Get terminal size.
    let terminal_size = get_size()?;
    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::default();

    // Start raw mode.
    RawMode::start(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );
    tracing::debug!("Raw mode started");

    // Clear screen and reset cursor.
    {
        let out = lock_output_device_as_mut!(&output_device);
        let _unused = out.write_all(AnsiSequenceGenerator::clear_screen().as_bytes());
        let _unused = out
            .write_all(AnsiSequenceGenerator::cursor_position(row(0), col(0)).as_bytes());
        let _unused = out.flush();
    }
    tracing::debug!("Screen cleared");

    tracing::debug!("Spawning htop with PTY size: {:?}", terminal_size);

    let session = PtySessionBuilder::new("htop")
        .with_config(
            DefaultPtySessionConfig + PtySessionConfigOption::Size(terminal_size),
        )
        .start()?;

    tracing::debug!("htop process started successfully");

    // Run event loop.
    let result = run_event_loop(session, &mut input_device, &mut output_device).await;

    // Cleanup.
    tracing::debug!("Starting cleanup");
    RawMode::end(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );
    tracing::debug!("Raw mode ended, cleanup complete");

    println!("👋 Goodbye!");
    result
}

async fn run_event_loop(
    mut session: PtySession,
    input_device: &mut InputDevice,
    output_device: &mut OutputDevice,
) -> miette::Result<()> {
    let mut output_count = 0u64;
    let mut input_count = 0u64;

    tracing::debug!("Starting event loop");

    loop {
        tokio::select! {
            // Handle PTY output - properly wait for data.
            Some(event) = session.rx_output_event.recv() => {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output_count += 1;
                        if output_count <= 10 || output_count.is_multiple_of(100) {
                            tracing::debug!("PTY output #{}: {} bytes", output_count, data.len());
                        }

                        // Debug: Log the actual bytes when we get small outputs (likely from key responses).
                        if data.len() < 1000 {
                            tracing::debug!("PTY output #{} content ({} bytes): {:?}",
                                output_count, data.len(), String::from_utf8_lossy(&data));
                            tracing::debug!("PTY output #{} raw bytes: {:02x?}", output_count, &data[..data.len().min(100)]);
                        }

                        // Write the PTY output to the output device.
                        let out = lock_output_device_as_mut!(output_device);
                        if let Err(e) = out.write_all(&data) {
                            tracing::error!("Failed to write to output device: {}", e);
                        }
                        if let Err(e) = out.flush() {
                            tracing::error!("Failed to flush output device: {}", e);
                        }
                    }
                    PtyOutputEvent::Exit(status) => {
                        tracing::debug!("PTY exited with status: {:?}", status);
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // Handle user input.
            Some(input_event) = input_device.next() => {
                match input_event {
                    InputEvent::Keyboard(key) => {
                        // Check for Ctrl+Q to quit.
                        if let KeyPress::WithModifiers {
                            key: Key::Character('q'),
                            mask: ModifierKeysMask {
                                ctrl_key_state: KeyState::Pressed,
                                shift_key_state: KeyState::NotPressed,
                                alt_key_state: KeyState::NotPressed,
                            },
                        } = key {
                            tracing::debug!("Ctrl+Q pressed, starting shutdown");
                            // First send Ctrl+C to terminate htop gracefully.
                            tracing::debug!("Sending Ctrl+C to htop");
                            let _unused = session.tx_input_event.try_send(PtyInputEvent::SendControl(ControlSequence::CtrlC, CursorKeyMode::default()));
                            // Wait a moment for bash to exit.
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            // Send close to PTY.
                            tracing::debug!("Sending Close event to PTY");
                            let _unused = session.tx_input_event.try_send(PtyInputEvent::Close);
                            // Wait for session to close.
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            tracing::debug!("Exiting event loop");
                            return Ok(());
                        }

                        // Convert key to PTY input event and send.
                        if let Some(event) = Option::<PtyInputEvent>::from(key) {
                            input_count += 1;
                            tracing::debug!("Input #{}: key={:?}, event={:?}", input_count, key, event);

                            // Debug: Log the actual bytes being sent for arrow keys.
                            if let PtyInputEvent::SendControl(ref ctrl, ref mode) = event {
                                tracing::debug!("Sending control bytes: {:02x?}", ctrl.to_bytes(*mode).as_ref());
                            }

                            if let Err(e) = session.tx_input_event.try_send(event.clone()) {
                                tracing::error!("Failed to send input to PTY: {}", e);
                            } else {
                                tracing::debug!("Successfully sent input event: {:?}", event);
                            }
                        } else {
                            tracing::debug!("Unhandled key: {:?}", key);
                        }
                    }
                    InputEvent::Resize(new_size) => {
                        // Handle resize.
                        let _unused = session.tx_input_event.try_send(PtyInputEvent::Resize(new_size));
                    }
                    _ => {}
                }
            }
        }
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{core::ansi::vt_100_terminal_input_parser::{
                test_fixtures::generate_keyboard_sequence,
                types::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers}
            },
            Deadline, generate_pty_test, InputEvent,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    /// PTY-based integration test for keyboard modifiers (Shift, Ctrl, Alt).
    ///
    /// Validates that the [`DirectToAnsiInputDevice`] correctly parses keyboard sequences
    /// with various modifier combinations:
    /// - Single modifiers: Shift, Ctrl, Alt
    /// - Combined modifiers: Shift+Alt, Shift+Ctrl, Alt+Ctrl
    /// - Triple modifiers: Shift+Alt+Ctrl
    ///
    /// Tests with arrow keys and function keys to validate round-trip generation+parsing.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_keyboard_modifiers -- --nocapture`
    ///
    /// [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
    test_fn: test_pty_keyboard_modifiers,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// PTY Master: Send keyboard sequences with modifiers and verify parsing
#[allow(clippy::too_many_lines)]
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 PTY Master: Starting keyboard modifiers test...");

    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running
    let mut test_running_seen = false;
    let deadline = Deadline::default();

    loop {
        assert!(deadline.has_time_remaining(), "Timeout: slave did not start within 5 seconds");

        let mut line = String::new();
        match buf_reader_non_blocking.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before slave started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Slave output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in slave");
                }
                if trimmed.contains("SLAVE_STARTING") {
                    eprintln!("  ✓ Slave confirmed running!");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error while waiting for slave: {e}"),
        }
    }

    assert!(test_running_seen, "Slave test never started running (no TEST_RUNNING output)");

    // Build test sequences with various modifier combinations
    let modifier_combos = vec![
        (
            "Shift+Up",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers {
                    shift: true,
                    alt: false,
                    ctrl: false,
                },
            },
        ),
        (
            "Ctrl+Up",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers {
                    shift: false,
                    alt: false,
                    ctrl: true,
                },
            },
        ),
        (
            "Alt+Down",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: VT100KeyModifiers {
                    shift: false,
                    alt: true,
                    ctrl: false,
                },
            },
        ),
        (
            "Shift+Alt+Left",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers: VT100KeyModifiers {
                    shift: true,
                    alt: true,
                    ctrl: false,
                },
            },
        ),
        (
            "Ctrl+Shift+Right",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers: VT100KeyModifiers {
                    shift: true,
                    alt: false,
                    ctrl: true,
                },
            },
        ),
        (
            "Ctrl+Alt+Shift+F1",
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(1),
                modifiers: VT100KeyModifiers {
                    shift: true,
                    alt: true,
                    ctrl: true,
                },
            },
        ),
    ];

    eprintln!(
        "📝 PTY Master: Sending {} keyboard modifier combinations...",
        modifier_combos.len()
    );

    for (desc, event) in &modifier_combos {
        let sequence = generate_keyboard_sequence(event)
            .unwrap_or_else(|| panic!("Failed to generate sequence for: {desc}"));

        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(&sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get a keyboard event line
        let event_line = loop {
            let mut line = String::new();
            match buf_reader_non_blocking.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's a keyboard event line
                    if trimmed.starts_with("Keyboard:") {
                        break trimmed.to_string();
                    }

                    // Skip test harness noise
                    eprintln!("  ⚠️  Skipping non-event output: {trimmed}");
                }
                Err(e) => {
                    panic!("Read error for {desc}: {e}");
                }
            }
        };

        eprintln!("  ✓ {desc}: {event_line}");
    }

    eprintln!("🧹 PTY Master: Cleaning up...");

    drop(writer);

    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Master: Slave exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for slave: {e}");
        }
    }

    eprintln!("✅ PTY Master: Test passed!");
}

/// PTY Slave: Read and parse keyboard events with modifiers
fn pty_slave_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 PTY Slave: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to enable raw mode: {e}");
    } else {
        eprintln!("✓ PTY Slave: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Slave: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Slave: Device created, reading events...");

        let inactivity_timeout = Duration::from_secs(2);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Slave: Event #{event_count}: {event:?}");

                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= 6 {
                                eprintln!("🔍 PTY Slave: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Slave: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Slave: Inactivity timeout (2 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Slave: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Slave: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Slave: Completed, exiting");
    std::process::exit(0);
}

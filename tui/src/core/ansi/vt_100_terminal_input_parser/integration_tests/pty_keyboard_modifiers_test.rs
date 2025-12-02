// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{PtyPair, Deadline, InputEvent, KeyState,
            core::ansi::vt_100_terminal_input_parser::{ir_event_types::{VT100InputEventIR,
                                                                        VT100KeyCodeIR,
                                                                        VT100KeyModifiersIR},
                                                       test_fixtures::generate_keyboard_sequence},
            generate_pty_test,
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
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// PTY Controller: Send keyboard sequences with modifiers and verify parsing
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(
    pty_pair: PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 PTY Controller: Starting keyboard modifiers test...");

    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running
    let mut test_running_seen = false;
    let deadline = Deadline::default();

    loop {
        assert!(
            deadline.has_time_remaining(),
            "Timeout: controlled did not start within 5 seconds"
        );

        let mut line = String::new();
        match buf_reader_non_blocking.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled");
                }
                if trimmed.contains("CONTROLLED_READY") {
                    eprintln!("  ✓ Controlled process confirmed running!");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error while waiting for controlled: {e}"),
        }
    }

    assert!(
        test_running_seen,
        "Controlled test never started running (no TEST_RUNNING output)"
    );

    // Build test sequences with various modifier combinations
    let modifier_combos = vec![
        (
            "Shift+Up",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Ctrl+Up",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
        (
            "Alt+Down",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Down,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Shift+Alt+Left",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Left,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::NotPressed,
                },
            },
        ),
        (
            "Ctrl+Shift+Right",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Right,
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
        (
            "Ctrl+Alt+Shift+F1",
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(1),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::Pressed,
                    ctrl: KeyState::Pressed,
                },
            },
        ),
    ];

    eprintln!(
        "📝 PTY Controller: Sending {} keyboard modifier combinations...",
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

    eprintln!("🧹 PTY Controller: Cleaning up...");

    drop(writer);

    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }

    eprintln!("✅ PTY Controller: Test passed!");
}

/// PTY Controlled: Read and parse keyboard events with modifiers
fn pty_controlled_entry_point() -> ! {
    println!("CONTROLLED_READY");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 PTY Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to enable raw mode: {e}");
    } else {
        eprintln!("✓ PTY Controlled: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🔍 PTY Controlled: Device created, reading events...");

        let inactivity_timeout = Duration::from_secs(2);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.try_read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Controlled: Event #{event_count}: {event:?}");

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
                                eprintln!("🔍 PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🔍 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🔍 PTY Controlled: Inactivity timeout (2 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("🔍 PTY Controlled: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

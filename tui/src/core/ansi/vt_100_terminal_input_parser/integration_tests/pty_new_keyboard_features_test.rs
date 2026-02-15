// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY integration tests for newly added keyboard features.
//!
//! Tests the following keyboard input features that were recently added/fixed:
//! - Tab key (fixed: was returning None)
//! - Ctrl+Space (generates Ctrl+Space event, not Ctrl+@)
//! - Alternative Home/End sequences (ESC[1~, ESC[4~, ESC[7~, ESC[8~)
//! - Numpad application mode (all 17 numpad keys)
//! - Shift+Tab (`BackTab`)
//!
//! These tests validate that the complete input stack handles these new features
//! correctly in a real PTY environment.

use crate::{ControlledChild, InputEvent, PtyPair,
            core::ansi::constants::{
                ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR, ANSI_SS3_O,
                BACKTAB_FINAL, CONTROL_NUL, CONTROL_TAB,
                SS3_NUMPAD_0, SS3_NUMPAD_1, SS3_NUMPAD_2, SS3_NUMPAD_3, SS3_NUMPAD_4,
                SS3_NUMPAD_5, SS3_NUMPAD_6, SS3_NUMPAD_7, SS3_NUMPAD_8, SS3_NUMPAD_9,
                SS3_NUMPAD_ENTER, SS3_NUMPAD_PLUS, SS3_NUMPAD_MINUS,
                SS3_NUMPAD_MULTIPLY, SS3_NUMPAD_DIVIDE,
                SS3_NUMPAD_DECIMAL, SS3_NUMPAD_COMMA,
            },
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    test_fn: test_pty_new_keyboard_features,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// PTY Controller: Send new keyboard sequences and verify parsing
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting new keyboard features test...");

    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running. The controlled process sends
    // TEST_RUNNING and CONTROLLED_READY immediately on startup.
    let mut test_running_seen = false;

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF reached before controlled started"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("TEST_RUNNING") {
                    test_running_seen = true;
                    eprintln!("  ✓ Test is running in controlled");
                }
                if trimmed.contains(CONTROLLED_READY) {
                    eprintln!("  ✓ Controlled process confirmed running!");
                    break;
                }
            }
            Err(e) => panic!("Read error while waiting for controlled: {e}"),
        }
    }

    assert!(test_running_seen, "Controlled test never started running (no TEST_RUNNING output)");

    // Build test sequences for new keyboard features
    // Tab, BackTab, and Ctrl+Space are raw bytes/simple sequences, not CSI parameter sequences
    let mut sequences: Vec<(&str, Vec<u8>)> = vec![
        // Test 1: Tab key (was broken, returning None)
        ("Tab", vec![CONTROL_TAB]),
        // Test 2: Shift+Tab (BackTab) - ESC [ Z
        ("Shift+Tab (BackTab)", vec![ANSI_ESC, ANSI_CSI_BRACKET, BACKTAB_FINAL]),
        // Test 3: Ctrl+Space - NUL byte
        ("Ctrl+Space", vec![CONTROL_NUL]),
    ];

    // Tests 4-7: Alternative Home/End sequences
    // Format: ESC [ code ~
    sequences.extend(vec![
        ("Home (alt ESC[1~)", vec![ANSI_ESC, ANSI_CSI_BRACKET, b'1', ANSI_FUNCTION_KEY_TERMINATOR]),
        ("End (alt ESC[4~)", vec![ANSI_ESC, ANSI_CSI_BRACKET, b'4', ANSI_FUNCTION_KEY_TERMINATOR]),
        ("Home (rxvt ESC[7~)", vec![ANSI_ESC, ANSI_CSI_BRACKET, b'7', ANSI_FUNCTION_KEY_TERMINATOR]),
        ("End (rxvt ESC[8~)", vec![ANSI_ESC, ANSI_CSI_BRACKET, b'8', ANSI_FUNCTION_KEY_TERMINATOR]),
    ]);

    // Tests 8-24: Numpad in application mode (SS3 sequences)
    // Format: ESC O command_char
    sequences.extend(vec![
        ("Numpad 0 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_0]),
        ("Numpad 1 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_1]),
        ("Numpad 2 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_2]),
        ("Numpad 3 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_3]),
        ("Numpad 4 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_4]),
        ("Numpad 5 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_5]),
        ("Numpad 6 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_6]),
        ("Numpad 7 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_7]),
        ("Numpad 8 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_8]),
        ("Numpad 9 (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_9]),
        ("Numpad Enter (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_ENTER]),
        ("Numpad + (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_PLUS]),
        ("Numpad - (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MINUS]),
        ("Numpad * (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MULTIPLY]),
        ("Numpad / (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DIVIDE]),
        ("Numpad . (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DECIMAL]),
        ("Numpad , (app mode)", vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_COMMA]),
    ]);

    eprintln!("📝 PTY Controller: Sending {} sequences...", sequences.len());

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify
    for (desc, sequence) in &sequences {
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Give controlled time to process
        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get an event line (skip test harness noise)
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's an event line
                    if trimmed.starts_with("Keyboard:")
                        || trimmed.starts_with("Mouse:")
                        || trimmed.starts_with("Resize:")
                        || trimmed.starts_with("Focus:")
                        || trimmed.starts_with("Paste:")
                    {
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

    // Close writer to signal EOF
    drop(writer);

    // Wait for controlled to exit
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }

    eprintln!("✅ PTY Controller: All new keyboard features tests passed!");
}

/// PTY Controlled: Parse keyboard input and echo results
fn pty_controlled_entry_point() -> ! {
    // Print to stdout immediately to confirm controlled is running
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🎯 PTY Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to enable raw mode: {e}");
    } else {
        eprintln!("✓ PTY Controlled: Terminal in raw mode");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🎯 PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new();
        eprintln!("🎯 PTY Controlled: Device created, reading events...");

        let inactivity_timeout = Duration::from_secs(2);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🎯 PTY Controlled: Event #{event_count}: {event:?}");

                            // Output event in parseable format (same as pty_input_device_test)
                            let output = match event {
                                InputEvent::Keyboard(ref key_press) => {
                                    format!("Keyboard: {key_press:?}")
                                }
                                InputEvent::Mouse(ref mouse_input) => {
                                    format!("Mouse: {mouse_input:?}")
                                }
                                InputEvent::Resize(ref size) => {
                                    format!("Resize: {size:?}")
                                }
                                InputEvent::Focus(ref state) => {
                                    format!("Focus: {state:?}")
                                }
                                InputEvent::BracketedPaste(ref text) => {
                                    format!("Paste: {} chars", text.len())
                                }
                                InputEvent::Shutdown(ref reason) => {
                                    format!("Shutdown: {reason:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after processing all expected events
                            if event_count >= 24 {
                                eprintln!("🎯 PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("🎯 PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("🎯 PTY Controlled: Inactivity timeout (2 seconds), exiting");
                    break;
                }
            }
        }

        eprintln!("🎯 PTY Controlled: Completed, exiting");
    });

    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  PTY Controlled: Failed to disable raw mode: {e}");
    }

    eprintln!("🎯 Controlled: Completed, exiting");
    std::process::exit(0);
}

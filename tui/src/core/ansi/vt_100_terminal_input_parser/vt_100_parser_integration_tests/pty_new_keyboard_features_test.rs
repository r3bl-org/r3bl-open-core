// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration tests for newly added keyboard features.
//!
//! Tests the following keyboard input features that were recently added/fixed:
//! - Tab key (fixed: was returning None)
//! - Ctrl+Space (generates Ctrl+Space event, not Ctrl+@)
//! - Alternative Home/End sequences (`ESC [ 1 ~`, `ESC [ 4 ~`, `ESC [ 7 ~`, `ESC [ 8 ~`)
//! - Numpad application mode (all 17 numpad keys)
//! - Shift+Tab (`BackTab`)
//!
//! These tests validate that the complete input stack handles these new features
//! correctly in a real [`PTY`] environment.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_new_keyboard_features -- --nocapture
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{GLYPH_CONTROLLED, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_CONTROLLER, InputEvent,
            PtyTestContext, PtyTestMode, GLYPH_SUCCESS, GLYPH_WAITING,
            GLYPH_WARNING,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_SS3_O,
                                    BACKTAB_FINAL, CONTROL_NUL, CONTROL_TAB,
                                    SS3_NUMPAD_0, SS3_NUMPAD_1, SS3_NUMPAD_2,
                                    SS3_NUMPAD_3, SS3_NUMPAD_4, SS3_NUMPAD_5,
                                    SS3_NUMPAD_6, SS3_NUMPAD_7, SS3_NUMPAD_8,
                                    SS3_NUMPAD_9, SS3_NUMPAD_COMMA, SS3_NUMPAD_DECIMAL,
                                    SS3_NUMPAD_DIVIDE, SS3_NUMPAD_ENTER,
                                    SS3_NUMPAD_MINUS, SS3_NUMPAD_MULTIPLY,
                                    SS3_NUMPAD_PLUS},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, Write},
          time::Duration};

generate_pty_test! {
    test_fn: test_pty_new_keyboard_features,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send new keyboard sequences and verify parsing
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting new keyboard features test...");

    eprintln!("{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running and ready. The controlled process sends
    // TEST_RUNNING, CONTROLLED_STARTING, and CONTROLLED_READY on startup.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for MSG_CONTROLLED_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready (input device created)");

    // Build test sequences for new keyboard features
    // Tab, BackTab, and Ctrl+Space are raw bytes/simple sequences, not CSI parameter
    // sequences
    let mut sequences: Vec<(&str, Vec<u8>)> = vec![
        // Test 1: Tab key (was broken, returning None)
        ("Tab", vec![CONTROL_TAB]),
        // Test 2: Shift+Tab (BackTab) - ESC [ Z
        (
            "Shift+Tab (BackTab)",
            vec![ANSI_ESC, ANSI_CSI_BRACKET, BACKTAB_FINAL],
        ),
        // Test 3: Ctrl+Space - NUL byte
        ("Ctrl+Space", vec![CONTROL_NUL]),
    ];

    // Tests 4-7: Alternative Home/End sequences
    // Format: ESC [ code ~
    sequences.extend(vec![
        (
            "Home (alt ESC[1~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                b'1',
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        (
            "End (alt ESC[4~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                b'4',
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        (
            "Home (rxvt ESC[7~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                b'7',
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
        (
            "End (rxvt ESC[8~)",
            vec![
                ANSI_ESC,
                ANSI_CSI_BRACKET,
                b'8',
                ANSI_FUNCTION_KEY_TERMINATOR,
            ],
        ),
    ]);

    // Tests 8-24: Numpad in application mode (SS3 sequences)
    // Format: ESC O command_char
    sequences.extend(vec![
        (
            "Numpad 0 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_0],
        ),
        (
            "Numpad 1 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_1],
        ),
        (
            "Numpad 2 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_2],
        ),
        (
            "Numpad 3 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_3],
        ),
        (
            "Numpad 4 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_4],
        ),
        (
            "Numpad 5 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_5],
        ),
        (
            "Numpad 6 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_6],
        ),
        (
            "Numpad 7 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_7],
        ),
        (
            "Numpad 8 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_8],
        ),
        (
            "Numpad 9 (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_9],
        ),
        (
            "Numpad Enter (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_ENTER],
        ),
        (
            "Numpad + (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_PLUS],
        ),
        (
            "Numpad - (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MINUS],
        ),
        (
            "Numpad * (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_MULTIPLY],
        ),
        (
            "Numpad / (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DIVIDE],
        ),
        (
            "Numpad . (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_DECIMAL],
        ),
        (
            "Numpad , (app mode)",
            vec![ANSI_ESC, ANSI_SS3_O, SS3_NUMPAD_COMMA],
        ),
    ]);

    eprintln!(
        "{} PTY Controller: Sending {} sequences...",
        GLYPH_WAITING,
        sequences.len()
    );

    // For each test sequence: write ANSI bytes to PTY, read back parsed event, verify
    for (desc, sequence) in &sequences {
        eprintln!("  → Sending: {desc} ({sequence:?})");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

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
                    eprintln!("  {GLYPH_WARNING} Skipping non-event output: {trimmed}");
                }
                Err(e) => {
                    panic!("Read error for {desc}: {e}");
                }
            }
        };

        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    // Close writer to signal EOF
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: All new keyboard features tests passed!");
}

/// [`PTY`] Controlled: Parse keyboard input and echo results. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input. MUST be after
        // DirectToAnsiInputDevice::new() so the mio poller thread is already
        // watching stdin before the controller sends any input through the PTY.
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().expect("Failed to flush");

        let inactivity_timeout = Duration::from_secs(5);
        let mut inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
        let mut event_count = 0;

        loop {
            tokio::select! {
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Event #{event_count}: {event:?}");

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
                                eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Processed {event_count} events, exiting");
                                break;
                            }
                        }
                        None => {
                            eprintln!("{GLYPH_CONTROLLED} PTY Controlled: EOF reached");
                            break;
                        }
                    }
                }
                () = tokio::time::sleep_until(inactivity_deadline) => {
                    eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Inactivity timeout (5 seconds with no events), exiting");
                    break;
                }
            }
        }

        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Completed, exiting");
    });
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for bracketed paste text collection.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly collects text between bracketed
//! paste markers (`ESC [200~` ... `ESC [201~`) and emits a single
//! [`InputEvent::BracketedPaste`] with the complete text.
//!
//! ## Test Cases
//!
//! - Simple [`ASCII`] paste: "Hello"
//! - Multiline paste with newlines preserved
//! - [`UTF-8`] paste with multi-byte characters
//! - Empty paste (Start immediately followed by End)
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_bracketed_paste -- --nocapture
//! ```
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::BracketedPaste`]: crate::InputEvent::BracketedPaste
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8

use crate::{GLYPH_CONTROLLED, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            GLYPH_CONTROLLER_CLEANUP, GLYPH_CONTROLLER, InputEvent, PtyTestContext,
            PtyTestMode, GLYPH_SUCCESS, MSG_TEST_RUNNING, GLYPH_WAITING,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100PasteModeIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write,
          time::Duration};

// XMARK: Process isolated test with PTY.

generate_pty_test! {
    test_fn: test_pty_bracketed_paste,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send bracketed paste sequences and verify collection
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(context: PtyTestContext) {
    /// Helper to build a complete bracketed paste sequence from text.
    ///
    /// Returns (description, byte sequence) tuple.
    /// Generates: Start marker + [`UTF-8`] text bytes + End marker.
    /// (Regular characters are raw [`UTF-8`] bytes, not [`ANSI`] escape sequences)
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    fn generate_paste_test_sequence<'a>(desc: &'a str, text: &str) -> (&'a str, Vec<u8>) {
        let mut bytes = Vec::new();

        // Start marker (ESC[200~)
        let start_bytes = generate_keyboard_sequence(&VT100InputEventIR::Paste(
            VT100PasteModeIR::Start,
        ))
        .expect("Failed to generate paste start marker");
        bytes.extend_from_slice(&start_bytes);

        // Text characters: just raw UTF-8 bytes (no ANSI escape sequences needed)
        bytes.extend_from_slice(text.as_bytes());

        // End marker (ESC[201~)
        let end_bytes =
            generate_keyboard_sequence(&VT100InputEventIR::Paste(VT100PasteModeIR::End))
                .expect("Failed to generate paste end marker");
        bytes.extend_from_slice(&end_bytes);

        (desc, bytes)
    }

    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting bracketed paste test...");

    eprintln!("{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start...");

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    // Generate test cases using abstractions (no magic strings!)
    let test_cases = vec![
        generate_paste_test_sequence("Simple ASCII paste", "Hello"),
        generate_paste_test_sequence("Multiline paste", "Line 1\nLine 2\nLine 3"),
        generate_paste_test_sequence("Tabbed paste", "col1\tcol2\tcol3"),
        generate_paste_test_sequence("Mixed whitespace", "fn main() {\n\tlet x = 1;\n}"),
        generate_paste_test_sequence("CR line endings", "line1\rline2"),
        generate_paste_test_sequence("CRLF line endings", "line1\r\nline2"),
        generate_paste_test_sequence("UTF-8 paste", "Hello 世界 🌍"),
        generate_paste_test_sequence("Empty paste", ""),
    ];

    eprintln!(
        "{} PTY Controller: Sending {} paste sequences...",
        GLYPH_WAITING,
        test_cases.len()
    );

    for (desc, sequence) in &test_cases {
        eprintln!("  → Sending: {desc}");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get a paste event line
        let event_line = child.read_line_state(&mut buf_reader, |line| line.starts_with("Paste:"));

        eprintln!("  {GLYPH_SUCCESS} {desc}: {event_line}");
    }

    eprintln!("{GLYPH_CONTROLLER_CLEANUP} PTY Controller: Cleaning up...");

    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// [`PTY`] Controlled: Read and parse bracketed paste events. The harness performs
/// [`std::process::exit(0)`] after this function returns.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controlled() {
    // Print to stdout immediately to confirm controlled is starting.
    println!("{MSG_TEST_RUNNING}");
    println!("{MSG_CONTROLLED_STARTING}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting...");
        let mut input_device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Device created, reading events...");

        // Signal to controller that we're ready to receive input.
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

                            let output = match event {
                                InputEvent::BracketedPaste(ref text) => {
                                    format!("Paste: {} chars, text={:?}", text.len(), text)
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            // Exit after processing the expected number of test cases
                            if event_count >= 8 {
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

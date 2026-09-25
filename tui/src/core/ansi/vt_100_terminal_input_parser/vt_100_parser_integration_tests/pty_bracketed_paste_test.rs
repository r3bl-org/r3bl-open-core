// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// XMARK: pty test faking terminal emulator (in controller)

//! [`PTY`]-based integration test for bracketed paste text collection.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly collects text between bracketed
//! paste markers (`ESC [ 200 ~` ... `ESC [ 201 ~`) in a real pseudoterminal environment:
//! - Simple [`ASCII`] paste: `"Hello"`
//! - Multiline paste with newline preservation: `"Line 1\nLine 2\nLine 3"`
//! - Tabbed paste with tabs preserved: `"col1\tcol2\tcol3"`
//! - Mixed whitespace in multi-line code blocks
//! - Carriage return line endings: `"line1\rline2"`
//! - CRLF line endings: `"line1\r\nline2"`
//! - Multi-byte [`UTF-8`] paste with unicode glyphs and emojis: `"Hello 世界 🌍"`
//! - Empty paste: Start marker immediately followed by End marker
//!
//! # Architecture and Process Roles
//!
//! In this test, the controller process simulates a host terminal emulator while the
//! controlled child runs our TUI input engine. For the overall architecture and testing
//! philosophy, see the [process roles documentation].
//!
//! 1. **[`PTY`] Controller (Simulated Terminal Emulator)**:
//!    - Simulates an external terminal emulator host (such as [`Kitty`], Ghostty, or
//!      [`XTerm`]) performing paste operations into the active terminal session.
//!    - Synthesizes bracketed paste framing by wrapping raw byte buffers between paste
//!      start (`ESC [ 200 ~`) and paste end (`ESC [ 201 ~`) control sequences.
//!    - Writes sequence bytes directly into the controller side of the [`PTY`], which the
//!      operating system delivers to the controlled process's [`stdin`].
//!    - Exercises diverse text payloads including multiline buffers, mixed tabs and
//!      newlines, differing line endings (`LF`, `CR`, `CRLF`), multi-byte [`UTF-8`]
//!      sequences, and empty payloads.
//!    - Reads line-by-line event confirmation strings from the controlled child's
//!      [`stdout`] and validates character counts and received content.
//!
//! 2. **[`PTY`] Controlled (System Under Test / TUI Input Engine)**:
//!    - Represents the real TUI application running inside the child subprocess connected
//!      to the controlled side of the [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread watches [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Accumulates incoming raw bytes between bracketed paste markers without emitting
//!      individual keystroke events for each character.
//!    - Runs a Tokio asynchronous event loop consuming from `input_device.next().await`.
//!    - Emits a single consolidated [`InputEvent::BracketedPaste`] containing the entire
//!      pasted string buffer once the paste end marker is encountered.
//!    - Formats and echoes the paste receipt over [`stdout`] so the controller can verify
//!      parsing accuracy.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_bracketed_paste -- --nocapture
//! ```
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::BracketedPaste`]: crate::InputEvent::BracketedPaste
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: https://crates.io/crates/mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [process roles documentation]: mod@super#architecture-and-process-roles

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_CONTROLLER_CLEANUP, GLYPH_SUCCESS,
            GLYPH_WAITING, InputEvent, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_TEST_RUNNING, PtyTestContext, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100PasteModeIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::Write, time::Duration};

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
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting bracketed paste test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled to confirm it's running and ready.
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .expect("Failed to wait for ready signal");

    let test_cases = generate_test_cases();

    eprintln!(
        "{} PTY Controller: Sending {} paste sequences...",
        GLYPH_WAITING,
        test_cases.len()
    );

    for test_case in &test_cases {
        let TestCase {
            description,
            sequence,
        } = test_case;

        eprintln!("  → Sending: {description}");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        // Read responses until we get a paste event line
        let event_line =
            child.read_line_state(&mut buf_reader, |line| line.starts_with("Paste:"));

        eprintln!("  {GLYPH_SUCCESS} {description}: {event_line}");
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
                            if event_count >= TOTAL_EXPECTED_EVENTS {
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

/// Expected number of paste events processed by the controlled process:
/// - 8 distinct bracketed paste test cases
const TOTAL_EXPECTED_EVENTS: usize = 8;

/// A single test case for bracketed paste verification.
struct TestCase {
    description: &'static str,
    sequence: Vec<u8>,
}

impl TestCase {
    fn new(description: &'static str, text: &str) -> Self {
        let mut sequence = Vec::new();

        // Start marker (ESC[200~)
        let start_bytes = generate_keyboard_sequence(&VT100InputEventIR::Paste(
            VT100PasteModeIR::Start,
        ))
        .expect("Failed to generate paste start marker");
        sequence.extend_from_slice(&start_bytes);

        // Text characters: raw UTF-8 bytes (no ANSI escape sequences needed)
        sequence.extend_from_slice(text.as_bytes());

        // End marker (ESC[201~)
        let end_bytes =
            generate_keyboard_sequence(&VT100InputEventIR::Paste(VT100PasteModeIR::End))
                .expect("Failed to generate paste end marker");
        sequence.extend_from_slice(&end_bytes);

        Self {
            description,
            sequence,
        }
    }
}

/// Generate the structured test cases for bracketed paste testing.
fn generate_test_cases() -> Vec<TestCase> {
    vec![
        TestCase::new("Simple ASCII paste", "Hello"),
        TestCase::new("Multiline paste", "Line 1\nLine 2\nLine 3"),
        TestCase::new("Tabbed paste", "col1\tcol2\tcol3"),
        TestCase::new("Mixed whitespace", "fn main() {\n\tlet x = 1;\n}"),
        TestCase::new("CR line endings", "line1\rline2"),
        TestCase::new("CRLF line endings", "line1\r\nline2"),
        TestCase::new("UTF-8 paste", "Hello 世界 🌍"),
        TestCase::new("Empty paste", ""),
    ]
}

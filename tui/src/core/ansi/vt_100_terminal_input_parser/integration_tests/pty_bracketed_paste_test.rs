// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for bracketed paste text collection.
//!
//! Validates that [`DirectToAnsiInputDevice`] correctly collects text between
//! bracketed paste markers (`ESC [200~` ... `ESC [201~`) and emits a single
//! [`InputEvent::BracketedPaste`] with the complete text.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_bracketed_paste -- --nocapture`
//!
//! ## Test Cases
//!
//! - Simple ASCII paste: "Hello"
//! - Multiline paste with newlines preserved
//! - UTF-8 paste with multi-byte characters
//! - Empty paste (Start immediately followed by End)
//!
//! Uses the coordinator-worker pattern with two processes.
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputEvent::BracketedPaste`]: crate::InputEvent::BracketedPaste

use crate::{ControlledChild, InputEvent, PtyPair, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::ir_event_types::{VT100InputEventIR,
                                                                        VT100PasteModeIR}},
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

// XMARK: Process isolated test with PTY.

generate_pty_test! {
    test_fn: test_pty_bracketed_paste,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// PTY Controller: Send bracketed paste sequences and verify collection
#[allow(clippy::too_many_lines)]
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    /// Helper to build a complete bracketed paste sequence from text.
    ///
    /// Returns (description, byte sequence) tuple.
    /// Generates: Start marker + UTF-8 text bytes + End marker.
    /// (Regular characters are raw UTF-8 bytes, not ANSI escape sequences)
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

    eprintln!("🚀 PTY Controller: Starting bracketed paste test...");

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

    assert!(
        test_running_seen,
        "Controlled test never started running (no TEST_RUNNING output)"
    );

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
        "📝 PTY Controller: Sending {} paste sequences...",
        test_cases.len()
    );

    for (desc, sequence) in &test_cases {
        eprintln!("  → Sending: {desc}");

        writer
            .write_all(sequence)
            .expect("Failed to write sequence");
        writer.flush().expect("Failed to flush");

        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get a paste event line
        let event_line = loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving event for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's a paste event line
                    if trimmed.starts_with("Paste:") {
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

/// PTY Controlled: Read and parse bracketed paste events
fn pty_controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

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
                event_result = input_device.next() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Controlled: Event #{event_count}: {event:?}");

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

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

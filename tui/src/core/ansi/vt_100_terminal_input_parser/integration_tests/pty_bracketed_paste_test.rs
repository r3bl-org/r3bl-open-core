// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{core::ansi::vt_100_terminal_input_parser::{
                test_fixtures::generate_keyboard_sequence,
                types::{VT100InputEvent, VT100PasteMode}
            },
            Deadline, generate_pty_test, InputEvent,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

// XMARK: Process isolated test functions using env vars & PTY.

generate_pty_test! {
    /// PTY-based integration test for bracketed paste text collection.
    ///
    /// Validates that [`DirectToAnsiInputDevice`] correctly collects text between
    /// bracketed paste markers (ESC[200~ ... ESC[201~) and emits a single
    /// [`InputEvent::BracketedPaste`] with the complete text.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_pty_bracketed_paste -- --nocapture`
    ///
    /// ## Test Cases
    ///
    /// - Simple ASCII paste: "Hello"
    /// - Multiline paste with newlines preserved
    /// - UTF-8 paste with multi-byte characters
    /// - Empty paste (Start immediately followed by End)
    ///
    /// Uses the coordinator-worker pattern with two processes.
    ///
    /// [`DirectToAnsiInputDevice`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice
    /// [`InputEvent::BracketedPaste`]: crate::InputEvent::BracketedPaste
    test_fn: test_pty_bracketed_paste,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// PTY Master: Send bracketed paste sequences and verify collection
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    /// Helper to build a complete bracketed paste sequence from text.
    ///
    /// Returns (description, byte sequence) tuple.
    /// Generates: Start marker + UTF-8 text bytes + End marker.
    /// (Regular characters are raw UTF-8 bytes, not ANSI escape sequences)
    fn generate_paste_test_sequence<'a>(desc: &'a str, text: &str) -> (&'a str, Vec<u8>) {
        let mut bytes = Vec::new();

        // Start marker (ESC[200~)
        let start_bytes = generate_keyboard_sequence(&VT100InputEvent::Paste(VT100PasteMode::Start))
            .expect("Failed to generate paste start marker");
        bytes.extend_from_slice(&start_bytes);

        // Text characters: just raw UTF-8 bytes (no ANSI escape sequences needed)
        bytes.extend_from_slice(text.as_bytes());

        // End marker (ESC[201~)
        let end_bytes = generate_keyboard_sequence(&VT100InputEvent::Paste(VT100PasteMode::End))
            .expect("Failed to generate paste end marker");
        bytes.extend_from_slice(&end_bytes);

        (desc, bytes)
    }

    eprintln!("🚀 PTY Master: Starting bracketed paste test...");

    let mut writer = pty_pair.master.take_writer().expect("Failed to get writer");
    let reader_non_blocking = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader_non_blocking = BufReader::new(reader_non_blocking);

    eprintln!("📝 PTY Master: Waiting for slave to start...");

    // Wait for slave to confirm it's running.
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

    // Generate test cases using abstractions (no magic strings!)
    let test_cases = vec![
        generate_paste_test_sequence("Simple ASCII paste", "Hello"),
        generate_paste_test_sequence("Multiline paste", "Line 1\nLine 2\nLine 3"),
        generate_paste_test_sequence("UTF-8 paste", "Hello 世界 🌍"),
        generate_paste_test_sequence("Empty paste", ""),
    ];

    eprintln!("📝 PTY Master: Sending {} paste sequences...", test_cases.len());

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
            match buf_reader_non_blocking.read_line(&mut line) {
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

/// PTY Slave: Read and parse bracketed paste events
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
                event_result = input_device.try_read_event() => {
                    match event_result {
                        Some(event) => {
                            event_count += 1;
                            inactivity_deadline = tokio::time::Instant::now() + inactivity_timeout;
                            eprintln!("🔍 PTY Slave: Event #{event_count}: {event:?}");

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
                            if event_count >= 4 {
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

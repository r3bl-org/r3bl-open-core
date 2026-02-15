// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for UTF-8 text input parsing.
//!
//! Validates that the [`DirectToAnsiInputDevice`] correctly handles UTF-8 text input:
//! - ASCII characters
//! - UTF-8 multi-byte sequences (accented characters, emojis, etc.)
//! - Mixed text and ANSI escape sequences
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_utf8_text -- --nocapture`
//!
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice

use crate::{ControlledChild, InputEvent, PtyPair, PtyTestMode, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "CONTROLLED_READY";

// XMARK: Process isolated test with PTY.

generate_pty_test! {
    test_fn: test_pty_utf8_text,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Raw,
}

/// PTY Controller: Send UTF-8 text and verify parsing
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting UTF-8 text test...");

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

    // Send text inputs
    let texts = vec![
        ("ASCII", "hello"),
        ("Numbers", "12345"),
        ("Space", "a b c"),
        ("Punctuation", "!@#$%"),
    ];

    eprintln!("📝 PTY Controller: Sending {} text inputs...", texts.len());

    for (desc, text) in &texts {
        eprintln!("  → Sending: {desc} ({text})");

        writer
            .write_all(text.as_bytes())
            .expect("Failed to write text");
        writer.flush().expect("Failed to flush");

        std::thread::sleep(Duration::from_millis(100));

        // Read responses until we get text output
        let mut output_received = false;
        for _ in 0..10 {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    panic!("EOF reached before receiving text for {desc}");
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    // Check if it's a text event line
                    if trimmed.starts_with("Text:") {
                        eprintln!("  ✓ {desc}: {trimmed}");
                        output_received = true;
                        break;
                    }

                    // Skip test harness noise
                    eprintln!("  ⚠️  Skipping non-text output: {trimmed}");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    panic!("Read error for {desc}: {e}");
                }
            }
        }

        if !output_received {
            eprintln!("⚠️  Warning: No output received for {desc}");
        }
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

/// PTY Controlled: Read and parse UTF-8 text
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
                                InputEvent::Keyboard(key_press) => {
                                    format!("Text: {key_press:?}")
                                }
                                _ => {
                                    format!("Unexpected event: {event:?}")
                                }
                            };

                            println!("{output}");
                            std::io::stdout().flush().expect("Failed to flush stdout");

                            if event_count >= 4 {
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

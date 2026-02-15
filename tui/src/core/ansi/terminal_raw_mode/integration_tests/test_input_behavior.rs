// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test 3: Input behavior verification.
//!
//! Tests that raw mode *actually works* for input processing - not just that
//! flags are set correctly. Verifies character-by-character reading without
//! buffering, echo, or signal interpretation.

use crate::{ControlledChild, PtyPair, PtyTestMode, generate_pty_test, ANSI_ESC, CONTROL_C,
            CONTROL_D, CONTROL_LF};
use std::{io::{BufRead, BufReader, Read, Write},
          time::Duration};

generate_pty_test! {
    /// PTY-based integration test for raw mode input behavior.
    ///
    /// Sends various input sequences from master to slave and verifies:
    /// 1. Characters arrive immediately (no line buffering)
    /// 2. Control characters pass through as bytes (e.g., Ctrl+C = 0x03)
    /// 3. No echo (typed characters don't appear in output)
    ///
    /// This is the most important test as it validates actual terminal behavior,
    /// not just configuration settings.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_raw_mode_input_behavior -- --nocapture`
    test_fn: test_raw_mode_input_behavior,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Cooked,
}

/// Controller process: sends input and verifies controlled process reports correct bytes.
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting input behavior test...");

    let mut writer = pty_pair.controller().take_writer().expect("Failed to get writer");
    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process to be ready...");

    // Wait for controlled process to signal ready
    let controlled_ready = loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF before controlled process ready"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");
                if trimmed.contains("SLAVE_READY") {
                    eprintln!("  ✓ Controlled process is ready");
                    break true;
                }
                assert!(!trimmed.contains("FAILED:"), "Test failed: {trimmed}");
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error: {e}"),
        }
    };

    assert!(controlled_ready, "Controlled process did not become ready");

    // Helper to read response from controlled process
    let mut read_response = || -> String {
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => panic!("EOF while reading response"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("RECEIVED:") || trimmed.contains("FAILED:") {
                        return trimmed.to_string();
                    }
                    eprintln!("  ⚠️  Skipping: {trimmed}");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("Read error: {e}"),
            }
        }
    };

    // Test 1: Send single character 'a'
    eprintln!("📝 PTY Controller: Test 1 - Single character 'a'...");
    writer.write_all(b"a").expect("Failed to write 'a'");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let response = read_response();
    eprintln!("  ← Controlled response: {response}");
    assert_eq!(response, "RECEIVED: 0x61 ('a')", "Expected to receive 'a'");

    // Test 2: Send Ctrl+C (should be 0x03, not trigger signal)
    eprintln!("📝 PTY Controller: Test 2 - Ctrl+C (should be 0x03)...");
    writer.write_all(&[CONTROL_C]).expect("Failed to write Ctrl+C");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let response = read_response();
    eprintln!("  ← Controlled response: {response}");
    assert_eq!(
        response, "RECEIVED: 0x03 ('^C')",
        "Expected Ctrl+C as 0x03, not signal"
    );

    // Test 3: Send Ctrl+D (should be 0x04, not EOF)
    eprintln!("📝 PTY Controller: Test 3 - Ctrl+D (should be 0x04)...");
    writer.write_all(&[CONTROL_D]).expect("Failed to write Ctrl+D");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let response = read_response();
    eprintln!("  ← Controlled response: {response}");
    assert_eq!(
        response, "RECEIVED: 0x04 ('^D')",
        "Expected Ctrl+D as 0x04, not EOF"
    );

    // Test 4: Send newline (should be 0x0A, not trigger line buffering)
    eprintln!("📝 PTY Controller: Test 4 - Newline (should be 0x0A)...");
    writer.write_all(&[CONTROL_LF]).expect("Failed to write newline");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    let response = read_response();
    eprintln!("  ← Controlled response: {response}");
    assert_eq!(
        response, "RECEIVED: 0x0a ('\\n')",
        "Expected newline as 0x0A"
    );

    // Signal controlled process to exit
    eprintln!("📝 PTY Controller: Signaling controlled process to exit...");
    writer.write_all(&[ANSI_ESC]).expect("Failed to write ESC");
    writer.flush().expect("Failed to flush");
    std::thread::sleep(Duration::from_millis(100));

    drop(writer);

    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }

    eprintln!("✅ PTY Controller: Input behavior test passed!");
}

/// Controlled process: enables raw mode and reads input byte-by-byte.
fn pty_controlled_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Enabling raw mode...");

    // Enable raw mode
    if let Err(e) = crate::enable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed to enable raw mode: {e}");
        println!("FAILED: Could not enable raw mode");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    eprintln!("✓ Controlled: Raw mode enabled, ready to read input");
    println!("SLAVE_READY");
    std::io::stdout().flush().expect("Failed to flush");

    let mut stdin = std::io::stdin();
    let mut buffer = [0u8; 1];

    // Read input byte-by-byte
    loop {
        match stdin.read_exact(&mut buffer) {
            Ok(()) => {
                let byte = buffer[0];
                eprintln!("  🔍 Controlled: Read byte: 0x{byte:02x}");

                // ESC (0x1B) signals exit
                if byte == ANSI_ESC {
                    eprintln!("  ✓ Controlled: Received ESC, exiting");
                    break;
                }

                // Report what we received
                let display = match byte {
                    CONTROL_C => "'^C'".to_string(),
                    CONTROL_D => "'^D'".to_string(),
                    CONTROL_LF => "'\\n'".to_string(),
                    b'\r' => "'\\r'".to_string(),
                    b if b.is_ascii_graphic() || b == b' ' => {
                        format!("'{}'", char::from(b))
                    }
                    _ => format!("0x{byte:02x}"),
                };

                println!("RECEIVED: 0x{byte:02x} ({display})");
                std::io::stdout().flush().expect("Failed to flush");
            }
            Err(e) => {
                eprintln!("⚠️  Controlled: Read error: {e}");
                println!("FAILED: Read error");
                std::io::stdout().flush().expect("Failed to flush");
                break;
            }
        }
    }

    if let Err(e) = crate::disable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

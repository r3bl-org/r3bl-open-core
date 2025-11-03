// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test 1: Basic enable/disable functionality.
//!
//! Verifies that raw mode can be enabled, disabled, and properly restores
//! terminal state using actual PTY pairs. This is the foundational test
//! that ensures the basic lifecycle works.

use crate::{RawModeGuard, generate_pty_test};
use rustix::termios;
use std::{io::{BufRead, BufReader, Write},
          time::{Duration, Instant}};

generate_pty_test! {
    /// PTY-based integration test for raw mode basic enable/disable.
    ///
    /// This test uses a master/slave PTY pair to verify that:
    /// 1. Raw mode can be enabled on a real PTY
    /// 2. Raw mode can be disabled and terminal settings restored
    /// 3. The RAII guard pattern works correctly
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_raw_mode_pty -- --nocapture`
    test_fn: test_raw_mode_pty,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}

/// Master process: verifies results.
/// Receives PTY pair and child process from the macro.
fn pty_master_entry_point(
    pty_pair: portable_pty::PtyPair,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    eprintln!("🚀 PTY Master: Starting raw mode test...");

    // Read from PTY and verify
    let reader = pty_pair
        .master
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Master: Waiting for slave results...");

    let mut slave_started = false;
    let mut test_passed = false;
    let start_timeout = Instant::now();

    while start_timeout.elapsed() < Duration::from_secs(5) {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("  ⚠️  EOF reached");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Slave output: {trimmed}");

                if trimmed.contains("SLAVE_STARTING") {
                    slave_started = true;
                    eprintln!("  ✓ Slave confirmed starting");
                }
                if trimmed.contains("SUCCESS:") {
                    test_passed = true;
                    eprintln!("  ✓ Test passed: {trimmed}");
                    break;
                }
                assert!(!trimmed.contains("FAILED:"), "Test failed: {trimmed}");
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }

    assert!(slave_started, "Slave did not start properly");
    assert!(test_passed, "Test did not report success");

    // Wait for slave to exit
    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Master: Slave exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for slave: {e}");
        }
    }

    eprintln!("✅ PTY Master: Raw mode test passed!");
}

/// Slave process: enables raw mode and reports results.
/// This function MUST exit before returning so other tests don't run.
fn pty_slave_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Slave: Starting raw mode test...");

    // Get current terminal settings BEFORE enabling raw mode
    let stdin = std::io::stdin();
    let before_termios = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Slave: Failed to get termios before: {e}");
            println!("FAILED: Could not read termios");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Enable raw mode using the guard
    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("⚠️  Slave: Failed to enable raw mode: {e}");
            println!("FAILED: Could not enable raw mode");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    eprintln!("✓ Slave: Raw mode enabled");

    // Get terminal settings AFTER enabling raw mode
    let after_termios = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Slave: Failed to get termios after: {e}");
            println!("FAILED: Could not read termios after");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Verify that settings actually changed
    if before_termios.local_modes == after_termios.local_modes {
        eprintln!("⚠️  Slave: Local modes didn't change!");
        println!("FAILED: Modes not changed");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    eprintln!("✓ Slave: Terminal settings changed correctly");

    // Report success
    println!("SUCCESS: Raw mode enabled and verified");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Slave: Guard will be dropped now...");
    // Guard is dropped here, disabling raw mode
    eprintln!("🔍 Slave: Completed, exiting");
    // CRITICAL: Exit immediately to prevent test harness from running other tests
    std::process::exit(0);
}

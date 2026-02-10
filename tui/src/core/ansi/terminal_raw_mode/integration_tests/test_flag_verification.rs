// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test 2: Termios flag verification.
//!
//! Verifies that raw mode sets the *correct* termios flags, not just that
//! *something* changed. This test documents the exact contract of raw mode
//! and catches regressions in flag handling.

use crate::{ControlledChild, PtyPair, RawModeGuard, VMIN_RAW_MODE, VTIME_RAW_MODE,
            drain_pty_and_wait, generate_pty_test};
use rustix::termios::{self, ControlModes, InputModes, LocalModes, OutputModes, SpecialCodeIndex};
use std::{io::{BufRead, BufReader, Write},
          time::{Duration, Instant}};

generate_pty_test! {
    /// PTY-based integration test for raw mode flag verification.
    ///
    /// Verifies that `make_raw()` sets the correct termios flags according to
    /// the POSIX `cfmakeraw` specification. This ensures our implementation
    /// matches crossterm and standard raw mode behavior.
    ///
    /// Checks:
    /// - Input modes: ICANON, ECHO, ISIG, IEXTEN disabled
    /// - Output modes: OPOST disabled
    /// - Control modes: CS8 set, 8-bit characters
    /// - Special codes: VMIN=1, VTIME=0 (byte-by-byte, no timeout)
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_raw_mode_flags -- --nocapture`
    test_fn: test_raw_mode_flags,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point
}

/// Controller process: verifies that controlled process reports correct flags.
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting flag verification test...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process flag checks...");

    let mut controlled_started = false;
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
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains("SLAVE_STARTING") {
                    controlled_started = true;
                    eprintln!("  ✓ Controlled process confirmed starting");
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

    assert!(controlled_started, "Controlled process did not start properly");
    assert!(test_passed, "Test did not report success");

    // Drain PTY and wait for child to prevent macOS PTY buffer deadlock.
    drain_pty_and_wait(buf_reader, pty_pair, &mut child);

    eprintln!("✅ PTY Controller: Flag verification test passed!");
}

/// Controlled process: enables raw mode and verifies specific termios flags.
fn pty_controlled_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Starting flag verification...");

    let stdin = std::io::stdin();

    // Enable raw mode using the guard
    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("⚠️  Controlled: Failed to enable raw mode: {e}");
            println!("FAILED: Could not enable raw mode");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    eprintln!("✓ Controlled: Raw mode enabled, checking flags...");

    // Get terminal settings after enabling raw mode
    let termios = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Controlled: Failed to get termios: {e}");
            println!("FAILED: Could not read termios");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Verify Local Modes (ICANON, ECHO, ISIG, IEXTEN should be OFF)
    if termios.local_modes.contains(LocalModes::ICANON) {
        eprintln!("⚠️  Controlled: ICANON is still ON (should be OFF)");
        println!("FAILED: ICANON not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ ICANON is OFF (no line buffering)");

    if termios.local_modes.contains(LocalModes::ECHO) {
        eprintln!("⚠️  Controlled: ECHO is still ON (should be OFF)");
        println!("FAILED: ECHO not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ ECHO is OFF (no character echo)");

    if termios.local_modes.contains(LocalModes::ISIG) {
        eprintln!("⚠️  Controlled: ISIG is still ON (should be OFF)");
        println!("FAILED: ISIG not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ ISIG is OFF (no signal generation)");

    if termios.local_modes.contains(LocalModes::IEXTEN) {
        eprintln!("⚠️  Controlled: IEXTEN is still ON (should be OFF)");
        println!("FAILED: IEXTEN not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ IEXTEN is OFF (no extended processing)");

    // Verify Output Modes (OPOST should be OFF)
    if termios.output_modes.contains(OutputModes::OPOST) {
        eprintln!("⚠️  Controlled: OPOST is still ON (should be OFF)");
        println!("FAILED: OPOST not disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ OPOST is OFF (no output processing)");

    // Verify Control Modes (CS8 should be set for 8-bit characters)
    if !termios.control_modes.contains(ControlModes::CS8) {
        eprintln!("⚠️  Controlled: CS8 is not set (should be ON)");
        println!("FAILED: CS8 not enabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ CS8 is ON (8-bit characters)");

    // Verify Input Modes (common flags should be OFF)
    let unwanted_input_flags = InputModes::ICRNL
        | InputModes::INLCR
        | InputModes::IGNCR
        | InputModes::IXON
        | InputModes::ISTRIP;

    if termios.input_modes.intersects(unwanted_input_flags) {
        eprintln!("⚠️  Controlled: Unwanted input modes still set");
        println!("FAILED: Input modes not properly disabled");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ Input processing modes disabled (ICRNL, IXON, etc.)");

    // Verify Special Codes (VMIN=1, VTIME=0 for byte-by-byte reading)
    let vmin = termios.special_codes[SpecialCodeIndex::VMIN];
    let vtime = termios.special_codes[SpecialCodeIndex::VTIME];

    if vmin != VMIN_RAW_MODE {
        eprintln!("⚠️  Controlled: VMIN={vmin} (expected {VMIN_RAW_MODE})");
        println!("FAILED: VMIN not set to {VMIN_RAW_MODE}");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ VMIN={VMIN_RAW_MODE} (return after 1 byte)");

    if vtime != VTIME_RAW_MODE {
        eprintln!("⚠️  Controlled: VTIME={vtime} (expected {VTIME_RAW_MODE})");
        println!("FAILED: VTIME not set to {VTIME_RAW_MODE}");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ VTIME={VTIME_RAW_MODE} (no timeout)");

    // All checks passed!
    println!("SUCCESS: All termios flags verified");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

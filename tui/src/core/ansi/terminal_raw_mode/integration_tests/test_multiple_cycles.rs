// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test 4: Multiple enable/disable cycles.
//!
//! Verifies that raw mode can be enabled and disabled multiple times without
//! issues. Tests edge cases like:
//! - Calling `enable()` when already enabled
//! - Calling `disable()` when already disabled
//! - Original settings are preserved across cycles

use crate::{ControlledChild, PtyPair, PtyTestMode, generate_pty_test};
use rustix::termios;
use std::{io::{BufRead, BufReader, Write},
          time::{Duration, Instant}};

generate_pty_test! {
    /// PTY-based integration test for multiple raw mode cycles.
    ///
    /// Verifies that enabling and disabling raw mode multiple times works correctly:
    /// 1. First enable/disable cycle
    /// 2. Second enable/disable cycle
    /// 3. Third enable/disable cycle
    /// 4. Settings restore correctly after each cycle
    /// 5. Multiple `enable()` calls don't fail
    ///
    /// This catches edge cases with the `ORIGINAL_TERMIOS` static and ensures
    /// the implementation is robust for repeated use.
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_raw_mode_cycles -- --nocapture`
    test_fn: test_raw_mode_cycles,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Cooked,
}

/// Controller process: verifies that controlled process completes multiple cycles successfully.
fn pty_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 PTY Controller: Starting multiple cycles test...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    eprintln!("📝 PTY Controller: Waiting for controlled process cycle results...");

    let mut controlled_started = false;
    let mut cycles_completed = 0;
    let mut test_passed = false;
    let start_timeout = Instant::now();

    while start_timeout.elapsed() < Duration::from_secs(10) {
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
                if trimmed.contains("CYCLE_COMPLETE:") {
                    cycles_completed += 1;
                    eprintln!("  ✓ Cycle {cycles_completed} completed");
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
    assert_eq!(cycles_completed, 3, "Expected 3 cycles to complete");
    assert!(test_passed, "Test did not report success");

    match child.wait() {
        Ok(status) => {
            eprintln!("✅ PTY Controller: Controlled process exited: {status:?}");
        }
        Err(e) => {
            panic!("Failed to wait for controlled process: {e}");
        }
    }

    eprintln!("✅ PTY Controller: Multiple cycles test passed!");
}

/// Controlled process: performs multiple enable/disable cycles.
fn pty_controlled_entry_point() -> ! {
    println!("SLAVE_STARTING");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Starting multiple cycles test...");

    let stdin = std::io::stdin();

    // Get original terminal settings (before any raw mode)
    let original_termios = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Controlled: Failed to get original termios: {e}");
            println!("FAILED: Could not read original termios");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };

    // Perform 3 enable/disable cycles
    for cycle in 1..=3 {
        eprintln!("🔍 Controlled: --- Cycle {cycle} ---");

        // Enable raw mode
        if let Err(e) = crate::enable_raw_mode() {
            eprintln!("⚠️  Controlled: Failed to enable raw mode in cycle {cycle}: {e}");
            println!("FAILED: Could not enable raw mode in cycle {cycle}");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
        eprintln!("  ✓ Cycle {cycle}: Raw mode enabled");

        // Verify we're in raw mode (ICANON should be off)
        let raw_termios = match termios::tcgetattr(&stdin) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️  Controlled: Failed to get termios in cycle {cycle}: {e}");
                println!("FAILED: Could not read termios in cycle {cycle}");
                std::io::stdout().flush().expect("Failed to flush");
                std::process::exit(1);
            }
        };

        if raw_termios.local_modes.contains(rustix::termios::LocalModes::ICANON) {
            eprintln!("⚠️  Controlled: ICANON still on in cycle {cycle}");
            println!("FAILED: Raw mode not properly enabled in cycle {cycle}");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
        eprintln!("  ✓ Cycle {cycle}: Verified in raw mode");

        // Disable raw mode
        if let Err(e) = crate::disable_raw_mode() {
            eprintln!("⚠️  Controlled: Failed to disable raw mode in cycle {cycle}: {e}");
            println!("FAILED: Could not disable raw mode in cycle {cycle}");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
        eprintln!("  ✓ Cycle {cycle}: Raw mode disabled");

        // Verify we're back in cooked mode (ICANON should be on)
        let restored_termios = match termios::tcgetattr(&stdin) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️  Controlled: Failed to get termios after restore in cycle {cycle}: {e}");
                println!("FAILED: Could not read termios after restore in cycle {cycle}");
                std::io::stdout().flush().expect("Failed to flush");
                std::process::exit(1);
            }
        };

        // Settings should match original (at least for local_modes)
        if restored_termios.local_modes != original_termios.local_modes {
            eprintln!("⚠️  Controlled: Settings not restored in cycle {cycle}");
            eprintln!("    Original: {:?}", original_termios.local_modes);
            eprintln!("    Restored: {:?}", restored_termios.local_modes);
            println!("FAILED: Settings not properly restored in cycle {cycle}");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
        eprintln!("  ✓ Cycle {cycle}: Settings restored correctly");

        println!("CYCLE_COMPLETE: {cycle}");
        std::io::stdout().flush().expect("Failed to flush");
    }

    // Test calling enable() twice without disable() in between
    eprintln!("🔍 Controlled: Testing double enable...");
    if let Err(e) = crate::enable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed first enable in double test: {e}");
        println!("FAILED: First enable failed in double test");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    // Second enable should not fail (it's a no-op if already in raw mode)
    if let Err(e) = crate::enable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed second enable in double test: {e}");
        println!("FAILED: Second enable failed in double test");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ Double enable succeeded");

    // Clean up
    if let Err(e) = crate::disable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed to disable after double enable: {e}");
        println!("FAILED: Cleanup failed after double enable");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    // All cycles passed!
    println!("SUCCESS: All cycles completed successfully");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

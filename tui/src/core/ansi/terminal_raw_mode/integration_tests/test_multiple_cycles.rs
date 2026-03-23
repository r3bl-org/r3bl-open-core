// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test 4: Multiple enable/disable cycles.
//!
//! Verifies that raw mode can be enabled and disabled multiple times without issues.
//! Tests edge cases like:
//! - Calling [`enable_raw_mode()`] when already enabled
//! - Calling [`disable_raw_mode()`] when already disabled
//! - Saved settings are preserved across cycles
//!
//! [`disable_raw_mode()`]: crate::disable_raw_mode
//! [`enable_raw_mode()`]: crate::enable_raw_mode

use crate::{PtyTestContext, PtyTestMode};
use rustix::termios;
use std::{io::{BufRead, Write},
          time::{Duration, Instant}};

generate_pty_test! {
    /// [`PTY`]-based integration test for multiple raw mode cycles.
    ///
    /// Verifies that enabling and disabling raw mode multiple times works correctly:
    /// 1. First [`enable`]/[`disable`] cycle.
    /// 2. Second [`enable`]/[`disable`] cycle.
    /// 3. Third [`enable`]/[`disable`] cycle.
    /// 4. Settings restore correctly after each cycle.
    /// 5. Multiple [`enable`] calls don't fail.
    ///
    /// This catches edge cases with the [`SAVED_TERMIOS`] static and ensures the
    /// implementation is robust for repeated use.
    ///
    /// Run with:
    /// ```bash
    /// cargo test -p r3bl_tui --lib test_raw_mode_cycles -- --nocapture
    /// ```
    ///
    /// [`disable`]: crate::disable_raw_mode
    /// [`enable`]: crate::enable_raw_mode
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`SAVED_TERMIOS`]: crate::terminal_raw_mode::raw_mode_unix::SAVED_TERMIOS
    test_fn: test_raw_mode_cycles,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Cooked,
}

/// Controller process: verifies that controlled process completes multiple cycles
/// successfully.
fn pty_controller_entry_point(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("🚀 PTY Controller: Starting multiple cycles test...");

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

    assert!(
        controlled_started,
        "Controlled process did not start properly"
    );
    assert_eq!(cycles_completed, 3, "Expected 3 cycles to complete");
    assert!(test_passed, "Test did not report success");

    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("✅ PTY Controller: Multiple cycles test passed!");
}

/// Controlled process: performs multiple enable/disable cycles.
#[allow(clippy::too_many_lines)]
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

        if raw_termios
            .local_modes
            .contains(rustix::termios::LocalModes::ICANON)
        {
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
                eprintln!(
                    "⚠️  Controlled: Failed to get termios after restore in cycle {cycle}: {e}"
                );
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

    // Second enable should not fail. SAVED_TERMIOS already holds the cooked
    // settings from the first enable, so the is_none() guard skips the save.
    // Raw mode is re-applied (harmless), and the saved cooked state is preserved.
    if let Err(e) = crate::enable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed second enable in double test: {e}");
        println!("FAILED: Second enable failed in double test");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ Double enable succeeded");

    // Disable should restore the cooked settings saved by the first enable.
    if let Err(e) = crate::disable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed to disable after double enable: {e}");
        println!("FAILED: Cleanup failed after double enable");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    // Verify cooked mode was properly restored after double-enable.
    let after_double = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Controlled: Failed to get termios after double enable: {e}");
            println!("FAILED: Could not read termios after double enable cleanup");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };
    if after_double.local_modes != original_termios.local_modes {
        eprintln!("⚠️  Controlled: Settings not restored after double enable");
        eprintln!("    Original: {:?}", original_termios.local_modes);
        eprintln!("    Restored: {:?}", after_double.local_modes);
        println!("FAILED: Settings not properly restored after double enable");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ Settings restored correctly after double enable");

    // Test calling disable() when already disabled (should be a safe no-op).
    eprintln!("🔍 Controlled: Testing double disable...");
    if let Err(e) = crate::disable_raw_mode() {
        eprintln!("⚠️  Controlled: Failed second disable in double test: {e}");
        println!("FAILED: Second disable failed in double test");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }

    // Terminal settings should be unchanged (still cooked).
    let after_double_disable = match termios::tcgetattr(&stdin) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Controlled: Failed to get termios after double disable: {e}");
            println!("FAILED: Could not read termios after double disable");
            std::io::stdout().flush().expect("Failed to flush");
            std::process::exit(1);
        }
    };
    if after_double_disable.local_modes != original_termios.local_modes {
        eprintln!("⚠️  Controlled: Settings changed after double disable");
        eprintln!("    Original: {:?}", original_termios.local_modes);
        eprintln!("    After:    {:?}", after_double_disable.local_modes);
        println!("FAILED: Double disable changed terminal settings");
        std::io::stdout().flush().expect("Failed to flush");
        std::process::exit(1);
    }
    eprintln!("  ✓ Double disable was a safe no-op");

    // All tests passed!
    println!("SUCCESS: All cycles completed successfully");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("🔍 Controlled: Completed, exiting");
    std::process::exit(0);
}

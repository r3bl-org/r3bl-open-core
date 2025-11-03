// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Manual validation test for `/dev/tty` fallback when stdin is redirected.
//!
//! This test verifies that raw mode works correctly when stdin is not a tty,
//! which happens in real-world scenarios like piped input.

use std::io::Write;

/// Manual test for `/dev/tty` fallback with redirected stdin.
///
/// This test verifies the enhancement we made to match crossterm's behavior:
/// when stdin is not a tty (e.g., piped input), the implementation falls back
/// to opening `/dev/tty` for terminal control.
///
/// Run this test in a real terminal using:
/// ```bash
/// # Test with piped input (stdin is NOT a tty, but /dev/tty works)
/// echo "test" | cargo test --package r3bl_tui --lib test_dev_tty_fallback_manual -- --ignored --nocapture
/// ```
///
/// # Why This Must Be Run Manually
///
/// Test harnesses spawn processes without controlling terminals, so `/dev/tty`
/// doesn't exist in automated tests. However, when you run this test from a
/// shell with redirected stdin, the shell provides a controlling terminal that
/// makes `/dev/tty` available.
///
/// # Expected Behavior
///
/// When stdin is redirected:
/// 1. `stdin.isatty()` returns `false`
/// 2. `enable_raw_mode()` falls back to opening `/dev/tty`
/// 3. Raw mode is successfully enabled via `/dev/tty`
/// 4. Terminal settings can be verified through `/dev/tty`
///
/// # Real-World Use Case
///
/// This validates scenarios like:
/// ```bash
/// echo "data" | my_tui_app
/// cat config.txt | my_tui_app
/// ```
///
/// Where the app needs terminal control even though stdin is redirected.
#[test]
#[ignore = "Manual test: echo 'test' | cargo test test_dev_tty_fallback_manual -- --ignored --nocapture"]
fn test_dev_tty_fallback_manual() {
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║   /dev/tty Fallback Validation Test                   ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();

    // Check if stdin is a tty
    let stdin = std::io::stdin();
    let stdin_is_tty = rustix::termios::isatty(&stdin);

    println!("📋 Test Environment:");
    println!("   stdin.isatty() = {}", stdin_is_tty);

    if stdin_is_tty {
        println!();
        println!("⚠️  WARNING: stdin IS a tty");
        println!(
            "   This test should be run with redirected stdin to verify the fallback."
        );
        println!();
        println!("   Run it like this:");
        println!(
            "   echo 'test' | cargo test test_dev_tty_fallback_manual -- --ignored --nocapture"
        );
        println!();
        panic!("Test requires redirected stdin to verify /dev/tty fallback");
    }

    println!("   ✓ Confirmed: stdin is NOT a tty (as expected for this test)");
    println!();

    // Try to enable raw mode - should succeed via /dev/tty fallback
    println!("🔧 Testing raw mode with /dev/tty fallback...");
    match crate::enable_raw_mode() {
        Ok(()) => {
            println!("   ✓ Raw mode enabled successfully!");
            println!();
        }
        Err(e) => {
            println!("   ✗ Failed to enable raw mode: {e:?}");
            println!();

            // Check if /dev/tty exists
            match std::fs::File::options().read(true).open("/dev/tty") {
                Ok(_) => {
                    println!("   /dev/tty exists but raw mode failed");
                    panic!("Raw mode should work when /dev/tty is available: {e:?}");
                }
                Err(e) => {
                    println!("   /dev/tty is not available: {e}");
                    println!(
                        "   This is expected in environments without a controlling terminal (CI, etc.)"
                    );
                    println!();
                    println!("⏭️  Skipping test - no controlling terminal");
                    return;
                }
            }
        }
    }

    // Verify /dev/tty is accessible and in raw mode
    println!("🔍 Verifying /dev/tty terminal settings...");
    match std::fs::File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(tty) => {
            match rustix::termios::tcgetattr(&tty) {
                Ok(termios) => {
                    println!("   ✓ Successfully read /dev/tty termios");

                    // Verify key raw mode flags
                    use rustix::termios::LocalModes;

                    if termios.local_modes.contains(LocalModes::ICANON) {
                        println!("   ✗ ICANON is ON (should be OFF in raw mode)");
                        panic!("Raw mode not properly enabled - ICANON still set");
                    }
                    println!("   ✓ ICANON is OFF (raw mode active)");

                    if termios.local_modes.contains(LocalModes::ECHO) {
                        println!("   ✗ ECHO is ON (should be OFF in raw mode)");
                        panic!("Raw mode not properly enabled - ECHO still set");
                    }
                    println!("   ✓ ECHO is OFF (raw mode active)");

                    if termios.local_modes.contains(LocalModes::ISIG) {
                        println!("   ✗ ISIG is ON (should be OFF in raw mode)");
                        panic!("Raw mode not properly enabled - ISIG still set");
                    }
                    println!("   ✓ ISIG is OFF (raw mode active)");
                }
                Err(e) => {
                    println!("   ✗ Failed to read /dev/tty termios: {e}");
                    panic!("Could not verify /dev/tty settings: {e}");
                }
            }
        }
        Err(e) => {
            println!("   ✗ Failed to open /dev/tty: {e}");
            panic!("Could not open /dev/tty for verification: {e}");
        }
    }

    println!();

    // Disable raw mode
    println!("🧹 Cleaning up...");
    match crate::disable_raw_mode() {
        Ok(()) => {
            println!("   ✓ Raw mode disabled successfully");
        }
        Err(e) => {
            println!("   ✗ Failed to disable raw mode: {e:?}");
            panic!("Failed to disable raw mode: {e:?}");
        }
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║   ✅ /dev/tty Fallback Test PASSED                     ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();
    println!("Summary:");
    println!("  • stdin was NOT a tty (redirected)");
    println!("  • Raw mode successfully enabled via /dev/tty fallback");
    println!("  • Terminal settings verified (ICANON, ECHO, ISIG all OFF)");
    println!("  • Raw mode successfully disabled");
    println!();

    std::io::stdout().flush().expect("Failed to flush");
}

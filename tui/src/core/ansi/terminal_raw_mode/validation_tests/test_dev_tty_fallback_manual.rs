// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Manual validation test for `/dev/tty` fallback when stdin is redirected.
//!
//! This test verifies that raw mode works correctly when stdin is not a tty,
//! which happens in real-world scenarios like piped input.

use crate::{GLYPH_CONTROLLED, GLYPH_CONTROLLER_CLEANUP, GLYPH_FAILURE, GLYPH_SKIPPING,
            GLYPH_SUCCESS, GLYPH_WARNING};
use rustix::termios::LocalModes;
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
/// echo "test" | cargo test --package r3bl_tui test_dev_tty_fallback_manual -- --ignored --nocapture
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
#[allow(clippy::too_many_lines)]
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
    println!("   stdin.isatty() = {stdin_is_tty}");

    if stdin_is_tty {
        println!();
        println!("{GLYPH_WARNING}  WARNING: stdin IS a tty");
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

    println!(
        "   {GLYPH_SUCCESS} Confirmed: stdin is NOT a tty (as expected for this test)"
    );
    println!();

    // Try to enable raw mode - should succeed via /dev/tty fallback
    println!("🔧 Testing raw mode with /dev/tty fallback...");
    match crate::enable_raw_mode() {
        Ok(()) => {
            println!("   {GLYPH_SUCCESS} Raw mode enabled successfully!");
            println!();
        }
        Err(e) => {
            println!("   {GLYPH_FAILURE} Failed to enable raw mode: {e:?}");
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
                    println!("{GLYPH_SKIPPING} Skipping test - no controlling terminal");
                    return;
                }
            }
        }
    }

    // Verify /dev/tty is accessible and in raw mode
    println!("{GLYPH_CONTROLLED} Verifying /dev/tty terminal settings...");
    match std::fs::File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(tty) => {
            match rustix::termios::tcgetattr(&tty) {
                Ok(termios) => {
                    println!("   {GLYPH_SUCCESS} Successfully read /dev/tty termios");

                    // Verify key raw mode flags
                    if termios.local_modes.contains(LocalModes::ICANON) {
                        println!(
                            "   {GLYPH_FAILURE} ICANON is ON (should be OFF in raw mode)"
                        );
                        panic!("Raw mode not properly enabled - ICANON still set");
                    }
                    println!("   {GLYPH_SUCCESS} ICANON is OFF (raw mode active)");

                    if termios.local_modes.contains(LocalModes::ECHO) {
                        println!(
                            "   {GLYPH_FAILURE} ECHO is ON (should be OFF in raw mode)"
                        );
                        panic!("Raw mode not properly enabled - ECHO still set");
                    }
                    println!("   {GLYPH_SUCCESS} ECHO is OFF (raw mode active)");

                    if termios.local_modes.contains(LocalModes::ISIG) {
                        println!(
                            "   {GLYPH_FAILURE} ISIG is ON (should be OFF in raw mode)"
                        );
                        panic!("Raw mode not properly enabled - ISIG still set");
                    }
                    println!("   {GLYPH_SUCCESS} ISIG is OFF (raw mode active)");
                }
                Err(e) => {
                    println!("   {GLYPH_FAILURE} Failed to read /dev/tty termios: {e}");
                    panic!("Could not verify /dev/tty settings: {e}");
                }
            }
        }
        Err(e) => {
            println!("   {GLYPH_FAILURE} Failed to open /dev/tty: {e}");
            panic!("Could not open /dev/tty for verification: {e}");
        }
    }

    println!();

    // Disable raw mode
    println!("{GLYPH_CONTROLLER_CLEANUP} Cleaning up...");
    match crate::disable_raw_mode() {
        Ok(()) => {
            println!("   {GLYPH_SUCCESS} Raw mode disabled successfully");
        }
        Err(e) => {
            println!("   {GLYPH_FAILURE} Failed to disable raw mode: {e:?}");
            panic!("Failed to disable raw mode: {e:?}");
        }
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║   {GLYPH_SUCCESS} /dev/tty Fallback Test PASSED                     ║");
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

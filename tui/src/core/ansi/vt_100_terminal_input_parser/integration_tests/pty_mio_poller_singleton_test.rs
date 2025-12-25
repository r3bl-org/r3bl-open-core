// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for [`DirectToAnsiInputDevice`] singleton semantics.
//!
//! Tests that only one [`DirectToAnsiInputDevice`] can exist at a time, and that
//! calling [`new()`] twice panics with a helpful message.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_mio_poller_singleton -- --nocapture`
//!
//! Tests that:
//! 1. First [`new()`] succeeds
//! 2. Second [`new()`] panics with message guiding to use [`subscribe()`]
//! 3. After dropping the first device, [`new()`] succeeds again
//!
//! ## Test Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ Controlled Process (in PTY)                                                 │
//! │                                                                             │
//! │  1. Create first DirectToAnsiInputDevice                                    │
//! │     Assert: succeeds                                                        │
//! │  2. Try to create second device                                             │
//! │     Assert: panics with expected message                                    │
//! │  3. Drop first device                                                       │
//! │  4. Create new device                                                       │
//! │     Assert: succeeds (gate was cleared on drop)                             │
//! │                                                                             │
//! │  If all assertions pass → TEST_PASSED                                       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! [`new()`]: crate::direct_to_ansi::DirectToAnsiInputDevice::new
//! [`subscribe()`]: crate::direct_to_ansi::DirectToAnsiInputDevice::subscribe

use crate::{ControlledChild, PtyPair, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::io::{BufRead, BufReader, Write};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "SINGLETON_TEST_READY";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "SINGLETON_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_singleton,
    controller: singleton_controller_entry_point,
    controlled: singleton_controlled_entry_point
}

/// Helper to wait for a specific signal from controlled.
fn wait_for_signal(buf_reader: &mut BufReader<impl std::io::Read>, signal: &str) {
    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF before receiving {signal}"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  <- Controlled: {trimmed}");
                if trimmed.contains(signal) {
                    return;
                }
            }
            Err(e) => panic!("Read error waiting for {signal}: {e}"),
        }
    }
}

/// Controller process: waits for controlled to complete successfully.
fn singleton_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("Singleton Controller: Starting...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    // Wait for controlled to be ready.
    eprintln!("Singleton Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, CONTROLLED_READY);
    eprintln!("  Controlled is ready");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  Test passed signal received");

    // Clean up.
    match child.wait() {
        Ok(status) => eprintln!("Singleton Controller: Controlled exited: {status:?}"),
        Err(e) => panic!("Failed to wait for controlled: {e}"),
    }

    eprintln!("Singleton Controller: Test passed!");
}

/// Controlled process: tests singleton device semantics.
fn singleton_controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // Enable raw mode for proper input handling.
    eprintln!("Singleton Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {e}");
    }

    eprintln!("Singleton Controlled: Starting singleton test...");

    // Step 1: First device creation should succeed.
    eprintln!("Step 1: Creating first device...");
    let device1 = DirectToAnsiInputDevice::new();
    eprintln!("  First device created successfully");

    // Step 2: Second device creation should panic.
    eprintln!("Step 2: Attempting to create second device (should panic)...");
    let result = std::panic::catch_unwind(|| {
        let _device2 = DirectToAnsiInputDevice::new();
    });

    assert!(
        result.is_err(),
        "Expected new() to panic when device already exists"
    );

    // Verify panic message mentions subscribe().
    if let Err(panic_value) = result {
        let panic_msg = if let Some(s) = panic_value.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_value.downcast_ref::<String>() {
            s.clone()
        } else {
            String::new()
        };

        assert!(
            panic_msg.contains("subscribe()"),
            "Panic message should mention subscribe(), got: {panic_msg}"
        );
        eprintln!("  Second device correctly panicked with message: {panic_msg}");
    }

    // Step 3: Drop first device.
    eprintln!("Step 3: Dropping first device...");
    drop(device1);
    eprintln!("  First device dropped");

    // Step 4: New device creation should succeed after drop.
    eprintln!("Step 4: Creating new device after drop...");
    let _device3 = DirectToAnsiInputDevice::new();
    eprintln!("  New device created successfully after drop");

    // All assertions passed!
    eprintln!("All singleton test assertions passed!");
    println!("{TEST_PASSED}");
    std::io::stdout().flush().expect("Failed to flush");

    // Disable raw mode.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("Failed to disable raw mode: {e}");
    }

    eprintln!("Singleton Controlled: Exiting");
    std::process::exit(0);
}

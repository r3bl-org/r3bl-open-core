// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for [`DirectToAnsiInputDevice`] singleton semantics.
//!
//! Tests that only one [`DirectToAnsiInputDevice`] can exist at a time, and that calling
//! [`new()`] twice panics with a helpful message.
//!
//! Tests that:
//! 1. First [`new()`] succeeds
//! 2. Second [`new()`] panics with message guiding to use [`try_subscribe()`]
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
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_mio_poller_singleton -- --nocapture
//! ```
//!
//! [`new()`]: crate::direct_to_ansi::DirectToAnsiInputDevice::new
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`try_subscribe()`]: crate::direct_to_ansi::DirectToAnsiInputDevice::try_subscribe

use crate::{CaughtPanicResult, PtyTestContext, PtyTestMode, extract_panic_message,
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice};
use std::io::Write;

/// Ready signal sent by controlled process after initialization.
const SINGLETON_READY: &str = "SINGLETON_TEST_READY";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "SINGLETON_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_singleton,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}
/// Controller process: sends input bytes and verifies controlled completes successfully.
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        writer,
    } = context;

    eprintln!("PTY Controller: Starting...");

    // Wait for controlled to be ready.
    eprintln!("PTY Controller: Waiting for controlled to start...");
    child
        .wait_for_ready(&mut buf_reader, SINGLETON_READY)
        .expect("Failed to wait for SINGLETON_READY");
    eprintln!("  Controlled is ready");

    // Wait for test to pass.
    child
        .wait_for_ready(&mut buf_reader, TEST_PASSED)
        .expect("Failed to wait for TEST_PASSED");
    eprintln!("  Test passed signal received");

    // Clean up.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("PTY Controller: Test passed!");
}

/// Controlled process: tests singleton device semantics. The harness performs
/// [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{SINGLETON_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    eprintln!("Singleton Controlled: Starting singleton test...");

    // Step 1: First device creation should succeed.
    eprintln!("Step 1: Creating first device...");
    let device1 = DirectToAnsiInputDevice::new()
        .expect("Failed to initialize DirectToAnsiInputDevice");
    eprintln!("  First device created successfully");

    // Step 2: Second device creation should panic.
    eprintln!("Step 2: Attempting to create second device (should panic)...");
    let result: CaughtPanicResult = std::panic::catch_unwind(|| {
        let _device2 = DirectToAnsiInputDevice::new();
    });

    assert!(
        result.is_err(),
        "Expected new() to panic when device already exists"
    );

    // Verify panic message mentions subscribe().
    let panic_msg = extract_panic_message(result);

    assert_eq!(
        panic_msg,
        "DirectToAnsiInputDevice::new() called while another device exists. \
         Use device.try_subscribe() (fallible) to get a guard, then guard.try_subscribe() \
         for additional receivers."
    );
    eprintln!("  Second device correctly panicked with message: {panic_msg}");

    // Step 3: Drop first device.
    eprintln!("Step 3: Dropping first device...");
    drop(device1);
    eprintln!("  First device dropped");

    // Step 4: New device creation should succeed after drop.
    eprintln!("Step 4: Creating new device after drop...");
    let _device3 = DirectToAnsiInputDevice::new()
        .expect("Failed to initialize DirectToAnsiInputDevice");
    eprintln!("  New device created successfully after drop");

    // All assertions passed!
    eprintln!("All singleton test assertions passed!");
    println!("{TEST_PASSED}");
    std::io::stdout().flush().expect("Failed to flush");
}

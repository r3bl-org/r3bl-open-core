// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for thread reuse (race condition).
//!
//! Tests that when a new subscriber appears **before** the thread checks
//! `receiver_count`, the thread correctly continues running (not relaunched). This
//! validates the documented race condition is semantically correct.
//! See [`InputDeviceResourceHandle`] for the race condition documentation.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse --
//! --nocapture`
//!
//! Tests that:
//! 1. Thread spawns on first subscribe (`thread_alive = true`)
//! 2. Device A drops → waker fires
//! 3. Device B subscribes **immediately** (before thread checks `receiver_count`)
//! 4. Thread continues running (same thread, not relaunched)
//!
//! This validates the race is **semantically correct**: thread stays alive
//! because there IS a receiver when it checks.
//!
//! ## Test Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ Controlled Process (in PTY)                          Thread #1 (mio_poller) │
//! │                                                                             │
//! │  1. Create DirectToAnsiInputDevice A                   ┌───────────────┐    │
//! │     Assert: thread_alive = true, receiver_count = 1    │ poll() blocks │    │
//! │     Capture generation_before                          └─────────┬─────┘    │
//! │                                                                  │          │
//! │  2. Read input from device A (proves thread works)               │          │
//! │  3. Drop device A ────────────────────────────────▶ waker fires! │          │
//! │     │ IMMEDIATELY (no sleep!)                                    │          │
//! │     ▼                                                  ┌─────────▼─────┐    │
//! │  4. Create DirectToAnsiInputDevice B                   │ wakes up,     │    │
//! │     Assert: thread_alive = true, receiver_count = 1    │ checks count  │    │
//! │     │                                                  │ count > 0 ✓   │    │
//! │     ▼                                                  │ continues!    │    │
//! │  5. Read input from device B                           └───────────────┘    │
//! │     Assert: generation_before == generation_after (SAME thread!)            │
//! │                                                                             │
//! │  If generation unchanged → TEST_PASSED (thread reused, not relaunched)      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key difference from the lifecycle test: we create device B **immediately**
//! after dropping device A, racing the thread's `receiver_count` check.
//!
//! [`InputDeviceResourceHandle`]: crate::direct_to_ansi::input::InputDeviceResourceHandle

use crate::{ControlledChild, PtyPair,
            direct_to_ansi::{DirectToAnsiInputDevice,
                             input::{global_input_resource::{guarded_ops, ThreadLiveness}}},
            generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "REUSE_TEST_READY";

/// Signal sent when device A is created.
const DEVICE_A_CREATED: &str = "REUSE_DEVICE_A_CREATED";

/// Signal sent when device B is created immediately after A dropped.
const DEVICE_B_CREATED: &str = "REUSE_DEVICE_B_CREATED";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "REUSE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_thread_reuse,
    controller: controller_entry_point,
    controlled: controlled_entry_point
}

/// Helper to wait for a specific signal from controlled.
fn wait_for_signal(buf_reader: &mut BufReader<impl std::io::Read>, signal: &str) {
    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) => panic!("EOF before receiving {signal}"),
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled: {trimmed}");
                if trimmed.contains(signal) {
                    return;
                }
            }
            Err(e) => panic!("Read error waiting for {signal}: {e}"),
        }
    }
}

/// Controller process: sends input bytes and verifies controlled completes successfully.
fn controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 Reuse Controller: Starting...");

    let mut writer = pty_pair
        .controller()
        .take_writer()
        .expect("Failed to get writer");
    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    // Wait for controlled to be ready.
    eprintln!("📝 Reuse Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, CONTROLLED_READY);
    eprintln!("  ✓ Controlled is ready");

    // Wait for device A, send input.
    wait_for_signal(&mut buf_reader, DEVICE_A_CREATED);
    eprintln!("📝 Reuse Controller: Sending input for device A...");
    writer.write_all(b"a").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for device B (created immediately after A dropped), send input.
    wait_for_signal(&mut buf_reader, DEVICE_B_CREATED);
    eprintln!("📝 Reuse Controller: Sending input for device B...");
    writer.write_all(b"b").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  ✓ Test passed signal received");

    // Clean up.
    drop(writer);
    match child.wait() {
        Ok(status) => eprintln!("✅ Reuse Controller: Controlled exited: {status:?}"),
        Err(e) => panic!("Failed to wait for controlled: {e}"),
    }

    eprintln!("✅ Reuse Controller: Test passed!");
}

/// Controlled process: tests thread reuse with immediate subscription.
fn controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // Enable raw mode for proper input handling.
    eprintln!("🔍 Reuse Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("⚠️  Failed to enable raw mode: {e}");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 Reuse Controlled: Starting thread reuse test...");

        // Step 1: Create device A - this spawns the thread.
        eprintln!("📍 Step 1: Creating device A...");
        let mut device_a = DirectToAnsiInputDevice::new();

        println!("{DEVICE_A_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device A to trigger subscription.
        let event_a = tokio::time::timeout(Duration::from_secs(5), device_a.try_read_event())
            .await
            .expect("Timeout reading from device A");
        eprintln!("  ✓ Device A received event: {event_a:?}");

        // Verify thread is alive and capture generation for later comparison.
        assert_eq!(
            guarded_ops::is_thread_running(),
            ThreadLiveness::Running,
            "Expected thread_alive = Alive after device A created"
        );
        let initial_receiver_count = guarded_ops::get_receiver_count();
        assert_eq!(initial_receiver_count, 1, "Expected receiver_count = 1");
        let generation_before = guarded_ops::get_thread_generation();
        eprintln!("  ✓ Thread alive, receiver_count = 1, generation = {generation_before}");

        // Step 2: Drop device A and IMMEDIATELY create device B.
        // This tests the race condition where a new subscriber appears before
        // the thread can check receiver_count.
        eprintln!("📍 Step 2: Dropping device A and immediately creating device B...");

        // Drop device A.
        drop(device_a);
        eprintln!("  ✓ Device A dropped (waker should have fired)");

        // Immediately create device B (no sleep!).
        let mut device_b = DirectToAnsiInputDevice::new();
        eprintln!("  ✓ Device B created immediately");

        println!("{DEVICE_B_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device B to trigger subscription.
        let event_b = tokio::time::timeout(Duration::from_secs(5), device_b.try_read_event())
            .await
            .expect("Timeout reading from device B");
        eprintln!("  ✓ Device B received event: {event_b:?}");

        // Step 3: Verify thread is still alive AND same generation (reused, not relaunched).
        assert_eq!(
            guarded_ops::is_thread_running(),
            ThreadLiveness::Running,
            "Expected thread_alive = Alive (thread should continue serving device B)"
        );
        assert_eq!(
            guarded_ops::get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device B subscribed"
        );
        let generation_after = guarded_ops::get_thread_generation();
        assert_eq!(
            generation_before, generation_after,
            "Expected same thread generation (reuse, not relaunch). \
             Before: {generation_before}, After: {generation_after}"
        );
        eprintln!(
            "  ✓ Thread still alive, receiver_count = 1, generation = {generation_after} (same thread reused!)"
        );

        // All assertions passed!
        eprintln!("🎉 Thread reuse test passed! Race condition handled correctly.");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Clean up.
        drop(device_b);
    });

    // Disable raw mode.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("⚠️  Failed to disable raw mode: {e}");
    }

    eprintln!("🔍 Reuse Controlled: Exiting");
    std::process::exit(0);
}

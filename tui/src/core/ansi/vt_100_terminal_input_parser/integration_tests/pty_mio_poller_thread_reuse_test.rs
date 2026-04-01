// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for thread reuse (fast path).
//!
//! Tests that the mio-poller thread is **reused** (not relaunched) when a new subscriber
//! appears while the thread is still alive. This validates the fast-path behavior
//! documented in [`RRT::subscribe()`].
//!
//! **Companion test**: [`pty_mio_poller_thread_lifecycle_test`] validates the opposite
//! scenario -- thread exit and relaunch (slow path).
//!
//! Tests that:
//! 1. Thread spawns on first subscribe (`thread_alive = true`)
//! 2. A temporary subscriber keeps `receiver_count > 0` across the device transition
//! 3. Device B subscribes and receives events from the **same** thread
//! 4. Thread continues running (same generation, not relaunched)
//!
//! ## Strategy: Overlapping Subscriptions
//!
//! The original test tried to race `drop(device_a)` against
//! `DirectToAnsiInputDevice::new()` for device B. Under CPU load, the mio-poller thread
//! could wake and see `receiver_count() == 0` before the subscribe completed, causing it
//! to exit (flaky failure).
//!
//! The fix uses [`SINGLETON.subscribe_to_existing()`] to create a temporary subscriber
//! that persists until device B is fully ready, ensuring the thread stays alive.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse -- --nocapture
//! ```
//!
//! [`pty_mio_poller_thread_lifecycle_test`]: super::pty_mio_poller_thread_lifecycle_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RRT::subscribe()`]: crate::core::resilient_reactor_thread::RRT::subscribe
//! [`SINGLETON.subscribe_to_existing()`]:
//!     crate::direct_to_ansi::input::global_input_resource::SINGLETON

use crate::{PtyTestContext, PtyTestMode,
            direct_to_ansi::{DirectToAnsiInputDevice, input::global_input_resource},
            generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const REUSE_READY: &str = "REUSE_TEST_READY";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "REUSE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_thread_reuse,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
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
fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("Reuse Controller: Starting...");

    // Wait for controlled to be ready.
    eprintln!("📝 Reuse Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, REUSE_READY);
    eprintln!("  ✓ Controlled is ready");

    // Send multiple inputs to be read by different device instances.
    eprintln!("📝 Reuse Controller: Sending input for device A...");
    writer.write_all(b"a").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    std::thread::sleep(Duration::from_millis(50));

    eprintln!("📝 Reuse Controller: Sending input for device B...");
    writer.write_all(b"b").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  ✓ Test passed signal received");

    // Clean up.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("✅ Reuse Controller: Test passed!");
}

/// Controlled process: tests thread reuse with overlapping subscriptions. The harness
/// performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{REUSE_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 Reuse Controlled: Starting reuse test...");

        // Step 1: Create device A.
        eprintln!("📍 Step 1: Creating device A...");
        let mut device_a = DirectToAnsiInputDevice::new();

        // Read one event from device A.
        let event_a = tokio::time::timeout(Duration::from_secs(5), device_a.next())
            .await
            .expect("Timeout reading from device A");
        eprintln!("  ✓ Device A received event: {event_a:?}");

        // Capture generation of the running thread.
        let generation_a = global_input_resource::SINGLETON.get_thread_generation();
        eprintln!("  ✓ Thread generation A: {generation_a}");

        // Step 2: Create overlapping subscription.
        // This keeps the thread alive even when device_a is dropped.
        eprintln!("📍 Step 2: Creating overlapping subscription...");
        let temp_subscriber = global_input_resource::SINGLETON.subscribe_to_existing();
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            2,
            "Expected 2 receivers (device_a + temp)"
        );

        // Step 3: Drop device A and create device B.
        eprintln!("📍 Step 3: Dropping device A, creating device B...");
        drop(device_a);
        let mut device_b = DirectToAnsiInputDevice::new();

        // Read one event from device B.
        let event_b = tokio::time::timeout(Duration::from_secs(5), device_b.next())
            .await
            .expect("Timeout reading from device B");
        eprintln!("  ✓ Device B received event: {event_b:?}");

        // Step 4: Verify thread generation is UNCHANGED (reuse).
        let generation_b = global_input_resource::SINGLETON.get_thread_generation();
        assert_eq!(
            generation_a, generation_b,
            "Thread was relaunched! Gen A: {generation_a}, Gen B: {generation_b}"
        );
        eprintln!("  ✓ Thread generation B: {generation_b} (REUSED!)");

        // All assertions passed!
        eprintln!("🎉 All reuse assertions passed!");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Clean up.
        drop(device_b);
        drop(temp_subscriber);
    });
}

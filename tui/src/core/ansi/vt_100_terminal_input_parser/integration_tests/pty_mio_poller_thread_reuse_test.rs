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
//! Run with:
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_reuse -- --nocapture
//! ```
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
//! that overlaps with device A, ensuring `receiver_count` never reaches 0:
//!
//! ```text
//! temp_guard = subscribe_to_existing()    count: 1 -> 2
//! drop(device_a)                          count: 2 -> 1  (never 0!)
//! device_b = new()                        count: 1 -> 2
//! drop(temp_guard)                        count: 2 -> 1
//! ```
//!
//! This is deterministic regardless of OS thread scheduling.
//!
//! ## Test Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ Controlled Process (in PTY)                          Thread #1 (mio_poller) │
//! │                                                                             │
//! │  1. Create DirectToAnsiInputDevice A                   ┌───────────┐        │
//! │     Assert: thread_alive = true, receiver_count = 1    │ poll()    │        │
//! │     Capture generation_before                          │ blocks    │        │
//! │                                                        └─────┬─────┘        │
//! │  2. Read input from device A (proves thread works)           │              │
//! │  3. Create temp_guard via subscribe_to_existing()            │              │
//! │     (receiver_count: 1 -> 2)                                 │              │
//! │  4. Drop device A                                            │              │
//! │     (receiver_count: 2 -> 1, waker fires)    waker fires! ───┘              │
//! │                                                  ┌──────────┬─────────┐     │
//! │  5. Create DirectToAnsiInputDevice B             │ wakes up,          │     │
//! │     (receiver_count: 1 -> 2)                     │ checks count       │     │
//! │  6. Drop temp_guard                              │ count > 0 (ok!)    │     │
//! │     (receiver_count: 2 -> 1)                     │ continues!         │     │
//! │                                                  └────────────────────┘     │
//! │  7. Read input from device B                                                │
//! │     Assert: generation_before == generation_after (SAME thread!)            │
//! │                                                                             │
//! │  If generation unchanged -> TEST_PASSED (thread reused, not relaunched)     │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! [`pty_mio_poller_thread_lifecycle_test`]:
//!     super::pty_mio_poller_thread_lifecycle_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RRT::subscribe()`]: crate::core::resilient_reactor_thread::RRT::subscribe
//! [`SINGLETON.subscribe_to_existing()`]:
//!     crate::direct_to_ansi::input::global_input_resource::SINGLETON

use crate::{PtyPair, PtyTestMode, SingleThreadSafeControlledChild,
            core::resilient_reactor_thread::LivenessState,
            direct_to_ansi::{DirectToAnsiInputDevice,
                             input::global_input_resource::SINGLETON}};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "REUSE_TEST_READY";

/// Signal sent when device A is created.
const DEVICE_A_CREATED: &str = "REUSE_DEVICE_A_CREATED";

/// Signal sent when device B is created after the overlap transition.
const DEVICE_B_CREATED: &str = "REUSE_DEVICE_B_CREATED";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "REUSE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_thread_reuse,
    controller: controller_entry_point,
    controlled: controlled_entry_point,
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
                eprintln!("  <- Controlled: {trimmed}");
                if trimmed.contains(signal) {
                    return;
                }
            }
            Err(e) => panic!("Read error waiting for {signal}: {e}"),
        }
    }
}

/// Controller process: sends input bytes and verifies controlled completes successfully.
fn controller_entry_point(pty_pair: PtyPair, child: SingleThreadSafeControlledChild) {
    eprintln!("Reuse Controller: Starting...");

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
    eprintln!("Reuse Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, CONTROLLED_READY);
    eprintln!("  Controlled is ready");

    // Wait for device A, send input.
    wait_for_signal(&mut buf_reader, DEVICE_A_CREATED);
    eprintln!("Reuse Controller: Sending input for device A...");
    writer.write_all(b"a").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for device B (created after overlap transition), send input.
    wait_for_signal(&mut buf_reader, DEVICE_B_CREATED);
    eprintln!("Reuse Controller: Sending input for device B...");
    writer.write_all(b"b").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  Test passed signal received");

    // Clean up.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("Reuse Controller: Test passed!");
}

/// Controlled process: tests thread reuse with overlapping subscriptions.
fn controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("Reuse Controlled: Starting thread reuse test...");

        // Step 1: Create device A - this spawns the thread.
        eprintln!("Step 1: Creating device A...");
        let mut device_a = DirectToAnsiInputDevice::new();

        println!("{DEVICE_A_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device A to prove the thread works.
        let event_a = tokio::time::timeout(Duration::from_secs(5), device_a.next())
            .await
            .expect("Timeout reading from device A");
        eprintln!("  Device A received event: {event_a:?}");

        // Verify thread is alive and capture generation for later comparison.
        assert_eq!(
            SINGLETON.is_thread_running(),
            LivenessState::Running,
            "Expected thread_alive = Alive after device A created"
        );
        let initial_receiver_count = SINGLETON.get_receiver_count();
        assert_eq!(initial_receiver_count, 1, "Expected receiver_count = 1");
        let generation_before = SINGLETON.get_thread_generation();
        eprintln!("  Thread alive, receiver_count = 1, generation = {generation_before}");

        // Step 2: Overlap transition from device A to device B.
        //
        // Create a temporary subscriber BEFORE dropping device A. This keeps
        // receiver_count > 0 across the transition, so the thread always takes
        // the fast path (reuse) regardless of OS thread scheduling.
        eprintln!("Step 2: Overlapping transition from device A to device B...");

        // Temporary subscriber keeps the thread alive during the transition.
        let temp_guard = SINGLETON.subscribe_to_existing(); // count: 1 -> 2
        eprintln!("  temp_guard created (receiver_count: 1 -> 2)");

        drop(device_a); // count: 2 -> 1 (never 0!)
        eprintln!("  Device A dropped (receiver_count: 2 -> 1)");

        let mut device_b = DirectToAnsiInputDevice::new(); // count: 1 -> 2
        eprintln!("  Device B created (receiver_count: 1 -> 2)");

        drop(temp_guard); // count: 2 -> 1
        eprintln!("  temp_guard dropped (receiver_count: 2 -> 1)");

        println!("{DEVICE_B_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device B to prove the thread serves it.
        let event_b = tokio::time::timeout(Duration::from_secs(5), device_b.next())
            .await
            .expect("Timeout reading from device B");
        eprintln!("  Device B received event: {event_b:?}");

        // Step 3: Verify thread is still alive AND same generation (reused, not
        // relaunched).
        assert_eq!(
            SINGLETON.is_thread_running(),
            LivenessState::Running,
            "Expected thread_alive = Alive (thread should continue serving device B)"
        );
        assert_eq!(
            SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device B subscribed"
        );
        let generation_after = SINGLETON.get_thread_generation();
        assert_eq!(
            generation_before, generation_after,
            "Expected same thread generation (reuse, not relaunch). \
             Before: {generation_before}, After: {generation_after}"
        );
        eprintln!(
            "  Thread still alive, receiver_count = 1, generation = {generation_after} (same thread reused!)"
        );

        // All assertions passed!
        eprintln!("Thread reuse test passed! Race condition handled correctly.");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Clean up.
        drop(device_b);
    });

    eprintln!("Reuse Controlled: Exiting");
    std::process::exit(0);
}

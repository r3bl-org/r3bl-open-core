// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for [`mio_poller`] thread lifecycle (slow path).
//!
//! Tests the complete thread spawn -> drop -> respawn cycle using observable state
//! functions. See [Device Lifecycle] for the lifecycle being tested.
//!
//! **Companion test**: [`pty_mio_poller_thread_reuse_test`] validates the opposite
//! scenario -- thread reuse via overlapping subscriptions (fast path).
//!
//! Tests that:
//! 1. Thread spawns on first subscribe (`thread_alive = true`, `receiver_count = 1`)
//! 2. Thread exits when receiver drops (`thread_alive = false`, `receiver_count = 0`)
//! 3. New thread spawns on next subscribe (proves `Drop` impl worked)
//!
//! ## Test Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ Controlled Process (in PTY)                                                 │
//! │                                                                             │
//! │  1. Assert: thread_alive = false, receiver_count = 0 (initial state)        │
//! │  2. Create DirectToAnsiInputDevice A                                        │
//! │     Assert: thread_alive = true, receiver_count = 1                         │
//! │  3. Read input from device A (proves thread #1 works)                       │
//! │  4. Drop device A                                                           │
//! │     Wait for thread to exit                                                 │
//! │     Assert: thread_alive = false, receiver_count = 0                        │
//! │  5. Create DirectToAnsiInputDevice B                                        │
//! │     Assert: thread_alive = true, receiver_count = 1 (NEW thread!)           │
//! │  6. Read input from device B (proves thread #2 works)                       │
//! │                                                                             │
//! │  If all assertions pass → TEST_PASSED                                       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_mio_poller_thread_lifecycle -- --nocapture
//! ```
//!
//! [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
//! [`pty_mio_poller_thread_reuse_test`]: super::pty_mio_poller_thread_reuse_test
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [Device Lifecycle]: crate::direct_to_ansi::DirectToAnsiInputDevice#device-lifecycle

use crate::{GLYPH_COMPLETION, GLYPH_CONTROLLED, GLYPH_CONTROLLER, GLYPH_STEP,
            GLYPH_SUCCESS, GLYPH_WAITING, PtyTestContext, PtyTestMode,
            core::resilient_reactor_thread::ThreadState,
            direct_to_ansi::{DirectToAnsiInputDevice, input::global_input_resource},
            generate_pty_test};
use std::{io::Write,
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const LIFECYCLE_READY: &str = "LIFECYCLE_TEST_READY";

/// Signal sent when device A is created and verified.
const DEVICE_A_CREATED: &str = "DEVICE_A_CREATED";

/// Signal sent when device A is dropped and thread exit verified.
const DEVICE_A_DROPPED: &str = "DEVICE_A_DROPPED";

/// Signal sent when device B is created (proves relaunch worked).
const DEVICE_B_CREATED: &str = "DEVICE_B_CREATED";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "LIFECYCLE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_thread_lifecycle,
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
        mut writer,
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting...");

    // Wait for controlled to be ready.
    eprintln!("{GLYPH_WAITING} PTY Controller: Waiting for controlled to start...");
    child
        .wait_for_ready(&mut buf_reader, LIFECYCLE_READY)
        .expect("Failed to wait for LIFECYCLE_READY");
    eprintln!("  {GLYPH_SUCCESS} Controlled is ready");

    // Wait for device A to be created, then send input.
    child
        .wait_for_ready(&mut buf_reader, DEVICE_A_CREATED)
        .expect("Failed to wait for DEVICE_A_CREATED");
    eprintln!("{GLYPH_WAITING} PTY Controller: Sending input for device A...");
    writer.write_all(b"a").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for device A to be dropped.
    child
        .wait_for_ready(&mut buf_reader, DEVICE_A_DROPPED)
        .expect("Failed to wait for DEVICE_A_DROPPED");
    eprintln!("  {GLYPH_SUCCESS} Device A dropped, thread should have exited");

    // Wait for device B to be created, then send input.
    child
        .wait_for_ready(&mut buf_reader, DEVICE_B_CREATED)
        .expect("Failed to wait for DEVICE_B_CREATED");
    eprintln!("{GLYPH_WAITING} PTY Controller: Sending input for device B...");
    writer.write_all(b"b").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    child
        .wait_for_ready(&mut buf_reader, TEST_PASSED)
        .expect("Failed to wait for TEST_PASSED");
    eprintln!("  {GLYPH_SUCCESS} Test passed signal received");

    // Clean up.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Test passed!");
}

/// Controlled process: tests thread lifecycle with assertions. The harness performs
/// [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{LIFECYCLE_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("{GLYPH_CONTROLLED} PTY Controlled: Starting lifecycle test...");
        let is_running = || {
            matches!(
                *global_input_resource::SINGLETON.shared_state.lock(),
                ThreadState::Running(_)
            )
        };
        let is_stopped = || {
            matches!(
                *global_input_resource::SINGLETON.shared_state.lock(),
                ThreadState::Stopped
            )
        };

        // Step 1: Verify initial state (no thread yet).
        eprintln!("{GLYPH_STEP} Step 1: Checking initial state...");
        assert!(is_stopped(), "Expected thread_alive = Dead initially");
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 initially"
        );
        eprintln!("  {GLYPH_SUCCESS} Initial state: thread_alive=false, receiver_count=0");

        // Step 2: Create device A - this spawns thread #1.
        eprintln!("{GLYPH_STEP} Step 2: Creating device A...");
        let mut device_a = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");

        // Signal that we're ready for input, then read.
        println!("{DEVICE_A_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device A.
        let event_a = tokio::time::timeout(Duration::from_secs(5), device_a.next())
            .await
            .expect("Timeout reading from device A");
        eprintln!("  {GLYPH_SUCCESS} Device A received event: {event_a:?}");

        // Verify thread is alive and capture generation.
        assert!(is_running(), "Expected thread_alive = Alive after device A created");
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device A subscribed"
        );
        let generation_a = global_input_resource::SINGLETON.get_thread_generation();
        eprintln!("  {GLYPH_SUCCESS} After device A: thread_alive=true, receiver_count=1, generation={generation_a}");

        // Step 3: Drop device A - this should cause thread #1 to exit.
        eprintln!("{GLYPH_STEP} Step 3: Dropping device A...");
        drop(device_a);

        // Give thread time to detect no receivers and exit.
        // With mio::Waker, thread should exit nearly instantaneously.
        eprintln!("  ⏳ Waiting for thread to exit...");
        let mut thread_exited = false;
        for i in 0..100 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            if is_stopped() {
                eprintln!("  {} Thread exited after {}ms", GLYPH_SUCCESS, i + 1);
                thread_exited = true;
                break;
            }
        }

        assert!(thread_exited, "Thread did not exit within 100ms");
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 after device A dropped"
        );
        eprintln!("  {GLYPH_SUCCESS} After device A dropped: thread_alive=false, receiver_count=0");

        println!("{DEVICE_A_DROPPED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Step 4: Create device B - this should spawn thread #2.
        eprintln!("{GLYPH_STEP} Step 4: Creating device B (should spawn new thread)...");
        let mut device_b = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");

        println!("{DEVICE_B_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device B.
        let event_b = tokio::time::timeout(Duration::from_secs(5), device_b.next())
            .await
            .expect("Timeout reading from device B");
        eprintln!("  {GLYPH_SUCCESS} Device B received event: {event_b:?}");

        // Verify NEW thread is alive with a NEW generation.
        assert!(
            is_running(),
            "Expected thread_alive = Alive after device B created (new thread)"
        );
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device B subscribed"
        );
        let generation_b = global_input_resource::SINGLETON.get_thread_generation();
        assert!(
            generation_b > generation_a,
            "Expected new generation (relaunch). Before: {generation_a}, After: {generation_b}"
        );
        eprintln!(
            "  {GLYPH_SUCCESS} After device B: thread_alive=true, receiver_count=1, generation={generation_b} (NEW thread!)"
        );

        // All assertions passed!
        eprintln!("{GLYPH_COMPLETION} All lifecycle assertions passed!");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Clean up.
        drop(device_b);
    });
}

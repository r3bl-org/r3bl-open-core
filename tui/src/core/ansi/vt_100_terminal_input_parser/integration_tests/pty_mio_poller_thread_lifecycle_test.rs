// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for [`mio_poller`] thread lifecycle.
//!
//! Tests the complete thread spawn → drop → respawn cycle using observable state
//! functions. See [Device Lifecycle] for the lifecycle being tested.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_mio_poller_thread_lifecycle --
//! --nocapture`
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
//! [Device Lifecycle]: crate::direct_to_ansi::DirectToAnsiInputDevice#device-lifecycle
//! [`mio_poller`]: crate::direct_to_ansi::input::mio_poller

use crate::{ControlledChild, PtyPair, PtyTestMode,
            core::resilient_reactor_thread::LivenessState,
            direct_to_ansi::{DirectToAnsiInputDevice, input::global_input_resource},
            generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "LIFECYCLE_TEST_READY";

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
    controller: lifecycle_controller_entry_point,
    controlled: lifecycle_controlled_entry_point,
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
fn lifecycle_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("🚀 Lifecycle Controller: Starting...");

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
    eprintln!("📝 Lifecycle Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, CONTROLLED_READY);
    eprintln!("  ✓ Controlled is ready");

    // Wait for device A to be created, then send input.
    wait_for_signal(&mut buf_reader, DEVICE_A_CREATED);
    eprintln!("📝 Lifecycle Controller: Sending input for device A...");
    writer.write_all(b"a").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for device A to be dropped.
    wait_for_signal(&mut buf_reader, DEVICE_A_DROPPED);
    eprintln!("  ✓ Device A dropped, thread should have exited");

    // Wait for device B to be created, then send input.
    wait_for_signal(&mut buf_reader, DEVICE_B_CREATED);
    eprintln!("📝 Lifecycle Controller: Sending input for device B...");
    writer.write_all(b"b").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  ✓ Test passed signal received");

    // Clean up.
    drop(writer);
    match child.wait() {
        Ok(status) => eprintln!("✅ Lifecycle Controller: Controlled exited: {status:?}"),
        Err(e) => panic!("Failed to wait for controlled: {e}"),
    }

    eprintln!("✅ Lifecycle Controller: Test passed!");
}

/// Controlled process: tests thread lifecycle with assertions.
fn lifecycle_controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("🔍 Lifecycle Controlled: Starting lifecycle test...");

        // Step 1: Verify initial state (no thread yet).
        eprintln!("📍 Step 1: Checking initial state...");
        assert_eq!(
            global_input_resource::SINGLETON.is_thread_running(),
            LivenessState::Terminated,
            "Expected thread_alive = Dead initially"
        );
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 initially"
        );
        eprintln!("  ✓ Initial state: thread_alive=false, receiver_count=0");

        // Step 2: Create device A - this spawns thread #1.
        eprintln!("📍 Step 2: Creating device A...");
        let mut device_a = DirectToAnsiInputDevice::new();

        // Signal that we're ready for input, then read.
        println!("{DEVICE_A_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device A.
        let event_a = tokio::time::timeout(Duration::from_secs(5), device_a.next())
            .await
            .expect("Timeout reading from device A");
        eprintln!("  ✓ Device A received event: {event_a:?}");

        // Verify thread is alive and capture generation.
        assert_eq!(
            global_input_resource::SINGLETON.is_thread_running(),
            LivenessState::Running,
            "Expected thread_alive = Alive after device A created"
        );
        assert_eq!(
            global_input_resource::SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device A subscribed"
        );
        let generation_a = global_input_resource::SINGLETON.get_thread_generation();
        eprintln!("  ✓ After device A: thread_alive=true, receiver_count=1, generation={generation_a}");

        // Step 3: Drop device A - this should cause thread #1 to exit.
        eprintln!("📍 Step 3: Dropping device A...");
        drop(device_a);

        // Give thread time to detect no receivers and exit.
        // With mio::Waker, thread should exit nearly instantaneously.
        eprintln!("  ⏳ Waiting for thread to exit...");
        let mut thread_exited = false;
        for i in 0..100 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            if global_input_resource::SINGLETON.is_thread_running() == LivenessState::Terminated {
                eprintln!("  ✓ Thread exited after {}ms", i + 1);
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
        eprintln!("  ✓ After device A dropped: thread_alive=false, receiver_count=0");

        println!("{DEVICE_A_DROPPED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Step 4: Create device B - this should spawn thread #2.
        eprintln!("📍 Step 4: Creating device B (should spawn new thread)...");
        let mut device_b = DirectToAnsiInputDevice::new();

        println!("{DEVICE_B_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Read one event from device B.
        let event_b = tokio::time::timeout(Duration::from_secs(5), device_b.next())
            .await
            .expect("Timeout reading from device B");
        eprintln!("  ✓ Device B received event: {event_b:?}");

        // Verify NEW thread is alive with a NEW generation.
        assert_eq!(
            global_input_resource::SINGLETON.is_thread_running(),
            LivenessState::Running,
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
            "  ✓ After device B: thread_alive=true, receiver_count=1, generation={generation_b} (NEW thread!)"
        );

        // All assertions passed!
        eprintln!("🎉 All lifecycle assertions passed!");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Clean up.
        drop(device_b);
    });

    eprintln!("🔍 Lifecycle Controlled: Exiting");
    std::process::exit(0);
}

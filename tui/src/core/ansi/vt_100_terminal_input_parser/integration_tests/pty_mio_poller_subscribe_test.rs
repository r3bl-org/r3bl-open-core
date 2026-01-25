// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY-based integration test for [`subscribe()`] multi-receiver functionality.
//!
//! Tests that [`DirectToAnsiInputDevice::subscribe()`] creates additional receivers
//! that independently receive all input events via the broadcast channel.
//!
//! Run with: `cargo test -p r3bl_tui --lib test_pty_mio_poller_subscribe -- --nocapture`
//!
//! Tests that:
//! 1. Thread spawns on first device (`receiver_count = 1`)
//! 2. `subscribe()` creates additional receiver (`receiver_count = 2`)
//! 3. Both receivers get the SAME input event (broadcast semantics)
//! 4. Dropping subscriber decrements count (`receiver_count = 1`)
//! 5. Thread stays alive while device exists
//! 6. Thread exits when device drops (`receiver_count = 0`)
//!
//! ## Test Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ Controlled Process (in PTY)                                                 │
//! │                                                                             │
//! │  1. Assert: thread_alive = false, receiver_count = 0 (initial state)        │
//! │  2. Create DirectToAnsiInputDevice                                          │
//! │     Assert: receiver_count = 1                                              │
//! │  3. Call device.subscribe() to get second handle                            │
//! │     Assert: receiver_count = 2                                              │
//! │  4. Read input from BOTH handles - verify BOTH receive same event           │
//! │  5. Drop subscriber handle                                                  │
//! │     Assert: receiver_count = 1, thread_alive = true                         │
//! │  6. Read input from device (proves thread still works)                      │
//! │  7. Drop device                                                             │
//! │     Assert: thread_alive = false, receiver_count = 0                        │
//! │                                                                             │
//! │  If all assertions pass → TEST_PASSED                                       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! [`subscribe()`]: crate::direct_to_ansi::DirectToAnsiInputDevice::subscribe

use crate::{ControlledChild, PtyPair,
            core::resilient_reactor_thread::LivenessState,
            direct_to_ansi::{DirectToAnsiInputDevice,
                             input::{channel_types::{PollerEvent, StdinEvent},
                                     global_input_resource::SINGLETON}},
            generate_pty_test};
use std::{io::{BufRead, BufReader, Write},
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const CONTROLLED_READY: &str = "SUBSCRIBE_TEST_READY";

/// Signal sent when device and subscriber are created.
const SUBSCRIBERS_CREATED: &str = "SUBSCRIBERS_CREATED";

/// Signal sent after subscriber is dropped.
const SUBSCRIBER_DROPPED: &str = "SUBSCRIBER_DROPPED";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "SUBSCRIBE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_subscribe,
    controller: subscribe_controller_entry_point,
    controlled: subscribe_controlled_entry_point
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
fn subscribe_controller_entry_point(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("Subscribe Controller: Starting...");

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
    eprintln!("Subscribe Controller: Waiting for controlled to start...");
    wait_for_signal(&mut buf_reader, CONTROLLED_READY);
    eprintln!("  Controlled is ready");

    // Wait for both subscribers to be created, then send input.
    wait_for_signal(&mut buf_reader, SUBSCRIBERS_CREATED);
    eprintln!("Subscribe Controller: Sending input 'x' for both receivers...");
    writer.write_all(b"x").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for subscriber to be dropped, then send more input.
    wait_for_signal(&mut buf_reader, SUBSCRIBER_DROPPED);
    eprintln!("Subscribe Controller: Sending input 'y' for remaining device...");
    writer.write_all(b"y").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    wait_for_signal(&mut buf_reader, TEST_PASSED);
    eprintln!("  Test passed signal received");

    // Clean up.
    drop(writer);
    match child.wait() {
        Ok(status) => eprintln!("Subscribe Controller: Controlled exited: {status:?}"),
        Err(e) => panic!("Failed to wait for controlled: {e}"),
    }

    eprintln!("Subscribe Controller: Test passed!");
}

/// Controlled process: tests [`subscribe()`] multi-receiver functionality.
#[allow(clippy::too_many_lines)]
fn subscribe_controlled_entry_point() -> ! {
    println!("{CONTROLLED_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // Enable raw mode for proper input handling.
    eprintln!("Subscribe Controlled: Setting terminal to raw mode...");
    if let Err(e) = crate::core::ansi::terminal_raw_mode::enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {e}");
    }

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("Subscribe Controlled: Starting subscribe test...");

        // Step 1: Verify initial state (no thread yet).
        eprintln!("Step 1: Checking initial state...");
        assert_eq!(
            SINGLETON.is_thread_running(),
            LivenessState::Terminated,
            "Expected thread_alive = Dead initially"
        );
        assert_eq!(
            SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 initially"
        );
        eprintln!("  Initial state: thread_alive=Dead, receiver_count=0");

        // Step 2: Create device - this spawns the thread.
        eprintln!("Step 2: Creating device...");
        let mut device = DirectToAnsiInputDevice::new();
        assert_eq!(
            SINGLETON.is_thread_running(),
            LivenessState::Running,
            "Expected thread_alive = Alive after device created"
        );
        assert_eq!(
            SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device created"
        );
        eprintln!("  Device created: thread_alive=true, receiver_count=1");

        // Step 3: Create subscriber via subscribe().
        eprintln!("Step 3: Creating subscriber via device.subscribe()...");
        let mut subscriber = device.subscribe();
        assert_eq!(
            SINGLETON.get_receiver_count(),
            2,
            "Expected receiver_count = 2 after subscribe()"
        );
        eprintln!("  Subscriber created: receiver_count=2");

        // Signal that we're ready for input.
        println!("{SUBSCRIBERS_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Step 4: Read from BOTH - they should both receive the same event.
        eprintln!("Step 4: Reading from both device and subscriber...");

        // Read from device.
        let event_device =
            tokio::time::timeout(Duration::from_secs(5), device.next())
                .await
                .expect("Timeout reading from device");
        eprintln!("  Device received: {event_device:?}");

        // Read from subscriber (using the raw receiver).
        let subscriber_rx = subscriber
            .receiver
            .as_mut()
            .expect("Subscriber receiver is None");
        let msg: PollerEvent =
            tokio::time::timeout(Duration::from_secs(5), subscriber_rx.recv())
                .await
                .expect("Timeout reading from subscriber")
                .expect("Channel closed");
        let PollerEvent::Stdin(StdinEvent::Input(event)) = msg else {
            panic!("Expected Stdin(Input(_)), got {msg:?}")
        };
        let event_subscriber = Some(event);
        eprintln!("  Subscriber received: {event_subscriber:?}");

        // Both should have received the same event.
        assert_eq!(
            event_device, event_subscriber,
            "Both receivers should get the same event (broadcast semantics)"
        );
        eprintln!("  Both receivers got the same event!");

        // Step 5: Drop subscriber - thread should stay alive for device.
        eprintln!("Step 5: Dropping subscriber...");
        drop(subscriber);

        // Give a moment for the drop to propagate.
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(
            SINGLETON.is_thread_running(),
            LivenessState::Running,
            "Expected thread_alive = Alive after subscriber dropped (device still exists)"
        );
        assert_eq!(
            SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after subscriber dropped"
        );
        eprintln!("  After subscriber drop: thread_alive=Alive, receiver_count=1");

        println!("{SUBSCRIBER_DROPPED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Step 6: Device can still read events.
        eprintln!("Step 6: Reading from device after subscriber dropped...");
        let event_after_drop =
            tokio::time::timeout(Duration::from_secs(5), device.next())
                .await
                .expect("Timeout reading from device after subscriber drop");
        eprintln!("  Device received after subscriber drop: {event_after_drop:?}");

        // Step 7: Drop device - thread should exit.
        eprintln!("Step 7: Dropping device...");
        drop(device);

        // Wait for thread to exit.
        let mut thread_exited = false;
        for i in 0..100 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            if SINGLETON.is_thread_running() == LivenessState::Terminated {
                eprintln!("  Thread exited after {}ms", i + 1);
                thread_exited = true;
                break;
            }
        }

        assert!(thread_exited, "Thread did not exit within 100ms");
        assert_eq!(
            SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 after device dropped"
        );
        eprintln!("  After device drop: thread_alive=false, receiver_count=0");

        // All assertions passed!
        eprintln!("All subscribe test assertions passed!");
        println!("{TEST_PASSED}");
        std::io::stdout().flush().expect("Failed to flush");
    });

    // Disable raw mode.
    if let Err(e) = crate::core::ansi::terminal_raw_mode::disable_raw_mode() {
        eprintln!("Failed to disable raw mode: {e}");
    }

    eprintln!("Subscribe Controlled: Exiting");
    std::process::exit(0);
}

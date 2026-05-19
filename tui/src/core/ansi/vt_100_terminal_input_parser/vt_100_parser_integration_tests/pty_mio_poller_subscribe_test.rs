// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for guard-centric multi-receiver functionality.
//!
//! Tests that [`InputSubscriberGuard::try_subscribe()`] creates additional receivers that
//! independently receive all input events via the broadcast channel.
//!
//! Tests that:
//! 1. Thread spawns on first device (`receiver_count = 1`)
//! 2. Guard-centric subscribe creates additional receiver (`receiver_count = 2`)
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
//! │  3. Call device.try_subscribe(), then guard.try_subscribe() for peer handle   │
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
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_mio_poller_subscribe -- --nocapture
//! ```
//!
//! [`InputSubscriberGuard::try_subscribe()`]:
//!     crate::direct_to_ansi::input::InputSubscriberGuard::try_subscribe
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{PtyTestContext, PtyTestMode,
            core::resilient_reactor_thread::{RRTEvent, ThreadState},
            direct_to_ansi::{DirectToAnsiInputDevice,
                             input::{channel_types::{PollerEvent, StdinEvent},
                                     global_input_resource::SINGLETON}},
            generate_pty_test};
use std::{io::Write,
          time::Duration};

/// Ready signal sent by controlled process after initialization.
const SUBSCRIBE_READY: &str = "SUBSCRIBE_TEST_READY";

/// Signal sent when device and subscriber are created.
const SUBSCRIBERS_CREATED: &str = "SUBSCRIBERS_CREATED";

/// Signal sent after subscriber is dropped.
const SUBSCRIBER_DROPPED: &str = "SUBSCRIBER_DROPPED";

/// Signal sent when test completes successfully.
const TEST_PASSED: &str = "SUBSCRIBE_TEST_PASSED";

generate_pty_test! {
    test_fn: test_pty_mio_poller_subscribe,
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

    eprintln!("Subscribe Controller: Starting...");

    // Wait for controlled to be ready.
    eprintln!("Subscribe Controller: Waiting for controlled to start...");
    child
        .wait_for_ready(&mut buf_reader, SUBSCRIBE_READY)
        .expect("Failed to wait for SUBSCRIBE_READY");
    eprintln!("  Controlled is ready");

    // Wait for both subscribers to be created, then send input.
    child
        .wait_for_ready(&mut buf_reader, SUBSCRIBERS_CREATED)
        .expect("Failed to wait for SUBSCRIBERS_CREATED");
    eprintln!("Subscribe Controller: Sending input 'x' for both receivers...");
    writer.write_all(b"x").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for subscriber to be dropped, then send more input.
    child
        .wait_for_ready(&mut buf_reader, SUBSCRIBER_DROPPED)
        .expect("Failed to wait for SUBSCRIBER_DROPPED");
    eprintln!("Subscribe Controller: Sending input 'y' for remaining device...");
    writer.write_all(b"y").expect("Failed to write");
    writer.flush().expect("Failed to flush");

    // Wait for test to pass.
    child
        .wait_for_ready(&mut buf_reader, TEST_PASSED)
        .expect("Failed to wait for TEST_PASSED");
    eprintln!("  Test passed signal received");

    // Clean up.
    drop(writer);
    child.drain_and_wait(buf_reader, pty_pair);

    eprintln!("Subscribe Controller: Test passed!");
}

/// Controlled process: tests guard-centric multi-receiver functionality. The harness
/// performs [`std::process::exit(0)`] after this function returns.
#[allow(clippy::too_many_lines)]
fn controlled() {
    println!("{SUBSCRIBE_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        eprintln!("Subscribe Controlled: Starting subscribe test...");
        let is_running = || {
            matches!(
                *SINGLETON.shared_state.lock(),
                ThreadState::Running(_)
            )
        };
        let is_stopped = || {
            matches!(
                *SINGLETON.shared_state.lock(),
                ThreadState::Stopped
            )
        };

        // Step 1: Verify initial state (no thread yet).
        eprintln!("Step 1: Checking initial state...");
        assert!(is_stopped(), "Expected thread_alive = Dead initially");
        assert_eq!(
            SINGLETON.get_receiver_count(),
            0,
            "Expected receiver_count = 0 initially"
        );
        eprintln!("  Initial state: thread_alive=Dead, receiver_count=0");

        // Step 2: Create device - this spawns the thread.
        eprintln!("Step 2: Creating device...");
        let mut device = DirectToAnsiInputDevice::new()
            .expect("Failed to initialize DirectToAnsiInputDevice");
        assert!(
            is_running(),
            "Expected thread_alive = Alive after device created"
        );
        assert_eq!(
            SINGLETON.get_receiver_count(),
            1,
            "Expected receiver_count = 1 after device created"
        );
        eprintln!("  Device created: thread_alive=true, receiver_count=1");

        // Step 3: Create subscriber via guard-based replication.
        eprintln!("Step 3: Creating subscriber via guard.try_subscribe()...");
        let subscriber_seed = device
            .try_subscribe()
            .expect("Failed to create InputSubscriberGuard from device");
        let mut subscriber = subscriber_seed
            .try_subscribe()
            .expect("Failed to create InputSubscriberGuard from existing guard");
        drop(subscriber_seed);
        assert_eq!(
            SINGLETON.get_receiver_count(),
            2,
            "Expected receiver_count = 2 after guard-based try_subscribe()"
        );
        eprintln!("  Subscriber created: receiver_count=2");

        // Signal that we're ready for input.
        println!("{SUBSCRIBERS_CREATED}");
        std::io::stdout().flush().expect("Failed to flush");

        // Step 4: Read from BOTH - they should both receive the same event.
        eprintln!("Step 4: Reading from both device and subscriber...");

        // Read from device.
        let event_device = tokio::time::timeout(Duration::from_secs(5), device.next())
            .await
            .expect("Timeout reading from device");
        eprintln!("  Device received: {event_device:?}");

        // Read from subscriber (using the raw receiver).
        let rrt_event =
            tokio::time::timeout(Duration::from_secs(5), subscriber.receiver.recv())
                .await
                .expect("Timeout reading from subscriber")
                .expect("Channel closed");
        let RRTEvent::Worker(PollerEvent::Stdin(StdinEvent::Input(event))) = rrt_event
        else {
            panic!("Expected Worker(Stdin(Input(_))), got {rrt_event:?}")
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

        assert!(
            is_running(),
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
            if is_stopped() {
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
}

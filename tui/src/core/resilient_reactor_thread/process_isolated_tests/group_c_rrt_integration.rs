// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::fixtures::*;
use crate::resilient_reactor_thread::{LivenessState, RRT, RRTEvent, ShutdownReason};
use std::{thread::sleep,
          time::{Duration, Instant}};

/// Verify that [`RRT::subscribe()`] spawns a dedicated thread and that it transitions to
/// [`LivenessState::TerminatedOrNotStarted`] after the worker stops.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_spawns_thread() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard = rrt.subscribe().unwrap();

    // Wait for thread to start.
    sleep(Duration::from_millis(50));
    assert_eq!(rrt.is_thread_running(), LivenessState::Running);

    send_cmd(&cmd_sender, b's');
    // Wait for thread to exit.
    sleep(Duration::from_millis(100));
    assert_eq!(
        rrt.is_thread_running(),
        LivenessState::TerminatedOrNotStarted
    );
    teardown_factory();
}

/// Verify that a second [`RRT::subscribe()`] while the thread is still running reuses the
/// existing thread (fast path) and increments the receiver count.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_fast_path_reuse() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard1 = rrt.subscribe().unwrap();

    sleep(Duration::from_millis(50));
    let gen1 = rrt.get_thread_generation();

    // Second subscribe reuses the thread (fast path).
    let _guard2 = rrt.subscribe().unwrap();
    let gen2 = rrt.get_thread_generation();

    assert_eq!(gen1, gen2, "Expected same generation (thread reuse)");
    assert_eq!(rrt.get_receiver_count(), 2);

    send_cmd(&cmd_sender, b's');
    sleep(Duration::from_millis(100));
    teardown_factory();
}

/// Verify that [`RRT::subscribe()`] after thread termination launches a new thread (slow
/// path) with a new generation number.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_slow_path_after_termination() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (worker2, wake_fn2, cmd_sender2) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Ok((worker1, wake_fn1)), Some(cmd_sender1.clone())),
            (Ok((worker2, wake_fn2)), Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.subscribe().unwrap();
        sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        send_cmd(&cmd_sender1, b's');
        sleep(Duration::from_millis(100));
        assert_eq!(
            rrt.is_thread_running(),
            LivenessState::TerminatedOrNotStarted
        );

        // Second subscribe after termination (slow path).
        let _guard2 = rrt.subscribe().unwrap();
        sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation (thread relaunch)");

        send_cmd(&cmd_sender2, b's');
        sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

/// Verify that a subscriber receives the [`ShutdownReason::RestartPolicyExhausted`] event
/// when the restart budget is exhausted.
///
/// # Panics
///
/// Panics on assertion failure, if the shutdown event is not received within 5 seconds,
/// or if test infrastructure (mutex, channel) fails.
pub fn test_shutdown_received_by_subscriber() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(0),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let guard = rrt.subscribe().unwrap();
    let mut receiver = guard.receiver.resubscribe();

    sleep(Duration::from_millis(50));

    // Worker returns Restart, budget=0 -> immediate exhaustion.
    send_cmd(&cmd_sender, b'r');

    // Wait for the shutdown event.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found_shutdown = false;
    while Instant::now() < deadline {
        match receiver.try_recv() {
            Ok(RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { .. })) => {
                found_shutdown = true;
                break;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                sleep(Duration::from_millis(10));
            }
            _ => {}
        }
    }
    assert!(found_shutdown, "Subscriber should receive Shutdown event");
    teardown_factory();
}

/// Verify that [`RRT::subscribe()`] succeeds after a worker panic, launching a new thread
/// with a new generation.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_after_panic_recovery() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (worker2, wake_fn2, cmd_sender2) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Ok((worker1, wake_fn1)), Some(cmd_sender1.clone())),
            (Ok((worker2, wake_fn2)), Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.subscribe().unwrap();
        sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        // Cause a panic.
        send_cmd(&cmd_sender1, b'p');
        sleep(Duration::from_millis(100));
        assert_eq!(
            rrt.is_thread_running(),
            LivenessState::TerminatedOrNotStarted
        );

        // Subscribe again after panic (should relaunch).
        let _guard2 = rrt.subscribe().unwrap();
        sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation after panic recovery");
        assert_eq!(rrt.is_thread_running(), LivenessState::Running);

        send_cmd(&cmd_sender2, b's');
        sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

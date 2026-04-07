// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::fixtures::*;
use crate::resilient_reactor_thread::{RRT, RRTEvent, ShutdownReason, ThreadState};
use std::time::{Duration, Instant};

fn is_running(rrt: &RRT<TestWorker>) -> bool {
    matches!(*rrt.shared_state.lock(), ThreadState::Running(_))
}

fn is_stopped(rrt: &RRT<TestWorker>) -> bool {
    matches!(*rrt.shared_state.lock(), ThreadState::Stopped)
}

/// Verify that [`RRT::try_subscribe()`] spawns a dedicated thread and that it transitions
/// to `ThreadState::Stopped` after the worker stops.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_spawns_thread() {
    let (worker, interrupt, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, interrupt)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard = rrt.try_subscribe().unwrap();

    // Should be running immediately after try_subscribe returns Ok.
    assert!(is_running(&rrt));

    send_cmd(&cmd_sender, b's');
    // Wait for thread to exit (transition to Stopped).
    {
        let mut state_guard = rrt.shared_state.lock();
        while !matches!(*state_guard, ThreadState::Stopped) {
            state_guard = rrt.shared_state.wait(state_guard);
        }
    }
    assert!(is_stopped(&rrt));
    teardown_factory();
}

/// Verify that a second [`RRT::try_subscribe()`] while the thread is still running reuses
/// the existing thread (fast path) and increments the receiver count.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_fast_path_reuse() {
    let (worker, interrupt, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, interrupt)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard1 = rrt.try_subscribe().unwrap();

    let gen1 = rrt.get_thread_generation();

    // Second subscribe reuses the thread (fast path).
    let _guard2 = rrt.try_subscribe().unwrap();
    let gen2 = rrt.get_thread_generation();

    assert_eq!(gen1, gen2, "Expected same generation (thread reuse)");
    assert_eq!(rrt.get_receiver_count(), 2);

    send_cmd(&cmd_sender, b's');
    // Wait for thread to exit (transition to Stopped).
    {
        let mut state_guard = rrt.shared_state.lock();
        while !matches!(*state_guard, ThreadState::Stopped) {
            state_guard = rrt.shared_state.wait(state_guard);
        }
    }
    teardown_factory();
}

/// Verify that [`RRT::try_subscribe()`] after thread termination launches a new thread
/// (slow path) with a new generation number.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_slow_path_after_termination() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (worker2, interrupt2, cmd_sender2) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Ok((worker1, interrupt1)), Some(cmd_sender1.clone())),
            (Ok((worker2, interrupt2)), Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.try_subscribe().unwrap();
        let gen1 = rrt.get_thread_generation();

        send_cmd(&cmd_sender1, b's');
        // Wait for thread to exit (transition to Stopped).
        {
            let mut state_guard = rrt.shared_state.lock();
            while !matches!(*state_guard, ThreadState::Stopped) {
                state_guard = rrt.shared_state.wait(state_guard);
            }
        }
        assert!(is_stopped(&rrt));

        // Second subscribe after termination (slow path).
        let _guard2 = rrt.try_subscribe().unwrap();
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation (thread relaunch)");

        send_cmd(&cmd_sender2, b's');
        // Wait for thread to exit (transition to Stopped).
        {
            let mut state_guard = rrt.shared_state.lock();
            while !matches!(*state_guard, ThreadState::Stopped) {
                state_guard = rrt.shared_state.wait(state_guard);
            }
        }
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
    let (worker, interrupt, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, interrupt)), Some(cmd_sender.clone()))],
        no_delay_policy(0),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let guard = rrt.try_subscribe().unwrap();
    let mut receiver = guard.receiver.resubscribe();

    // Worker returns Restart, budget=0 -> immediate exhaustion.
    send_cmd(&cmd_sender, b'r');

    // Wait for the shutdown event.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let found_shutdown = rt.block_on(async {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(Ok(RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted {
                ..
            }))) =
                tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await
            {
                return true;
            }
        }
        false
    });

    assert!(found_shutdown, "Subscriber should receive Shutdown event");
    teardown_factory();
}

/// Verify that [`RRT::try_subscribe()`] succeeds after a worker panic, launching a new
/// thread with a new generation.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_subscribe_after_panic_recovery() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (worker2, interrupt2, cmd_sender2) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Ok((worker1, interrupt1)), Some(cmd_sender1.clone())),
            (Ok((worker2, interrupt2)), Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.try_subscribe().unwrap();
        let gen1 = rrt.get_thread_generation();

        // Cause a panic.
        send_cmd(&cmd_sender1, b'p');
        // Wait for thread to exit (transition to Stopped).
        {
            let mut state_guard = rrt.shared_state.lock();
            while !matches!(*state_guard, ThreadState::Stopped) {
                state_guard = rrt.shared_state.wait(state_guard);
            }
        }
        assert!(is_stopped(&rrt));

        // Subscribe again after panic (should relaunch).
        let _guard2 = rrt.try_subscribe().unwrap();
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation after panic recovery");
        assert!(is_running(&rrt));

        send_cmd(&cmd_sender2, b's');
        // Wait for thread to exit (transition to Stopped).
        {
            let mut state_guard = rrt.shared_state.lock();
            while !matches!(*state_guard, ThreadState::Stopped) {
                state_guard = rrt.shared_state.wait(state_guard);
            }
        }
    }
    teardown_factory();
}

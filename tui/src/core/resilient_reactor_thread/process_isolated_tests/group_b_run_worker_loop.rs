// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::fixtures::*;
use crate::resilient_reactor_thread::{RRTEvent, ShutdownReason, ThreadState};
use std::{sync::{Arc, atomic::Ordering},
          thread::sleep,
          time::{Duration, Instant}};

/// Verify that sending `Stop` causes the worker loop to exit and clear the software
/// interrupt handle slot.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_worker_stop_exits_cleanly() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b's');
    handle.join().unwrap();

    assert!(matches!(*shared_state.lock(), ThreadState::Stopped));
    teardown_factory();
}

/// Verify that `Continue` keeps the loop running and events are emitted before `Stop`
/// terminates it.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_worker_continue_then_stop() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b's');
    handle.join().unwrap();

    let mut count = 0;
    while receiver.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 3);
    teardown_factory();
}

/// Verify that domain events sent by the worker are received in order on the broadcast
/// channel.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_domain_events_flow_through() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b's');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(0)) => {}
        other => panic!("Expected TestEvent(0), got {other:?}"),
    }
    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(1)) => {}
        other => panic!("Expected TestEvent(1), got {other:?}"),
    }
    teardown_factory();
}

/// Verify that a single restart creates a new worker and the loop continues.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_single_restart_success() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    // Worker1: restart.
    send_cmd(&cmd_sender1, b'r');
    // Wait for create() to be called.
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // Worker2: stop.
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 1);
    assert!(matches!(*shared_state.lock(), ThreadState::Stopped));
    teardown_factory();
}

/// Verify that restart with no delay policy completes quickly (under 500ms).
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_restart_no_delay_fast() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert!(start.elapsed() < Duration::from_millis(500));
    teardown_factory();
}

/// Verify that events emitted before and after a restart are both received.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_events_before_and_after_restart() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    // Worker1: event then restart.
    send_cmd(&cmd_sender1, b'e');
    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // Worker2: event then stop.
    send_cmd(&cmd_sender2, b'e');
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    // Both events should arrive (from different worker instances).
    let e1 = receiver.try_recv().unwrap();
    let e2 = receiver.try_recv().unwrap();
    assert!(matches!(e1, RRTEvent::Worker(TestEvent(_))));
    assert!(matches!(e2, RRTEvent::Worker(TestEvent(_))));
    teardown_factory();
}

/// Verify that the software interrupt handle slot is swapped to a new handle after restart.
///
/// # Panics
///
/// Panics on assertion failure, if the handle is not swapped within 5 seconds, or if test
/// infrastructure (mutex, channel, notify) fails.
pub fn test_interrupt_handle_swap_on_restart() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    // Invoke the current software interrupt to record its ID in LAST_INTERRUPT_ID.
    shared_state.interrupt_if_running();
    let old_id = LAST_INTERRUPT_ID.load(Ordering::SeqCst);

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();

    // Wait for the interrupt handle swap (happens right after create() returns).
    // Each TestInterrupt records its unique ID in LAST_INTERRUPT_ID when
    // trigger_software_interrupt() is called.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        sleep(Duration::from_millis(1));
        shared_state.interrupt_if_running();
        if LAST_INTERRUPT_ID.load(Ordering::SeqCst) != old_id {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Interrupt handle should have been swapped on restart"
        );
    }

    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();
    teardown_factory();
}

/// Verify that the restart budget resets after each successful
/// `create_and_register_os_sources()` call, allowing unlimited sequential restarts.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_budget_resets_on_successful_create() {
    // max_restarts=1, but each successful create resets the budget.
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();
    let (ok3, cmd_sender3) = create_ok_result();
    let (ok4, cmd_sender4) = create_ok_result();

    let (notify_receiver, _senders) = setup_factory(
        vec![
            (ok2, Some(cmd_sender2.clone())),
            (ok3, Some(cmd_sender3.clone())),
            (ok4, Some(cmd_sender4.clone())),
        ],
        no_delay_policy(1),
    );

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    // W1 -> restart -> W2 created (budget resets).
    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // W2 -> restart -> W3 created (budget resets again).
    send_cmd(&cmd_sender2, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // W3 -> restart -> W4 created (budget resets again).
    send_cmd(&cmd_sender3, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // W4 -> stop.
    send_cmd(&cmd_sender4, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 3);
    assert!(matches!(*shared_state.lock(), ThreadState::Stopped));
    teardown_factory();
}

/// Verify that exhausting the restart budget (factory returns errors) emits a
/// [`ShutdownReason::RestartPolicyExhausted`] event.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_restart_exhaustion() {
    // max=2. W1 and W2 restart OK (budget resets each time). W3 restarts but
    // factory is empty -> create() fails repeatedly -> exhausts budget.
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();
    let (ok3, cmd_sender3) = create_ok_result();

    let (notify_receiver, _senders) = setup_factory(
        vec![
            (ok2, Some(cmd_sender2.clone())),
            (ok3, Some(cmd_sender3.clone())),
        ],
        no_delay_policy(2),
    );

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    // W3 restarts, factory is empty -> create() returns Err -> budget exhausted.
    send_cmd(&cmd_sender3, b'r');
    handle.join().unwrap();

    // Should have received a Shutdown event.
    let mut found_shutdown = false;
    while let Ok(event) = receiver.try_recv() {
        if let RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { .. }) = event {
            found_shutdown = true;
        }
    }
    assert!(found_shutdown, "Expected Shutdown(RestartPolicyExhausted)");
    teardown_factory();
}

/// Verify that a zero restart budget causes immediate exhaustion on the first restart
/// attempt, without calling `create_and_register_os_sources()`.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_zero_budget_immediate_exhaustion() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(0));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { attempts: 1 }) => {}
        other => panic!("Expected Shutdown(attempts=1), got {other:?}"),
    }
    // create() never called (budget=0 means immediate exhaustion).
    assert_eq!(get_create_count(), 0);
    teardown_factory();
}

/// Verify that the [`ShutdownReason::RestartPolicyExhausted`] payload contains the
/// correct attempt count.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_shutdown_event_payload() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(0));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { attempts: 1 }) => {}
        other => panic!("Expected Shutdown(attempts=1), got {other:?}"),
    }
    teardown_factory();
}

/// Verify that a transient `create_and_register_os_sources()` failure is retried and
/// succeeds on the next attempt.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_create_failure_then_success() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("transient error")), None),
            (ok2, Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    // Wait for second create() call (first fails, second succeeds).
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 2);
    teardown_factory();
}

/// Verify that persistent `create_and_register_os_sources()` failures exhaust the restart
/// budget and emit a shutdown event.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_persistent_create_failure() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("fail 1")), None),
            (Err(miette::miette!("fail 2")), None),
            (Err(miette::miette!("fail 3")), None),
        ],
        no_delay_policy(3),
    );

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    handle.join().unwrap();

    let mut found_shutdown = false;
    while let Ok(event) = receiver.try_recv() {
        if matches!(
            event,
            RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { .. })
        ) {
            found_shutdown = true;
        }
    }
    assert!(found_shutdown, "Expected Shutdown(RestartPolicyExhausted)");
    // create() called 3 times (all failed) + default fallback calls.
    assert!(get_create_count() >= 3);
    teardown_factory();
}

/// Verify that the backoff delay from [`RestartPolicy::initial_delay`] is applied between
/// restart attempts (at least 50ms).
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
///
/// [`RestartPolicy::initial_delay`]: field@crate::RestartPolicy::initial_delay
pub fn test_backoff_delay_applied() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let policy = crate::resilient_reactor_thread::RestartPolicy {
        max_restarts: 1,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: None,
        max_delay: None,
    };
    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], policy);

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert!(start.elapsed() >= Duration::from_millis(50));
    teardown_factory();
}

/// Verify that the backoff delay resets to `initial_delay` after a successful
/// `create_and_register_os_sources()`, rather than continuing to grow.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel, notify) fails.
pub fn test_delay_resets_after_successful_create() {
    let (worker1, interrupt1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();
    let (ok3, cmd_sender3) = create_ok_result();

    let policy = crate::resilient_reactor_thread::RestartPolicy {
        max_restarts: 3,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: Some(2.0),
        max_delay: None,
    };
    let (notify_receiver, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("transient")), None),
            (ok2, Some(cmd_sender2.clone())),
            (ok3, Some(cmd_sender3.clone())),
        ],
        policy,
    );

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt1);

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender3, b's');
    handle.join().unwrap();

    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(150) && elapsed < Duration::from_millis(500),
        "Expected ~200ms, got {elapsed:?}"
    );
    teardown_factory();
}

/// Verify that a worker panic emits [`ShutdownReason::Panic`].
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_panic_sends_shutdown_panic() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

/// Verify that events emitted before a panic are still delivered, followed by the
/// [`ShutdownReason::Panic`] event.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_panic_after_events() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(_)) => {}
        other @ RRTEvent::Shutdown(_) => panic!("Expected Worker event, got {other:?}"),
    }
    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

/// Verify that a panic does not trigger a restart - the loop exits immediately without
/// calling `create_and_register_os_sources()`.
///
/// # Panics
///
/// Panics on assertion failure or if test infrastructure (mutex, channel) fails.
pub fn test_no_restart_after_panic() {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let shared_state = create_shared_state(interrupt);

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, Arc::clone(&shared_state));

    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 0);
    teardown_factory();
}

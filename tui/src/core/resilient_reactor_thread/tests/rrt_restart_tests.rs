// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EBADF EINTR mult

//! Tests for RRT self-healing restart logic.
//!
//! - **Group A** (Step 4): Pure function tests for [`advance_backoff_delay`] - no
//!   threads, no OS resources.
//! - **Group B** (Step 5): `run_worker_loop` tests using [`mpsc`] channels.
//! - **Group C** (Step 6): `RRT<TestWorker>` integration tests with real thread spawning.
//! - **Process-isolated coordinator** (Step 3): Groups B and C run in a subprocess to
//!   avoid static state interference between tests.
//!
//! [`advance_backoff_delay`]: super::super::advance_backoff_delay
//! [`mpsc`]: std::sync::mpsc

use super::super::*;
use crate::{Continuation, ControlledChild, PtyPair, PtyTestMode, generate_pty_test};
use std::{collections::VecDeque,
          io::{BufRead, BufReader, Write},
          sync::{Arc, Mutex,
                 atomic::{AtomicU32, Ordering},
                 mpsc},
          time::{Duration, Instant}};

/// Simple domain event for tests.
#[derive(Clone, Debug, PartialEq)]
struct TestEvent(u32);

/// Monotonic counter for unique waker IDs.
static NEXT_WAKER_ID: AtomicU32 = AtomicU32::new(0);

/// Tracks the most recently invoked waker's ID.
/// Used by [`test_waker_swap_on_restart()`] to detect waker replacement.
static LAST_WAKED_ID: AtomicU32 = AtomicU32::new(0);

/// Test waker that records its ID in [`LAST_WAKED_ID`] when [`wake()`] is called.
///
/// Each instance gets a unique ID from [`NEXT_WAKER_ID`]. This allows
/// [`test_waker_swap_on_restart()`] to detect that the framework swapped
/// the waker on restart.
///
/// [`wake()`]: RRTWaker::wake
struct TestWaker {
    id: u32,
}

impl RRTWaker for TestWaker {
    fn wake(&self) { LAST_WAKED_ID.store(self.id, Ordering::SeqCst); }
}

/// Test worker driven by an [`mpsc`] command channel.
///
/// Blocks on [`mpsc::Receiver::recv()`] - each call to [`poll_once()`] reads
/// one command byte:
/// - `b'c'`: [`Continuation::Continue`]
/// - `b'r'`: [`Continuation::Restart`]
/// - `b's'`: [`Continuation::Stop`]
/// - `b'e'`: Send [`TestEvent`] and continue
/// - `b'p'`: Panic (for testing `catch_unwind`)
///
/// [`mpsc`]: std::sync::mpsc
/// [`poll_once()`]: RRTWorker::poll_once
struct TestWorker {
    cmd_receiver: mpsc::Receiver<u8>,
    event_counter: u32,
}

impl RRTWorker for TestWorker {
    type Event = TestEvent;

    fn create() -> miette::Result<(Self, impl RRTWaker)> {
        let mut guard = TEST_FACTORY_STATE.lock().unwrap();
        let state = guard.as_mut().expect("TEST_FACTORY_STATE not initialized");
        state.create_count += 1;
        if let Some(ref notify_sender) = state.create_notify {
            notify_sender.send(()).ok();
        }
        state
            .create_results
            .pop_front()
            .unwrap_or_else(|| Err(miette::miette!("TestWorker: no create results")))
    }

    fn poll_once(
        &mut self,
        sender: &tokio::sync::broadcast::Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        match self.cmd_receiver.recv() {
            Ok(b'c') => Continuation::Continue,
            Ok(b'r') => Continuation::Restart,
            Ok(b's') => Continuation::Stop,
            Ok(b'e') => {
                let id = self.event_counter;
                self.event_counter += 1;
                drop(sender.send(RRTEvent::Worker(TestEvent(id))));
                Continuation::Continue
            }
            Ok(b'p') => panic!("TestWorker: deliberate panic for testing"),
            _ => Continuation::Stop,
        }
    }

    fn restart_policy() -> RestartPolicy {
        TEST_FACTORY_STATE
            .lock()
            .unwrap()
            .as_ref()
            .expect("TEST_FACTORY_STATE not initialized")
            .restart_policy
            .clone()
    }
}

/// Shared state controlling [`TestWorker::create()`] behavior.
struct TestFactoryState {
    /// Pre-loaded results. `create()` pops from the front.
    create_results: VecDeque<miette::Result<(TestWorker, TestWaker)>>,

    /// Counter incremented on each `create()` call (before popping).
    create_count: u32,

    /// Restart policy returned by `restart_policy()`.
    restart_policy: RestartPolicy,

    /// Notifies the test thread when `create()` is called.
    create_notify: Option<mpsc::Sender<()>>,
}

static TEST_FACTORY_STATE: Mutex<Option<TestFactoryState>> = Mutex::new(None);

/// Creates `(TestWorker, TestWaker, cmd_sender)`. The test thread uses `cmd_sender` to
/// send command bytes. Each [`TestWaker`] captures a unique ID and records it in
/// [`LAST_WAKED_ID`] when called.
fn create_test_resources() -> (TestWorker, TestWaker, mpsc::Sender<u8>) {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let worker = TestWorker {
        cmd_receiver,
        event_counter: 0,
    };
    let waker_id = NEXT_WAKER_ID.fetch_add(1, Ordering::Relaxed);
    let waker = TestWaker { id: waker_id };
    (worker, waker, cmd_sender)
}

/// Creates resources wrapped in `Ok(...)` for factory pre-loading. Returns the
/// `Ok` result and the corresponding `cmd_sender`.
fn create_ok_result() -> (miette::Result<(TestWorker, TestWaker)>, mpsc::Sender<u8>) {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    (Ok((worker, wake_fn)), cmd_sender)
}

/// Sets up [`TEST_FACTORY_STATE`] with pre-loaded create results and a policy.
/// Returns `(create_notify_receiver, cmd_senders)`.
fn setup_factory(
    results_and_senders: Vec<(
        miette::Result<(TestWorker, TestWaker)>,
        Option<mpsc::Sender<u8>>,
    )>,
    policy: RestartPolicy,
) -> (mpsc::Receiver<()>, Vec<mpsc::Sender<u8>>) {
    let (notify_sender, notify_receiver) = mpsc::channel();
    let mut cmd_senders = Vec::new();
    let mut create_results = VecDeque::new();

    for (result, sender) in results_and_senders {
        create_results.push_back(result);
        if let Some(s) = sender {
            cmd_senders.push(s);
        }
    }

    let mut guard = TEST_FACTORY_STATE.lock().unwrap();
    *guard = Some(TestFactoryState {
        create_results,
        create_count: 0,
        restart_policy: policy,
        create_notify: Some(notify_sender),
    });

    (notify_receiver, cmd_senders)
}

/// Clears [`TEST_FACTORY_STATE`].
fn teardown_factory() {
    let mut guard = TEST_FACTORY_STATE.lock().unwrap();
    *guard = None;
}

/// Reads `create_count` from the factory state.
fn get_create_count() -> u32 {
    TEST_FACTORY_STATE
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.create_count)
        .unwrap_or(0)
}

/// Sends a command byte to the worker via its `cmd_sender`.
fn send_cmd(cmd_sender: &mpsc::Sender<u8>, cmd: u8) { cmd_sender.send(cmd).unwrap(); }

/// Returns a [`RestartPolicy`] with no delays.
fn no_delay_policy(max_restarts: u8) -> RestartPolicy {
    RestartPolicy {
        max_restarts,
        initial_delay: None,
        backoff_multiplier: None,
        max_delay: None,
    }
}

/// Spawns `run_worker_loop::<TestWorker>()` on a named thread.
fn spawn_worker_loop(
    worker: TestWorker,
    sender: SafeSender<TestEvent>,
    safe_waker: SafeWaker,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("test-rrt-worker".into())
        .spawn(move || {
            run_worker_loop::<TestWorker>(worker, sender, safe_waker);
        })
        .unwrap()
}

// XMARK: Process isolated test.

/// Dispatches all restart integration tests sequentially within a single
/// isolated child process.
fn run_all_restart_tests_sequentially() {
    // Group B Step 5.0: Basic lifecycle.
    test_worker_stop_exits_cleanly();
    test_worker_continue_then_stop();
    test_domain_events_flow_through();

    // Group B Step 5.1: Restart success paths.
    test_single_restart_success();
    test_restart_no_delay_fast();
    test_events_before_and_after_restart();
    test_waker_swap_on_restart();
    test_budget_resets_on_successful_create();

    // Group B Step 5.2: Restart exhaustion paths.
    test_restart_exhaustion();
    test_zero_budget_immediate_exhaustion();
    test_shutdown_event_payload();

    // Group B Step 5.3: Worker create() failure paths.
    test_create_failure_then_success();
    test_persistent_create_failure();

    // Group B Step 5.4: TerminationGuard cleanup.
    test_guard_clears_waker_on_stop();
    test_guard_clears_waker_on_exhaustion();

    // Group B Step 5.5: Backoff timing.
    test_backoff_delay_applied();
    test_delay_resets_after_successful_create();

    // Group B Step 5.6: Panic handling.
    test_panic_sends_shutdown_panic();
    test_panic_after_events();
    test_guard_clears_waker_on_panic();
    test_no_restart_after_panic();

    // Group C Step 6: RRT<TestWorker> integration tests.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        test_subscribe_spawns_thread().await;
        test_subscribe_fast_path_reuse().await;
        test_subscribe_slow_path_after_termination().await;
        test_shutdown_received_by_subscriber().await;
        test_subscribe_after_panic_recovery().await;
    });
}

/// Process-isolated test entry point.
#[test]
fn test_rrt_restart_in_isolated_process() {
    crate::suppress_wer_dialogs();
    if std::env::var("ISOLATED_TEST_RUNNER").is_ok() {
        run_all_restart_tests_sequentially();
        std::process::exit(0);
    }

    let mut cmd = crate::new_isolated_test_command();
    cmd.env("ISOLATED_TEST_RUNNER", "1")
        .env("RUST_BACKTRACE", "1")
        .args([
            "--test-threads",
            "1",
            "test_rrt_restart_in_isolated_process",
        ]);

    let output = cmd.output().expect("Failed to run isolated test");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success()
        || stderr.contains("panicked at")
        || stderr.contains("Test failed with error")
    {
        eprintln!("Exit status: {:?}", output.status);
        eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Stderr: {stderr}");

        panic!(
            "Isolated test failed with status code {:?}: {}",
            output.status.code(),
            stderr
        );
    }
}

#[test]
fn test_backoff_exponential_doubling() {
    let policy = RestartPolicy {
        max_restarts: 5,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(2.0),
        max_delay: Some(Duration::from_secs(10)),
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(200));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(400));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(800));
}

#[test]
fn test_backoff_max_delay_capping() {
    let policy = RestartPolicy {
        max_restarts: 5,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(2.0),
        max_delay: Some(Duration::from_millis(300)),
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(200));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(300));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(300));
}

#[test]
fn test_backoff_constant_delay() {
    let policy = RestartPolicy {
        max_restarts: 3,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: None,
        max_delay: None,
    };

    let d1 = advance_backoff_delay(Duration::from_millis(50), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(50));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(50));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(50));
}

#[test]
fn test_backoff_unbounded_growth() {
    let policy = RestartPolicy {
        max_restarts: 10,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(3.0),
        max_delay: None,
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(300));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(900));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(2700));
}

fn test_worker_stop_exits_cleanly() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b's');
    handle.join().unwrap();

    assert!(safe_waker.lock().unwrap().is_none());
    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_worker_continue_then_stop() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

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

fn test_domain_events_flow_through() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

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

fn test_single_restart_success() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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
    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_restart_no_delay_fast() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert!(start.elapsed() < Duration::from_millis(500));
    teardown_factory();
}

fn test_events_before_and_after_restart() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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

fn test_waker_swap_on_restart() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], no_delay_policy(3));

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

    // Invoke the current waker to record its ID in LAST_WAKED_ID.
    {
        let guard = safe_waker.lock().unwrap();
        if let Some(w) = guard.as_ref() {
            w.wake();
        }
    }
    let old_id = LAST_WAKED_ID.load(Ordering::SeqCst);

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();

    // Wait for the waker swap (happens right after create() returns).
    // Each TestWaker records its unique ID in LAST_WAKED_ID when wake() is called.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(1));
        if let Ok(guard) = safe_waker.lock() {
            if let Some(w) = guard.as_ref() {
                w.wake();
                if LAST_WAKED_ID.load(Ordering::SeqCst) != old_id {
                    break;
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "Waker should have been swapped on restart"
        );
    }

    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();
    teardown_factory();
}

fn test_budget_resets_on_successful_create() {
    // max_restarts=1, but each successful create resets the budget.
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
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
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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
    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_restart_exhaustion() {
    // max=2. W1 and W2 restart OK (budget resets each time). W3 restarts but
    // factory is empty -> create() fails repeatedly -> exhausts budget.
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
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
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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

fn test_zero_budget_immediate_exhaustion() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(0));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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

fn test_shutdown_event_payload() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(0));

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

    send_cmd(&cmd_sender1, b'r');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { attempts: 1 }) => {}
        other => panic!("Expected Shutdown(attempts=1), got {other:?}"),
    }
    teardown_factory();
}

fn test_create_failure_then_success() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let (notify_receiver, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("transient error")), None),
            (ok2, Some(cmd_sender2.clone())),
        ],
        no_delay_policy(3),
    );

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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

fn test_persistent_create_failure() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("fail 1")), None),
            (Err(miette::miette!("fail 2")), None),
            (Err(miette::miette!("fail 3")), None),
        ],
        no_delay_policy(3),
    );

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

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

fn test_guard_clears_waker_on_stop() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b's');
    handle.join().unwrap();

    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_guard_clears_waker_on_exhaustion() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(0));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b'r');
    handle.join().unwrap();

    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_backoff_delay_applied() {
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();

    let policy = RestartPolicy {
        max_restarts: 1,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: None,
        max_delay: None,
    };
    let (notify_receiver, _senders) =
        setup_factory(vec![(ok2, Some(cmd_sender2.clone()))], policy);

    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

    send_cmd(&cmd_sender1, b'r');
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b's');
    handle.join().unwrap();

    assert!(start.elapsed() >= Duration::from_millis(50));
    teardown_factory();
}

fn test_delay_resets_after_successful_create() {
    // Policy: delay=50ms, mult=2.0. Pre-load: [Err, Ok(W2), Ok(W3)].
    // W1 restarts:
    //   attempt 1: sleep(50ms) -> create() Err
    //   attempt 2: sleep(100ms) -> create() Ok(W2), budget+delay reset
    // W2 restarts:
    //   attempt 1: sleep(50ms) -> create() Ok(W3), budget+delay reset
    // W3 stops.
    // Total delay ~200ms if reset works, ~250ms if not.
    let (worker1, wake_fn1, cmd_sender1) = create_test_resources();
    let (ok2, cmd_sender2) = create_ok_result();
    let (ok3, cmd_sender3) = create_ok_result();

    let policy = RestartPolicy {
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
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn1))));

    let start = Instant::now();
    let handle = spawn_worker_loop(worker1, sender, safe_waker.clone());

    send_cmd(&cmd_sender1, b'r');
    // Wait for create() calls: first Err, then Ok(W2).
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender2, b'r');
    // Wait for create() Ok(W3).
    notify_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    send_cmd(&cmd_sender3, b's');
    handle.join().unwrap();

    let elapsed = start.elapsed();
    // Total ~200ms (50+100+50). Allow generous bounds.
    assert!(
        elapsed >= Duration::from_millis(150) && elapsed < Duration::from_millis(500),
        "Expected ~200ms, got {elapsed:?}"
    );
    teardown_factory();
}

fn test_panic_sends_shutdown_panic() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

fn test_panic_after_events() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b'e');
    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    // Domain event first.
    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(_)) => {}
        other => panic!("Expected Worker event, got {other:?}"),
    }
    // Then shutdown.
    match receiver.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

fn test_guard_clears_waker_on_panic() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    assert!(safe_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_no_restart_after_panic() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();
    let (sender, _receiver) = tokio::sync::broadcast::channel(16);
    let safe_waker: SafeWaker = Arc::new(Mutex::new(Some(Box::new(wake_fn))));

    let (_notify_receiver, _senders) = setup_factory(vec![], no_delay_policy(3));
    let handle = spawn_worker_loop(worker, sender, safe_waker.clone());

    send_cmd(&cmd_sender, b'p');
    handle.join().unwrap();

    // Factory was never called for restart (panic exits immediately).
    assert_eq!(get_create_count(), 0);
    teardown_factory();
}

// This test runs in a PTY subprocess because `MioPollWorker::create()`
// registers stdin with epoll, which requires a real terminal fd. The PTY
// provides that.

const POLL_ERROR_READY: &str = "POLL_ERROR_READY";
const POLL_ERROR_PASSED: &str = "POLL_ERROR_PASSED";

generate_pty_test! {
    test_fn: test_production_poll_error_sends_error_and_restarts,
    controller: poll_error_controller,
    controlled: poll_error_controlled,
    mode: PtyTestMode::Cooked,
}

/// Waits for a line containing `signal` from the controlled process.
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

/// Controller: waits for the controlled process to complete the poll-error test.
fn poll_error_controller(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("Poll-Error Controller: Starting...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);

    wait_for_signal(&mut buf_reader, POLL_ERROR_READY);
    wait_for_signal(&mut buf_reader, POLL_ERROR_PASSED);

    crate::drain_pty_and_wait(buf_reader, pty_pair, &mut child);
    eprintln!("Poll-Error Controller: Test passed!");
}

/// Controlled: creates a real `MioPollWorker` via `create()`, corrupts the epoll fd,
/// and verifies that `poll_once()` returns `Restart` with `StdinEvent::Error`.
fn poll_error_controlled() -> ! {
    use crate::tui::terminal_lib_backends::direct_to_ansi::input::{channel_types::{PollerEvent,
                                                                                   StdinEvent},
                                                                   mio_poller::MioPollWorker};
    use std::os::unix::io::{AsRawFd, FromRawFd};

    println!("{POLL_ERROR_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // Create worker using the production create() (PTY provides real terminal stdin).
    let (mut worker, _waker) = MioPollWorker::create().unwrap();

    let (sender, mut receiver) =
        tokio::sync::broadcast::channel::<RRTEvent<PollerEvent>>(16);

    // Corrupt the epoll fd so poll() fails with EBADF (non-EINTR).
    // Safety: We intentionally close the fd to trigger the error path.
    // The OwnedFd takes ownership and closes it on drop.
    let raw_fd = worker.poll_handle.as_raw_fd();
    drop(unsafe { std::os::unix::io::OwnedFd::from_raw_fd(raw_fd) });

    let result = worker.poll_once(&sender);

    assert_eq!(result, Continuation::Restart);

    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(PollerEvent::Stdin(StdinEvent::Error)) => {}
        other => panic!("Expected StdinEvent::Error, got {other:?}"),
    }

    println!("{POLL_ERROR_PASSED}");
    std::io::stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}

async fn test_subscribe_spawns_thread() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard = rrt.subscribe().unwrap();

    // Wait for thread to start.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(rrt.is_thread_running(), LivenessState::Running);

    send_cmd(&cmd_sender, b's');
    // Wait for thread to exit.
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);
    teardown_factory();
}

async fn test_subscribe_fast_path_reuse() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let _guard1 = rrt.subscribe().unwrap();

    std::thread::sleep(Duration::from_millis(50));
    let gen1 = rrt.get_thread_generation();

    // Second subscribe reuses the thread (fast path).
    let _guard2 = rrt.subscribe().unwrap();
    let gen2 = rrt.get_thread_generation();

    assert_eq!(gen1, gen2, "Expected same generation (thread reuse)");
    assert_eq!(rrt.get_receiver_count(), 2);

    send_cmd(&cmd_sender, b's');
    std::thread::sleep(Duration::from_millis(100));
    teardown_factory();
}

async fn test_subscribe_slow_path_after_termination() {
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
        std::thread::sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        send_cmd(&cmd_sender1, b's');
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);

        // Second subscribe after termination (slow path).
        let _guard2 = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation (thread relaunch)");

        send_cmd(&cmd_sender2, b's');
        std::thread::sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

async fn test_shutdown_received_by_subscriber() {
    let (worker, wake_fn, cmd_sender) = create_test_resources();

    let (_notify_receiver, _senders) = setup_factory(
        vec![(Ok((worker, wake_fn)), Some(cmd_sender.clone()))],
        no_delay_policy(0),
    );

    let rrt: RRT<TestWorker> = RRT::new();
    let guard = rrt.subscribe().unwrap();
    let mut receiver = guard.maybe_receiver.as_ref().unwrap().resubscribe();

    std::thread::sleep(Duration::from_millis(50));

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
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {}
        }
    }
    assert!(found_shutdown, "Subscriber should receive Shutdown event");
    teardown_factory();
}

async fn test_subscribe_after_panic_recovery() {
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
        std::thread::sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        // Cause a panic.
        send_cmd(&cmd_sender1, b'p');
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);

        // Subscribe again after panic (should relaunch).
        let _guard2 = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation after panic recovery");
        assert_eq!(rrt.is_thread_running(), LivenessState::Running);

        send_cmd(&cmd_sender2, b's');
        std::thread::sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

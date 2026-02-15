// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tests for RRT self-healing restart logic.
//!
//! - **Group A** (Step 4): Pure function tests for [`advance_backoff_delay`] - no
//!   threads, no OS resources.
//! - **Group B** (Step 5): `run_worker_loop` tests using [`mpsc`] channels.
//! - **Group C** (Step 6): `RRT<TestFactory>` integration tests with real thread
//!   spawning.
//! - **Process-isolated coordinator** (Step 3): Groups B and C run in a subprocess
//!   to avoid static state interference between tests.
//!
//! [`advance_backoff_delay`]: super::super::advance_backoff_delay
//! [`mpsc`]: std::sync::mpsc

use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Write},
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use super::super::*;
use crate::{generate_pty_test, ControlledChild, Continuation, PtyPair, PtyTestMode};

/// Simple domain event for tests.
#[derive(Clone, Debug, PartialEq)]
struct TestEvent(u32);

/// Monotonic counter for unique [`TestWaker`] IDs.
static NEXT_WAKER_ID: AtomicU32 = AtomicU32::new(0);

/// No-op waker with a unique ID for identity comparison.
struct TestWaker {
    id: u32,
}

impl RRTWaker for TestWaker {
    fn wake(&self) -> std::io::Result<()> { Ok(()) }
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
/// [`poll_once()`]: RRTWorker::poll_once
/// [`mpsc`]: std::sync::mpsc
struct TestWorker {
    cmd_rx: mpsc::Receiver<u8>,
    event_counter: u32,
}

impl RRTWorker for TestWorker {
    type Event = TestEvent;

    fn poll_once(
        &mut self,
        tx: &tokio::sync::broadcast::Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        match self.cmd_rx.recv() {
            Ok(b'c') => Continuation::Continue,
            Ok(b'r') => Continuation::Restart,
            Ok(b's') => Continuation::Stop,
            Ok(b'e') => {
                let id = self.event_counter;
                self.event_counter += 1;
                drop(tx.send(RRTEvent::Worker(TestEvent(id))));
                Continuation::Continue
            }
            Ok(b'p') => panic!("TestWorker: deliberate panic for testing"),
            _ => Continuation::Stop,
        }
    }
}

/// Shared state controlling [`TestFactory::create()`] behavior.
struct TestFactoryState {
    /// Pre-loaded results. `create()` pops from the front.
    create_results: VecDeque<Result<(TestWorker, TestWaker), miette::Report>>,

    /// Counter incremented on each `create()` call (before popping).
    create_count: u32,

    /// Restart policy returned by `restart_policy()`.
    restart_policy: RestartPolicy,

    /// Notifies the test thread when `create()` is called.
    create_notify: Option<mpsc::Sender<()>>,
}

static TEST_FACTORY_STATE: Mutex<Option<TestFactoryState>> = Mutex::new(None);

/// Factory that serves pre-loaded results from [`TEST_FACTORY_STATE`].
struct TestFactory;

impl RRTFactory for TestFactory {
    type Event = TestEvent;
    type Worker = TestWorker;
    type Waker = TestWaker;

    fn create() -> Result<(Self::Worker, Self::Waker), miette::Report> {
        let mut guard = TEST_FACTORY_STATE.lock().unwrap();
        let state = guard.as_mut().expect("TEST_FACTORY_STATE not initialized");
        state.create_count += 1;
        if let Some(ref notify_tx) = state.create_notify {
            notify_tx.send(()).ok();
        }
        state
            .create_results
            .pop_front()
            .unwrap_or_else(|| Err(miette::miette!("TestFactory: no create results")))
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

/// Creates `(TestWorker, TestWaker, cmd_tx)`. The test thread uses `cmd_tx` to
/// send command bytes.
fn create_test_resources() -> (TestWorker, TestWaker, mpsc::Sender<u8>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker = TestWorker {
        cmd_rx,
        event_counter: 0,
    };
    let waker = TestWaker {
        id: NEXT_WAKER_ID.fetch_add(1, Ordering::Relaxed),
    };
    (worker, waker, cmd_tx)
}

/// Creates resources wrapped in `Ok(...)` for factory pre-loading. Returns the
/// `Ok` result and the corresponding `cmd_tx`.
fn create_ok_result()
-> (Result<(TestWorker, TestWaker), miette::Report>, mpsc::Sender<u8>) {
    let (worker, waker, cmd_tx) = create_test_resources();
    (Ok((worker, waker)), cmd_tx)
}

/// Sets up [`TEST_FACTORY_STATE`] with pre-loaded create results and a policy.
/// Returns `(create_notify_rx, cmd_senders)`.
fn setup_factory(
    results_and_senders: Vec<(
        Result<(TestWorker, TestWaker), miette::Report>,
        Option<mpsc::Sender<u8>>,
    )>,
    policy: RestartPolicy,
) -> (mpsc::Receiver<()>, Vec<mpsc::Sender<u8>>) {
    let (notify_tx, notify_rx) = mpsc::channel();
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
        create_notify: Some(notify_tx),
    });

    (notify_rx, cmd_senders)
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

/// Sends a command byte to the worker via its `cmd_tx`.
fn send_cmd(cmd_tx: &mpsc::Sender<u8>, cmd: u8) {
    cmd_tx.send(cmd).unwrap();
}

/// Returns a [`RestartPolicy`] with no delays.
fn no_delay_policy(max_restarts: u8) -> RestartPolicy {
    RestartPolicy {
        max_restarts,
        initial_delay: None,
        backoff_multiplier: None,
        max_delay: None,
    }
}

/// Spawns `run_worker_loop::<TestFactory>()` on a named thread.
fn spawn_worker_loop(
    worker: TestWorker,
    tx: tokio::sync::broadcast::Sender<RRTEvent<TestEvent>>,
    liveness: Arc<RRTLiveness>,
    shared_waker: Arc<Mutex<Option<TestWaker>>>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("test-rrt-worker".into())
        .spawn(move || {
            run_worker_loop::<TestFactory>(worker, tx, liveness, shared_waker);
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

    // Group B Step 5.3: Factory create() failure paths.
    test_create_failure_then_success();
    test_persistent_create_failure();

    // Group B Step 5.4: TerminationGuard cleanup.
    test_guard_clears_waker_on_stop();
    test_guard_marks_terminated_on_stop();
    test_guard_clears_waker_on_exhaustion();

    // Group B Step 5.5: Backoff timing.
    test_backoff_delay_applied();
    test_delay_resets_after_successful_create();

    // Group B Step 5.6: Panic handling.
    test_panic_sends_shutdown_panic();
    test_panic_after_events();
    test_guard_clears_waker_on_panic();
    test_guard_marks_terminated_on_panic();
    test_no_restart_after_panic();

    // Group C Step 6: RRT<TestFactory> integration tests.
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
        .args(["--test-threads", "1", "test_rrt_restart_in_isolated_process"]);

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
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b's');
    handle.join().unwrap();

    assert_eq!(liveness.is_running(), LivenessState::Terminated);
    assert!(shared_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_worker_continue_then_stop() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b's');
    handle.join().unwrap();

    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 3);
    teardown_factory();
}

fn test_domain_events_flow_through() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b's');
    handle.join().unwrap();

    match rx.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(0)) => {}
        other => panic!("Expected TestEvent(0), got {other:?}"),
    }
    match rx.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(1)) => {}
        other => panic!("Expected TestEvent(1), got {other:?}"),
    }
    teardown_factory();
}

fn test_single_restart_success() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let (notify_rx, _senders) =
        setup_factory(vec![(ok2, Some(cmd_tx2.clone()))], no_delay_policy(3));

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    // Worker1: restart.
    send_cmd(&cmd_tx1, b'r');
    // Wait for create() to be called.
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // Worker2: stop.
    send_cmd(&cmd_tx2, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 1);
    assert_eq!(liveness.is_running(), LivenessState::Terminated);
    teardown_factory();
}

fn test_restart_no_delay_fast() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let (notify_rx, _senders) =
        setup_factory(vec![(ok2, Some(cmd_tx2.clone()))], no_delay_policy(3));

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let start = Instant::now();
    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx2, b's');
    handle.join().unwrap();

    assert!(start.elapsed() < Duration::from_millis(500));
    teardown_factory();
}

fn test_events_before_and_after_restart() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let (notify_rx, _senders) =
        setup_factory(vec![(ok2, Some(cmd_tx2.clone()))], no_delay_policy(3));

    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    // Worker1: event then restart.
    send_cmd(&cmd_tx1, b'e');
    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // Worker2: event then stop.
    send_cmd(&cmd_tx2, b'e');
    send_cmd(&cmd_tx2, b's');
    handle.join().unwrap();

    // Both events should arrive (from different worker instances).
    let e1 = rx.try_recv().unwrap();
    let e2 = rx.try_recv().unwrap();
    assert!(matches!(e1, RRTEvent::Worker(TestEvent(_))));
    assert!(matches!(e2, RRTEvent::Worker(TestEvent(_))));
    teardown_factory();
}

fn test_waker_swap_on_restart() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let (notify_rx, _senders) =
        setup_factory(vec![(ok2, Some(cmd_tx2.clone()))], no_delay_policy(3));

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    // Capture the old waker's ID.
    let old_id = {
        let guard = shared_waker.lock().unwrap();
        guard.as_ref().map(|w| w.id)
    };

    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    // Wait for the waker swap (happens right after create() returns).
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut new_id = old_id;
    while Instant::now() < deadline && new_id == old_id {
        std::thread::sleep(Duration::from_millis(1));
        let guard = shared_waker.lock().unwrap();
        new_id = guard.as_ref().map(|w| w.id);
    }

    assert_ne!(old_id, new_id, "Waker should have been swapped on restart");
    assert!(new_id.is_some());

    send_cmd(&cmd_tx2, b's');
    handle.join().unwrap();
    teardown_factory();
}

fn test_budget_resets_on_successful_create() {
    // max_restarts=1, but each successful create resets the budget.
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();
    let (ok3, cmd_tx3) = create_ok_result();
    let (ok4, cmd_tx4) = create_ok_result();

    let (notify_rx, _senders) = setup_factory(
        vec![
            (ok2, Some(cmd_tx2.clone())),
            (ok3, Some(cmd_tx3.clone())),
            (ok4, Some(cmd_tx4.clone())),
        ],
        no_delay_policy(1),
    );

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    // W1 -> restart -> W2 created (budget resets).
    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // W2 -> restart -> W3 created (budget resets again).
    send_cmd(&cmd_tx2, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // W3 -> restart -> W4 created (budget resets again).
    send_cmd(&cmd_tx3, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // W4 -> stop.
    send_cmd(&cmd_tx4, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 3);
    assert_eq!(liveness.is_running(), LivenessState::Terminated);
    teardown_factory();
}

fn test_restart_exhaustion() {
    // max=2. W1 and W2 restart OK (budget resets each time). W3 restarts but
    // factory is empty -> create() fails repeatedly -> exhausts budget.
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();
    let (ok3, cmd_tx3) = create_ok_result();

    let (notify_rx, _senders) = setup_factory(
        vec![
            (ok2, Some(cmd_tx2.clone())),
            (ok3, Some(cmd_tx3.clone())),
        ],
        no_delay_policy(2),
    );

    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx2, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // W3 restarts, factory is empty -> create() returns Err -> budget exhausted.
    send_cmd(&cmd_tx3, b'r');
    handle.join().unwrap();

    // Should have received a Shutdown event.
    let mut found_shutdown = false;
    while let Ok(event) = rx.try_recv() {
        if let RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { .. }) = event
        {
            found_shutdown = true;
        }
    }
    assert!(found_shutdown, "Expected Shutdown(RestartPolicyExhausted)");
    teardown_factory();
}

fn test_zero_budget_immediate_exhaustion() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(0));

    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    handle.join().unwrap();

    match rx.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { attempts: 1 }) => {}
        other => panic!("Expected Shutdown(attempts=1), got {other:?}"),
    }
    // create() never called (budget=0 means immediate exhaustion).
    assert_eq!(get_create_count(), 0);
    teardown_factory();
}

fn test_shutdown_event_payload() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(0));

    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    handle.join().unwrap();

    match rx.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted { attempts: 1 }) => {}
        other => panic!("Expected Shutdown(attempts=1), got {other:?}"),
    }
    teardown_factory();
}

fn test_create_failure_then_success() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let (notify_rx, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("transient error")), None),
            (ok2, Some(cmd_tx2.clone())),
        ],
        no_delay_policy(3),
    );

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    // Wait for second create() call (first fails, second succeeds).
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx2, b's');
    handle.join().unwrap();

    assert_eq!(get_create_count(), 2);
    teardown_factory();
}

fn test_persistent_create_failure() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("fail 1")), None),
            (Err(miette::miette!("fail 2")), None),
            (Err(miette::miette!("fail 3")), None),
        ],
        no_delay_policy(3),
    );

    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    handle.join().unwrap();

    let mut found_shutdown = false;
    while let Ok(event) = rx.try_recv() {
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
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b's');
    handle.join().unwrap();

    assert!(shared_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_guard_marks_terminated_on_stop() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b's');
    handle.join().unwrap();

    assert_eq!(liveness.is_running(), LivenessState::Terminated);
    teardown_factory();
}

fn test_guard_clears_waker_on_exhaustion() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(0));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'r');
    handle.join().unwrap();

    assert!(shared_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_backoff_delay_applied() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();

    let policy = RestartPolicy {
        max_restarts: 1,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: None,
        max_delay: None,
    };
    let (notify_rx, _senders) =
        setup_factory(vec![(ok2, Some(cmd_tx2.clone()))], policy);

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let start = Instant::now();
    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx2, b's');
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
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (ok2, cmd_tx2) = create_ok_result();
    let (ok3, cmd_tx3) = create_ok_result();

    let policy = RestartPolicy {
        max_restarts: 3,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: Some(2.0),
        max_delay: None,
    };
    let (notify_rx, _senders) = setup_factory(
        vec![
            (Err(miette::miette!("transient")), None),
            (ok2, Some(cmd_tx2.clone())),
            (ok3, Some(cmd_tx3.clone())),
        ],
        policy,
    );

    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker1)));

    let start = Instant::now();
    let handle =
        spawn_worker_loop(worker1, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx1, b'r');
    // Wait for create() calls: first Err, then Ok(W2).
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx2, b'r');
    // Wait for create() Ok(W3).
    notify_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    send_cmd(&cmd_tx3, b's');
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
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'p');
    handle.join().unwrap();

    match rx.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

fn test_panic_after_events() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'e');
    send_cmd(&cmd_tx, b'p');
    handle.join().unwrap();

    // Domain event first.
    match rx.try_recv().unwrap() {
        RRTEvent::Worker(TestEvent(_)) => {}
        other => panic!("Expected Worker event, got {other:?}"),
    }
    // Then shutdown.
    match rx.try_recv().unwrap() {
        RRTEvent::Shutdown(ShutdownReason::Panic) => {}
        other => panic!("Expected Shutdown(Panic), got {other:?}"),
    }
    teardown_factory();
}

fn test_guard_clears_waker_on_panic() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'p');
    handle.join().unwrap();

    assert!(shared_waker.lock().unwrap().is_none());
    teardown_factory();
}

fn test_guard_marks_terminated_on_panic() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'p');
    handle.join().unwrap();

    assert_eq!(liveness.is_running(), LivenessState::Terminated);
    teardown_factory();
}

fn test_no_restart_after_panic() {
    let (worker, waker, cmd_tx) = create_test_resources();
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let liveness = Arc::new(RRTLiveness::new());
    let shared_waker: Arc<Mutex<Option<TestWaker>>> =
        Arc::new(Mutex::new(Some(waker)));

    let (_notify_rx, _senders) =
        setup_factory(vec![], no_delay_policy(3));
    let handle =
        spawn_worker_loop(worker, tx, Arc::clone(&liveness), Arc::clone(&shared_waker));

    send_cmd(&cmd_tx, b'p');
    handle.join().unwrap();

    // Factory was never called for restart (panic exits immediately).
    assert_eq!(get_create_count(), 0);
    teardown_factory();
}

// This test runs in a PTY subprocess because `MioPollWorkerFactory::create()`
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

/// Controlled: creates a real `MioPollWorker` via factory, corrupts the epoll fd,
/// and verifies that `poll_once()` returns `Restart` with `StdinEvent::Error`.
fn poll_error_controlled() -> ! {
    use crate::tui::terminal_lib_backends::direct_to_ansi::input::{
        channel_types::{PollerEvent, StdinEvent},
        mio_poller::MioPollWorkerFactory,
    };
    use std::os::unix::io::{AsRawFd, FromRawFd};

    println!("{POLL_ERROR_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    // Create worker using the real factory (PTY provides real terminal stdin).
    let (mut worker, _waker) = MioPollWorkerFactory::create().unwrap();

    let (tx, mut rx) = tokio::sync::broadcast::channel::<RRTEvent<PollerEvent>>(16);

    // Corrupt the epoll fd so poll() fails with EBADF (non-EINTR).
    // Safety: We intentionally close the fd to trigger the error path.
    // The OwnedFd takes ownership and closes it on drop.
    let raw_fd = worker.poll_handle.as_raw_fd();
    drop(unsafe { std::os::unix::io::OwnedFd::from_raw_fd(raw_fd) });

    let result = worker.poll_once(&tx);

    assert_eq!(result, Continuation::Restart);

    match rx.try_recv().unwrap() {
        RRTEvent::Worker(PollerEvent::Stdin(StdinEvent::Error)) => {}
        other => panic!("Expected StdinEvent::Error, got {other:?}"),
    }

    println!("{POLL_ERROR_PASSED}");
    std::io::stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}

async fn test_subscribe_spawns_thread() {
    let (worker, waker, cmd_tx) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![(Ok((worker, waker)), Some(cmd_tx.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestFactory> = RRT::new();
    let _guard = rrt.subscribe().unwrap();

    // Wait for thread to start.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(rrt.is_thread_running(), LivenessState::Running);

    send_cmd(&cmd_tx, b's');
    // Wait for thread to exit.
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);
    teardown_factory();
}

async fn test_subscribe_fast_path_reuse() {
    let (worker, waker, cmd_tx) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![(Ok((worker, waker)), Some(cmd_tx.clone()))],
        no_delay_policy(3),
    );

    let rrt: RRT<TestFactory> = RRT::new();
    let _guard1 = rrt.subscribe().unwrap();

    std::thread::sleep(Duration::from_millis(50));
    let gen1 = rrt.get_thread_generation();

    // Second subscribe reuses the thread (fast path).
    let _guard2 = rrt.subscribe().unwrap();
    let gen2 = rrt.get_thread_generation();

    assert_eq!(gen1, gen2, "Expected same generation (thread reuse)");
    assert_eq!(rrt.get_receiver_count(), 2);

    send_cmd(&cmd_tx, b's');
    std::thread::sleep(Duration::from_millis(100));
    teardown_factory();
}

async fn test_subscribe_slow_path_after_termination() {
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (worker2, waker2, cmd_tx2) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![
            (Ok((worker1, waker1)), Some(cmd_tx1.clone())),
            (Ok((worker2, waker2)), Some(cmd_tx2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestFactory> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        send_cmd(&cmd_tx1, b's');
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);

        // Second subscribe after termination (slow path).
        let _guard2 = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation (thread relaunch)");

        send_cmd(&cmd_tx2, b's');
        std::thread::sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

async fn test_shutdown_received_by_subscriber() {
    let (worker, waker, cmd_tx) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![(Ok((worker, waker)), Some(cmd_tx.clone()))],
        no_delay_policy(0),
    );

    let rrt: RRT<TestFactory> = RRT::new();
    let guard = rrt.subscribe().unwrap();
    let mut rx = guard.receiver.as_ref().unwrap().resubscribe();

    std::thread::sleep(Duration::from_millis(50));

    // Worker returns Restart, budget=0 -> immediate exhaustion.
    send_cmd(&cmd_tx, b'r');

    // Wait for the shutdown event.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found_shutdown = false;
    while Instant::now() < deadline {
        match rx.try_recv() {
            Ok(RRTEvent::Shutdown(ShutdownReason::RestartPolicyExhausted {
                ..
            })) => {
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
    let (worker1, waker1, cmd_tx1) = create_test_resources();
    let (worker2, waker2, cmd_tx2) = create_test_resources();

    let (_notify_rx, _senders) = setup_factory(
        vec![
            (Ok((worker1, waker1)), Some(cmd_tx1.clone())),
            (Ok((worker2, waker2)), Some(cmd_tx2.clone())),
        ],
        no_delay_policy(3),
    );

    let rrt: RRT<TestFactory> = RRT::new();

    // First subscribe.
    {
        let _guard = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen1 = rrt.get_thread_generation();

        // Cause a panic.
        send_cmd(&cmd_tx1, b'p');
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(rrt.is_thread_running(), LivenessState::Terminated);

        // Subscribe again after panic (should relaunch).
        let _guard2 = rrt.subscribe().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let gen2 = rrt.get_thread_generation();

        assert_ne!(gen1, gen2, "Expected new generation after panic recovery");
        assert_eq!(rrt.is_thread_running(), LivenessState::Running);

        send_cmd(&cmd_tx2, b's');
        std::thread::sleep(Duration::from_millis(100));
    }
    teardown_factory();
}

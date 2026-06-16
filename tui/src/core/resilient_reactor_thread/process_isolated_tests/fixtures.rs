// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{BroadcastSender, Continuation,
            DeadlockPreventionPolicy::PanicOnAnyLockNesting, InterruptHandle, RRTEvent,
            RRTSoftwareInterrupt, RRTWorker, RestartPolicy, ScopedMutex,
            ThreadLifecycleMonitor, ThreadState,
            core::resilient_reactor_thread::SubscriberGuard, run_worker_loop,
            scoped_mutex};
use std::{collections::VecDeque,
          sync::{Arc, LazyLock,
                 atomic::{AtomicU32, Ordering},
                 mpsc}};

/// Simple domain event for tests.
#[derive(Clone, Debug, PartialEq)]
pub struct TestEvent(pub u32);

/// Monotonic counter for unique interrupt handle IDs.
pub static NEXT_INTERRUPT_ID: AtomicU32 = AtomicU32::new(0);

/// Tracks the most recently invoked interrupt handle's ID.
pub static LAST_INTERRUPT_ID: AtomicU32 = AtomicU32::new(0);

/// Test interrupt handle that records its ID in [`LAST_INTERRUPT_ID`] when
/// [`trigger_software_interrupt()`] is called.
///
/// [`trigger_software_interrupt()`]: RRTSoftwareInterrupt::trigger_software_interrupt
#[derive(Debug)]
pub struct TestInterrupt {
    pub id: u32,
}

impl RRTSoftwareInterrupt for TestInterrupt {
    fn trigger_software_interrupt(&self) {
        LAST_INTERRUPT_ID.store(self.id, Ordering::SeqCst);
    }
}

/// Test worker driven by a broadcast command channel.
#[derive(Debug)]
pub struct TestWorker {
    pub input_receiver: Option<tokio::sync::broadcast::Receiver<u8>>,
    pub event_counter: u32,
}

impl RRTWorker for TestWorker {
    type Config = ();
    type Input = u8;
    type Output = TestEvent;
    type Interrupt = TestInterrupt;

    fn create_and_register_os_sources(
        _config: Self::Config,
        receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ) -> miette::Result<(Self, Self::Interrupt)> {
        TEST_FACTORY_STATE.write(|guard: &mut Option<TestFactoryState>| {
            let state = guard.as_mut().expect("TEST_FACTORY_STATE not initialized");
            state.create_count += 1;
            if let Some(ref notify_sender) = state.create_notify {
                let _ = notify_sender.send(());
            }
            let res = state
                .create_results
                .pop_front()
                .unwrap_or_else(|| Err(miette::miette!("TestWorker: no create results")));

            if let Ok(mut worker_interrupt) = res {
                worker_interrupt.0.input_receiver = Some(receiver);
                Ok(worker_interrupt)
            } else {
                res
            }
        })
    }

    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &tokio::sync::broadcast::Sender<RRTEvent<Self::Output>>,
    ) -> Continuation {
        let Some(rx) = self.input_receiver.as_mut() else {
            return Continuation::Stop;
        };
        #[allow(clippy::match_same_arms)]
        match rx.blocking_recv() {
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
        TEST_FACTORY_STATE.read(|guard: &Option<TestFactoryState>| {
            guard
                .as_ref()
                .expect("TEST_FACTORY_STATE not initialized")
                .restart_policy
                .clone()
        })
    }
}

/// Shared state controlling [`TestWorker::create_and_register_os_sources()`] behavior.
#[derive(Debug)]
pub struct TestFactoryState {
    pub create_results: VecDeque<miette::Result<(TestWorker, TestInterrupt)>>,
    pub create_count: u32,
    pub restart_policy: RestartPolicy,
    pub create_notify: Option<mpsc::Sender<()>>,
}

pub static TEST_FACTORY_STATE: LazyLock<
    ScopedMutex<Option<TestFactoryState>, { PanicOnAnyLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(ANY, None));

pub fn create_test_resources() -> (
    TestWorker,
    TestInterrupt,
    tokio::sync::broadcast::Sender<u8>,
) {
    let (cmd_sender, cmd_receiver) = tokio::sync::broadcast::channel(10);
    let worker = TestWorker {
        input_receiver: Some(cmd_receiver),
        event_counter: 0,
    };
    let interrupt_id = NEXT_INTERRUPT_ID.fetch_add(1, Ordering::Relaxed);
    let interrupt = TestInterrupt { id: interrupt_id };
    (worker, interrupt, cmd_sender)
}

/// Create a successful test worker and interrupt wrapped in a [`miette::Result`].
///
/// # Errors
///
/// This function never actually returns an error, but returns [`Result`] to match the
/// expected type signature in [`TestFactoryState`].
pub fn create_ok_result() -> miette::Result<(TestWorker, TestInterrupt)> {
    let (worker, interrupt, _cmd_sender) = create_test_resources();
    Ok((worker, interrupt))
}

#[must_use]
pub fn create_shared_state(
    interrupt: TestInterrupt,
    cmd_sender: tokio::sync::broadcast::Sender<u8>,
) -> Arc<ThreadLifecycleMonitor<TestWorker>> {
    Arc::new(ThreadLifecycleMonitor::<TestWorker>::new(
        ThreadState::Running(InterruptHandle::new(interrupt), cmd_sender),
    ))
}

/// Initialize [`TEST_FACTORY_STATE`] with pre-programmed create results and a restart
/// policy.
///
/// # Panics
///
/// Panics if the [`TEST_FACTORY_STATE`] mutex is poisoned.
#[allow(clippy::type_complexity)]
pub fn setup_factory(
    create_results_vec: Vec<miette::Result<(TestWorker, TestInterrupt)>>,
    policy: RestartPolicy,
) -> mpsc::Receiver<()> {
    let (notify_sender, notify_receiver) = mpsc::channel();
    let mut create_results = VecDeque::new();

    for result in create_results_vec {
        create_results.push_back(result);
    }

    TEST_FACTORY_STATE.write(|guard: &mut Option<TestFactoryState>| {
        *guard = Some(TestFactoryState {
            create_results,
            create_count: 0,
            restart_policy: policy,
            create_notify: Some(notify_sender),
        });
    });

    notify_receiver
}

/// Reset [`TEST_FACTORY_STATE`] to `None`.
///
/// # Panics
///
/// Panics if the [`TEST_FACTORY_STATE`] mutex is poisoned.
pub fn teardown_factory() {
    TEST_FACTORY_STATE.write(|guard: &mut Option<TestFactoryState>| {
        *guard = None;
    });
}

/// Return how many times [`TestWorker::create_and_register_os_sources()`] has been
/// called.
///
/// # Panics
///
/// Panics if the [`TEST_FACTORY_STATE`] mutex is poisoned.
pub fn get_create_count() -> u32 {
    #[allow(clippy::map_unwrap_or)]
    TEST_FACTORY_STATE.read(|guard: &Option<TestFactoryState>| {
        guard.as_ref().map(|s| s.create_count).unwrap_or(0)
    })
}

/// Send a command byte to a [`TestWorker`]'s command channel.
///
/// # Panics
///
/// Panics if the receiver has been dropped.
pub fn send_cmd(cmd_sender: &tokio::sync::broadcast::Sender<u8>, cmd: u8) {
    let _ = cmd_sender.send(cmd);
}

/// Send a command byte via a [`SubscriberGuard`].
///
/// # Panics
///
/// Panics if the [`tokio`] runtime fails to build, or if the underlying send operation
/// fails.
///
/// [`tokio`]: tokio
pub fn send_cmd_via_guard(guard: &SubscriberGuard<TestWorker>, cmd: u8) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        guard.get_input_sender().send(cmd).await.unwrap();
    });
}

#[must_use]
pub fn no_delay_policy(max_restarts: u8) -> RestartPolicy {
    RestartPolicy {
        max_restarts,
        initial_delay: None,
        backoff_multiplier: None,
        max_delay: None,
    }
}

/// This is used for OS debugging when using [`btop`] or [`top`], etc.
///
/// [`btop`]: https://github.com/aristocratos/btop
/// [`top`]: https://linux.die.net/man/1/top
pub const THREAD_NAME: &str = "test-rrt-worker";

/// Spawn [`run_worker_loop`] on a dedicated thread named [`THREAD_NAME`].
///
/// # Panics
///
/// Panics if the thread cannot be spawned.
pub fn spawn_worker_loop(
    worker: TestWorker,
    sender: BroadcastSender<TestEvent>,
    shared_state: Arc<ThreadLifecycleMonitor<TestWorker>>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name(THREAD_NAME.into())
        .spawn(move || {
            run_worker_loop::<TestWorker>(worker, (), sender, shared_state);
        })
        .unwrap()
}

/// Assert that a process-isolated test child exited successfully.
///
/// # Panics
///
/// Panics if the child exited with a non-zero status or its [`stderr`] contains
/// unexpected errors (anything other than deliberate panics).
///
/// [`stderr`]: std::io::stderr
pub fn controller_fn(output: std::process::Output) {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let has_unexpected_error = stderr.contains("Test failed with error")
        || (!stderr.contains("deliberate panic") && stderr.contains("panicked at"));

    if !output.status.success() || has_unexpected_error {
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

pub fn create_mock_guard(
    sender: BroadcastSender<TestEvent>,
    shared_state: Arc<ThreadLifecycleMonitor<TestWorker>>,
) -> SubscriberGuard<TestWorker> {
    SubscriberGuard::new(sender.clone(), sender.subscribe(), shared_state)
}

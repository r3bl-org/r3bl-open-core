// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{BroadcastSender, Continuation,
            DeadlockPreventionPolicy::PanicOnAnyLockNesting, InterruptHandle, RRTEvent,
            RRTSoftwareInterrupt, RRTWorker, RestartPolicy, ScopedMutex,
            ThreadLifecycleMonitor, ThreadState, run_worker_loop, scoped_mutex};
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

/// Test worker driven by an [`mpsc`] command channel.
#[derive(Debug)]
pub struct TestWorker {
    pub cmd_receiver: mpsc::Receiver<u8>,
    pub event_counter: u32,
}

impl RRTWorker for TestWorker {
    type Event = TestEvent;
    type Interrupt = TestInterrupt;

    fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)> {
        TEST_FACTORY_STATE.write(|guard: &mut Option<TestFactoryState>| {
            let state = guard.as_mut().expect("TEST_FACTORY_STATE not initialized");
            state.create_count += 1;
            if let Some(ref notify_sender) = state.create_notify {
                let _ = notify_sender.send(());
            }
            state
                .create_results
                .pop_front()
                .unwrap_or_else(|| Err(miette::miette!("TestWorker: no create results")))
        })
    }

    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &tokio::sync::broadcast::Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        #[allow(clippy::match_same_arms)]
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

pub fn create_test_resources() -> (TestWorker, TestInterrupt, mpsc::Sender<u8>) {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let worker = TestWorker {
        cmd_receiver,
        event_counter: 0,
    };
    let interrupt_id = NEXT_INTERRUPT_ID.fetch_add(1, Ordering::Relaxed);
    let interrupt = TestInterrupt { id: interrupt_id };
    (worker, interrupt, cmd_sender)
}

pub fn create_ok_result() -> (
    miette::Result<(TestWorker, TestInterrupt)>,
    mpsc::Sender<u8>,
) {
    let (worker, interrupt, cmd_sender) = create_test_resources();
    (Ok((worker, interrupt)), cmd_sender)
}

#[must_use]
pub fn create_shared_state(
    interrupt: TestInterrupt,
) -> Arc<ThreadLifecycleMonitor<TestWorker>> {
    Arc::new(ThreadLifecycleMonitor::<TestWorker>::new(
        ThreadState::Running(InterruptHandle::new(interrupt)),
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
    results_and_senders: Vec<(
        miette::Result<(TestWorker, TestInterrupt)>,
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

    TEST_FACTORY_STATE.write(|guard: &mut Option<TestFactoryState>| {
        *guard = Some(TestFactoryState {
            create_results,
            create_count: 0,
            restart_policy: policy,
            create_notify: Some(notify_sender),
        });
    });

    (notify_receiver, cmd_senders)
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
pub fn send_cmd(cmd_sender: &mpsc::Sender<u8>, cmd: u8) { cmd_sender.send(cmd).unwrap(); }

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
            run_worker_loop::<TestWorker>(worker, sender, shared_state);
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Continuation, RRTEvent, RRTSoftwareInterrupt, RRTWorker, ThreadLifecycleMonitor,
            ThreadState, InterruptHandle};
use std::sync::{Arc,
                atomic::{AtomicBool, Ordering}};

#[derive(Debug)]
struct TestInterrupt {
    interrupted: Arc<AtomicBool>,
}
impl RRTSoftwareInterrupt for TestInterrupt {
    fn trigger_software_interrupt(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct TestWorker;
impl RRTWorker for TestWorker {
    type Interrupt = TestInterrupt;
    fn create_and_register_os_sources(
        _config: Self::Config,
        _receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ) -> miette::Result<(Self, Self::Interrupt)> {
        unimplemented!()
    }
    fn block_until_ready_then_dispatch(
        &mut self,
        _sender: &tokio::sync::broadcast::Sender<RRTEvent<Self::Output>>,
    ) -> Continuation {
        unimplemented!()
    }
}

use super::super::ThreadStateStatus;

#[test]
fn test_state_status() {
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupt = TestInterrupt { interrupted };

    let (tx, _rx) = tokio::sync::broadcast::channel(16);

    let stable_states = vec![
        ThreadState::<TestWorker>::Stopped,
        ThreadState::<TestWorker>::Running(InterruptHandle::new(interrupt), tx),
    ];
    for state in stable_states {
        assert_eq!(state.status(), ThreadStateStatus::Stable);
    }

    let transient_states = vec![
        ThreadState::<TestWorker>::Starting,
        ThreadState::<TestWorker>::Stopping(crate::StopReason::ZeroReceivers),
        ThreadState::<TestWorker>::Restarting,
    ];
    for state in transient_states {
        assert_eq!(state.status(), ThreadStateStatus::Transient);
    }
}

#[test]
fn test_interrupt_if_running() {
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupt = TestInterrupt {
        interrupted: interrupted.clone(),
    };

    let monitor = ThreadLifecycleMonitor::<TestWorker>::new(ThreadState::Running(
        InterruptHandle::new(interrupt),
        tokio::sync::broadcast::channel(1).0,
    ));

    // 1. Running -> Should interrupt.
    monitor.interrupt_if_running();
    assert!(interrupted.load(Ordering::SeqCst));

    // 2. Other states -> Should NOT interrupt.
    interrupted.store(false, Ordering::SeqCst);

    let non_running_states = vec![
        ThreadState::Stopped,
        ThreadState::Starting,
        ThreadState::Stopping(crate::StopReason::WorkerRequested),
        ThreadState::Restarting,
    ];

    for state in non_running_states {
        {
            let mut guard = monitor.lock();
            *guard = state;
        }
        monitor.interrupt_if_running();
        assert!(
            !interrupted.load(Ordering::SeqCst),
            "Should not interrupt in state {:?}",
            monitor.lock()
        );
    }
}

#[test]
fn test_termination_guard_drop() {
    let monitor = Arc::new(ThreadLifecycleMonitor::<TestWorker>::new(
        ThreadState::Starting,
    ));

    // Case 1: Drop from Starting
    {
        let _guard: crate::resilient_reactor_thread::TerminationGuard<TestWorker> =
            monitor.clone().into();
    }
    assert!(matches!(*monitor.lock(), ThreadState::Stopped));

    // Case 2: Drop from Restarting
    {
        {
            let mut state = monitor.lock();
            *state = ThreadState::Restarting;
        }
        let _guard: crate::resilient_reactor_thread::TerminationGuard<TestWorker> =
            monitor.clone().into();
    }
    assert!(matches!(*monitor.lock(), ThreadState::Stopped));

    // Case 3: Drop from Stopping
    {
        {
            let mut state = monitor.lock();
            *state = ThreadState::Stopping(crate::StopReason::ZeroReceivers);
        }
        let _guard: crate::resilient_reactor_thread::TerminationGuard<TestWorker> =
            monitor.clone().into();
    }
    assert!(matches!(*monitor.lock(), ThreadState::Stopped));
}

#[test]
fn test_block_until_stable_state_reached() {
    let monitor = Arc::new(ThreadLifecycleMonitor::<TestWorker>::new(
        ThreadState::Starting,
    ));

    let monitor_clone = monitor.clone();
    let handle = std::thread::spawn(move || {
        // Blocks because state is Starting.
        let guard = monitor_clone.block_until_stable_state_reached();
        matches!(*guard, ThreadState::Running(_, _))
    });

    // Wait a bit to ensure it's blocked.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Transition to Running.
    {
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupt = TestInterrupt { interrupted };
        let state = monitor.lock();
        let state = monitor.set_state(
            state,
            ThreadState::Running(
                InterruptHandle::new(interrupt),
                tokio::sync::broadcast::channel(1).0,
            )
        );
        drop(state);
    }

    let is_running = handle.join().unwrap();
    assert!(is_running);
}

#[test]
fn test_block_until_stable_state_reached_failure_recovery() {
    let monitor = Arc::new(ThreadLifecycleMonitor::<TestWorker>::new(
        ThreadState::Starting,
    ));

    let monitor_clone = monitor.clone();
    let handle = std::thread::spawn(move || {
        // Blocks because state is Starting.
        let guard = monitor_clone.block_until_stable_state_reached();
        matches!(*guard, ThreadState::Stopped)
    });

    // Wait a bit to ensure it's blocked.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Transition to Stopped (representing a failure to start).
    {
        let state = monitor.lock();
        let state = monitor.set_state(state, ThreadState::Stopped);
        drop(state);
    }

    let is_stopped = handle.join().unwrap();
    assert!(is_stopped);
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::Continuation;
use crate::core::resilient_reactor_thread::{
    InputSender, InterruptHandle, RRTEvent, RRTSoftwareInterrupt, RRTWorker,
    ThreadLifecycleMonitor, ThreadState,
};
use std::sync::Arc;
use tokio::time::Duration;

#[derive(Debug)]
struct DummyInterrupt;

impl RRTSoftwareInterrupt for DummyInterrupt {
    fn trigger_software_interrupt(&self) {}
}

#[derive(Debug)]
struct DummyWorker;

impl RRTWorker for DummyWorker {
    type Config = ();
    type Input = String;
    type Output = ();
    type Interrupt = DummyInterrupt;

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

#[tokio::test]
async fn test_input_sender_waits_for_restart() {
    let monitor = Arc::new(ThreadLifecycleMonitor::<DummyWorker>::new(
        ThreadState::Restarting,
    ));

    let _input_sender = InputSender {
        shared_state: monitor.clone(),
    };

    let msg = "hello smart sender".to_string();

    // Spawn a Tokio task that calls input_sender.send. It should block because the
    // state is currently `Restarting`.
    let sender_task = tokio::spawn({
        let input_sender = InputSender {
            shared_state: monitor.clone(),
        };
        let msg = msg.clone();
        async move { input_sender.send(msg).await }
    });

    // Yield back to the Tokio runtime to allow the spawned task to reach the await point
    // where it waits on `input_sender_notify`.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // From the main thread, transition the state to `Running` with a fresh channel.
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    {
        let state_guard = monitor.lock();
        // set_state automatically calls input_sender_notify.notify_waiters()
        let _unused = monitor.set_state(
            state_guard,
            ThreadState::Running(InterruptHandle::new(DummyInterrupt), tx),
        );
    }

    // Await the sender task to ensure it successfully bypassed the transient state
    // and successfully delivered the message.
    let result = sender_task.await.unwrap();
    assert!(result.is_ok(), "Sender task should succeed");

    // Verify the message was successfully delivered to the new channel receiver.
    let received = rx.recv().await.unwrap();
    assert_eq!(received, msg);
}

#[tokio::test]
async fn test_input_sender_fails_if_stopped() {
    let monitor = Arc::new(ThreadLifecycleMonitor::<DummyWorker>::new(
        ThreadState::Stopped,
    ));

    let input_sender = InputSender {
        shared_state: monitor.clone(),
    };

    let msg = "should fail".to_string();

    // Send should immediately fail because state is permanently Stopped.
    let result = input_sender.send(msg).await;
    assert!(result.is_err(), "Sender task should fail when Stopped");
}

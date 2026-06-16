// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::Continuation;
use crate::core::resilient_reactor_thread::{
    RRTEvent, RRTSoftwareInterrupt, RRTWorker, RestartPolicy, RRT,
};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct SmartInterrupt;

impl RRTSoftwareInterrupt for SmartInterrupt {
    fn trigger_software_interrupt(&self) {}
}

#[derive(Debug)]
pub struct SmartWorker {
    pub config_val: String,
    pub receiver: tokio::sync::broadcast::Receiver<String>,
}

impl RRTWorker for SmartWorker {
    type Config = String;
    type Input = String;
    type Output = String;
    type Interrupt = SmartInterrupt;

    fn create_and_register_os_sources(
        config: Self::Config,
        receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ) -> miette::Result<(Self, Self::Interrupt)> {
        // Assert that the config is what we expect!
        assert_eq!(config, "test_config_value");
        CREATE_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);

        Ok((
            SmartWorker {
                config_val: config,
                receiver,
            },
            SmartInterrupt,
        ))
    }

    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &tokio::sync::broadcast::Sender<RRTEvent<Self::Output>>,
    ) -> Continuation {
        // Blockingly receive the next message
        match self.receiver.blocking_recv() {
            Ok(msg) => {
                if msg == "crash" {
                    return Continuation::Restart;
                }
                if msg == "stop" {
                    return Continuation::Stop;
                }
                let _unused = sender.send(RRTEvent::Worker(msg));
                Continuation::Continue
            }
            Err(_) => {
                // Disconnected
                Continuation::Stop
            }
        }
    }

    fn restart_policy() -> RestartPolicy {
        RestartPolicy {
            max_restarts: 3,
            initial_delay: Some(std::time::Duration::from_millis(200)),
            backoff_multiplier: None,
            max_delay: None,
        }
    }
}

static CREATE_CALL_COUNTER: AtomicU32 = AtomicU32::new(0);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_smart_sender_retry_and_config() {
    CREATE_CALL_COUNTER.store(0, Ordering::SeqCst);

    let rrt = RRT::<SmartWorker>::new();
    let mut rx = rrt.sender.subscribe();

    // 1. Subscribe and spawn the worker
    let _guard = rrt.try_subscribe("test_config_value".to_string()).unwrap();

    let input_sender = rrt.get_input_sender();

    // 2. Verify worker creation
    assert_eq!(CREATE_CALL_COUNTER.load(Ordering::SeqCst), 1);

    // 3. Send normal message
    input_sender.send("hello".to_string()).await.unwrap();

    // Wait for the output to confirm worker is healthy
    if let Ok(RRTEvent::Worker(msg)) = rx.recv().await {
        assert_eq!(msg, "hello");
    } else {
        panic!("Did not receive 'hello'");
    }

    // 4. Send "crash" command
    input_sender.send("crash".to_string()).await.unwrap();

    // Sleep for 50ms to ensure the worker thread has processed "crash" and entered the
    // 200ms `Restarting` delay period before we try to send the next message.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 5. Send message DURING crash!
    // This is the core feature of the smart sender: we await the send. It will
    // seamlessly suspend and wait for the new worker to spawn, then deliver it to the
    // new tokio channel!
    let send_result = input_sender.send("hello after crash".to_string()).await;
    assert!(send_result.is_ok(), "Smart sender should successfully retry");

    // 6. Verify worker was re-created
    assert_eq!(CREATE_CALL_COUNTER.load(Ordering::SeqCst), 2);

    // 7. Verify the worker received the message AFTER the restart
    if let Ok(RRTEvent::Worker(msg)) = rx.recv().await {
        assert_eq!(msg, "hello after crash");
    } else {
        panic!("Did not receive 'hello after crash'");
    }

    // 8. Clean up
    input_sender.send("stop".to_string()).await.unwrap();
}

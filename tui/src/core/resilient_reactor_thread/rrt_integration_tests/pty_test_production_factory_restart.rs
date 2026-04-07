// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration test for production
//! [`MioPollWorker::create_and_register_os_sources()`] restart cycles.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_production_factory_restart_cycle -- --nocapture
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{Continuation, MioSoftwareInterrupt, PtyTestContext, PtyTestMode, RRT, RRTEvent,
            RRTWorker, RestartPolicy, ThreadState, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::input::{channel_types::PollerEvent,
                                                                mio_poller::MioPollWorker}};
use std::{io::{Write, stdout},
          sync::atomic::{AtomicU32, Ordering},
          thread::sleep,
          time::{Duration, Instant}};
use tokio::sync::broadcast;

generate_pty_test! {
    test_fn: test_production_factory_restart_cycle,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    eprintln!("Factory-Restart Controller: Starting...");

    child
        .wait_for_ready(&mut buf_reader, FACTORY_RESTART_READY)
        .expect("Failed to wait for FACTORY_RESTART_READY");

    for i in 1..=3 {
        child
            .wait_for_ready(&mut buf_reader, SEND_KEY)
            .expect("Failed to wait for SEND_KEY");
        eprintln!("  -> Sending keystroke #{i}");
        writer.write_all(b"x").expect("Failed to write keystroke");
        writer.flush().expect("Failed to flush keystroke");
    }

    child
        .wait_for_ready(&mut buf_reader, FACTORY_RESTART_PASSED)
        .expect("Failed to wait for FACTORY_RESTART_PASSED");

    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("Factory-Restart Controller: Test passed!");
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    create_call_counter::reset();

    println!("{FACTORY_RESTART_READY}");
    stdout().flush().expect("Failed to flush");

    let rrt = RRT::<RestartTestWorker>::new();
    let _guard = rrt.try_subscribe().unwrap();

    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    create_call_counter::spin_wait_until(2);
    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    create_call_counter::spin_wait_until(3);
    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(*rrt.shared_state.lock(), ThreadState::Stopped) {
        assert!(
            std::time::Instant::now() < deadline,
            "Timeout waiting for worker thread to terminate",
        );
        sleep(Duration::from_millis(1));
    }

    let count = create_call_counter::get();
    eprintln!("Factory-Restart Controlled: create() called {count} times");

    assert_eq!(
        count, 3,
        "Expected 3 create() calls (initial + 2 restarts), got {count}"
    );

    println!("{FACTORY_RESTART_PASSED}");
    stdout().flush().expect("Failed to flush");
}

#[derive(Debug)]
struct RestartTestWorker {
    inner: MioPollWorker,
    poll_count: u32,
}

impl RRTWorker for RestartTestWorker {
    type Event = PollerEvent;
    type Interrupt = MioSoftwareInterrupt;

    fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)> {
        create_call_counter::increment();
        let (inner_worker, interrupt) = MioPollWorker::create_and_register_os_sources()?;
        Ok((
            RestartTestWorker {
                inner: inner_worker,
                poll_count: 0,
            },
            interrupt,
        ))
    }

    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &broadcast::Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        self.poll_count += 1;
        if self.poll_count == 1 {
            // First call: delegate to real worker. Blocks on poll.poll()
            // until the controller sends a keystroke via the PTY.
            self.inner.block_until_ready_then_dispatch(sender)
        } else {
            // Second call: restart or stop based on total create count.
            let count = create_call_counter::get();
            if count < 3 {
                Continuation::Restart
            } else {
                Continuation::Stop
            }
        }
    }

    fn restart_policy() -> RestartPolicy {
        RestartPolicy {
            max_restarts: 3,
            initial_delay: None,
            backoff_multiplier: None,
            max_delay: None,
        }
    }
}

mod create_call_counter {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    static COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn reset() { COUNT.store(0, Ordering::SeqCst); }

    pub fn increment() { COUNT.fetch_add(1, Ordering::SeqCst); }

    pub fn get() -> u32 { COUNT.load(Ordering::SeqCst) }

    pub fn spin_wait_until(target: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while get() < target {
            assert!(
                Instant::now() < deadline,
                "Timeout waiting for create() #{target}, current={}",
                get(),
            );
            sleep(Duration::from_millis(1));
        }
    }
}

const FACTORY_RESTART_READY: &str = "FACTORY_RESTART_READY";
const FACTORY_RESTART_PASSED: &str = "FACTORY_RESTART_PASSED";
const SEND_KEY: &str = "SEND_KEY";

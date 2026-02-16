// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY integration test for production `MioPollWorker::create()` restart cycles.
//!
//! Each worker processes a **real keystroke** from the controller via
//! `MioPollWorker::poll_once()` before restarting. This proves:
//!
//! - `MioPollWorker::create()` works correctly 3 times in sequence
//! - Each restarted worker can actually poll stdin and process events
//! - No fd leaks or stale epoll state between create/drop cycles
//! - Production [`MioPollWaker`] correctly couples to new Poll registry each time
//!
//! The PTY provides real terminal stdin (fd 0 on the controlled end), which is required
//! for `epoll_ctl` registration.
//!
//! See also: Group B Step 5.7 in [`rrt_restart_tests`] for the production poll-error path
//! test.
//!
//! [`rrt_restart_tests`]: super::rrt_restart_tests

use super::super::*;
use crate::{Continuation, ControlledChild, PtyPair, PtyTestMode, generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::input::{channel_types::PollerEvent,
                                                                mio_poller::MioPollWorker}};
use std::{io::{BufRead, BufReader, Read, Write, stdout},
          sync::atomic::{AtomicU32, Ordering},
          thread::sleep,
          time::{Duration, Instant}};

/// Worker that delegates the first `poll_once()` call to the real
/// [`MioPollWorker`], then returns [`Continuation::Restart`] or
/// [`Continuation::Stop`] on the second call.
///
/// This proves each restarted worker can actually process stdin events
/// via the production poll loop, not just that `create()` returns `Ok`.
struct RestartTestWorker {
    inner: MioPollWorker,
    poll_count: u32,
}

impl RRTWorker for RestartTestWorker {
    type Event = PollerEvent;

    fn create() -> miette::Result<(Self, impl RRTWaker)> {
        create_count::increment();
        let (inner_worker, wake_fn) = MioPollWorker::create()?;
        Ok((
            RestartTestWorker {
                inner: inner_worker,
                poll_count: 0,
            },
            wake_fn,
        ))
    }

    fn poll_once(
        &mut self,
        tx: &tokio::sync::broadcast::Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        self.poll_count += 1;
        if self.poll_count == 1 {
            // First call: delegate to real worker. Blocks on poll.poll()
            // until the controller sends a keystroke via the PTY.
            self.inner.poll_once(tx)
        } else {
            // Second call: restart or stop based on total create count.
            let count = create_count::get();
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

// XMARK: Process isolated test with PTY.

const FACTORY_RESTART_READY: &str = "FACTORY_RESTART_READY";
const FACTORY_RESTART_PASSED: &str = "FACTORY_RESTART_PASSED";
const SEND_KEY: &str = "SEND_KEY";

generate_pty_test! {
    test_fn: test_production_factory_restart_cycle,
    controller: factory_restart_controller,
    controlled: factory_restart_controlled,
    mode: PtyTestMode::Raw,
}

/// Waits for a line containing `signal` from the controlled process.
fn wait_for_signal(buf_reader: &mut BufReader<impl Read>, signal: &str) {
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

/// Controller: sends keystrokes when the controlled signals readiness, then
/// waits for all 3 create/poll cycles to complete.
fn factory_restart_controller(pty_pair: PtyPair, mut child: ControlledChild) {
    eprintln!("Factory-Restart Controller: Starting...");

    let reader = pty_pair
        .controller()
        .try_clone_reader()
        .expect("Failed to get reader");
    let mut buf_reader = BufReader::new(reader);
    let mut writer = pty_pair
        .controller()
        .take_writer()
        .expect("Failed to get writer");

    wait_for_signal(&mut buf_reader, FACTORY_RESTART_READY);

    // Each worker needs one keystroke to unblock poll.poll().
    // The controlled prints SEND_KEY when a worker is ready.
    for i in 1..=3 {
        wait_for_signal(&mut buf_reader, SEND_KEY);
        eprintln!("  -> Sending keystroke #{i}");
        writer.write_all(b"x").expect("Failed to write keystroke");
        writer.flush().expect("Failed to flush keystroke");
    }

    wait_for_signal(&mut buf_reader, FACTORY_RESTART_PASSED);

    crate::drain_pty_and_wait(buf_reader, pty_pair, &mut child);
    eprintln!("Factory-Restart Controller: Test passed!");
}

/// Controlled: runs `MioPollWorker::create()` 3 times via the RRT
/// restart loop. Each worker processes one real keystroke from the controller
/// before restarting, proving the restarted worker's epoll and stdin
/// registration actually function.
fn factory_restart_controlled() -> ! {
    create_count::reset();

    println!("{FACTORY_RESTART_READY}");
    stdout().flush().expect("Failed to flush");

    // RRT::subscribe() calls RestartTestWorker::create() and spawns the
    // worker thread. The _guard keeps the broadcast receiver alive.
    let rrt = RRT::<RestartTestWorker>::new();
    let _guard = rrt.subscribe().unwrap();

    // Worker 1 is now entering poll.poll(). Signal the controller to
    // send a keystroke. The PTY round-trip latency (~ms) is orders of
    // magnitude slower than thread startup -> poll.poll() entry (~us),
    // so the worker is guaranteed to be blocking by the time the
    // keystroke arrives.
    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    // Wait for Worker 2 to be created (restart happened), then send keystroke.
    create_count::spin_wait_until(2);
    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    // Wait for Worker 3 to be created (restart happened), then send keystroke.
    create_count::spin_wait_until(3);
    println!("{SEND_KEY}");
    stdout().flush().expect("Failed to flush");

    // Worker 3 processes the keystroke, then stops. Wait for thread exit.
    let deadline = Instant::now() + Duration::from_secs(5);
    while rrt.is_thread_running() != LivenessState::Terminated {
        assert!(
            std::time::Instant::now() < deadline,
            "Timeout waiting for worker thread to terminate",
        );
        sleep(Duration::from_millis(1));
    }

    let count = create_count::get();
    eprintln!("Factory-Restart Controlled: create() called {count} times");

    // Initial create() + 2 restarts = 3 total.
    assert_eq!(
        count, 3,
        "Expected 3 create() calls (initial + 2 restarts), got {count}"
    );

    println!("{FACTORY_RESTART_PASSED}");
    stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}

/// Encapsulates the atomic counter tracking how many times
/// [`RestartTestWorker::create()`] has been called.
mod create_count {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    static COUNT: AtomicU32 = AtomicU32::new(0);

    pub fn reset() { COUNT.store(0, Ordering::SeqCst); }

    pub fn increment() { COUNT.fetch_add(1, Ordering::SeqCst); }

    pub fn get() -> u32 { COUNT.load(Ordering::SeqCst) }

    /// Spin-waits until the counter reaches `target`, with a 5-second timeout.
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

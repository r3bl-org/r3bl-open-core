// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`] integration test for production [`MioPollWorker`] error handling.
//!
//! This test verifies that if the [`epoll`] file descriptor is corrupted, the worker
//! returns [`Continuation::Restart`] with a [`StdinEvent::Error`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_production_poll_error_sends_error_and_restarts -- --nocapture
//! ```
//!
//! [`Continuation::Restart`]: crate::Continuation::Restart
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`StdinEvent::Error`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::channel_types::StdinEvent::Error

use crate::{Continuation, PtyTestContext, PtyTestMode, RRTEvent, RRTWorker,
            generate_pty_test,
            tui::terminal_lib_backends::direct_to_ansi::input::{channel_types::{PollerEvent,
                                                                                StdinEvent},
                                                                mio_poller::MioPollWorker}};
use std::{io::{BufRead, BufReader, Write},
          os::unix::io::{AsRawFd, FromRawFd}};

const POLL_ERROR_READY: &str = "POLL_ERROR_READY";
const POLL_ERROR_PASSED: &str = "POLL_ERROR_PASSED";

generate_pty_test! {
    test_fn: test_production_poll_error_sends_error_and_restarts,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

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

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("Poll-Error Controller: Starting...");

    wait_for_signal(&mut buf_reader, POLL_ERROR_READY);
    wait_for_signal(&mut buf_reader, POLL_ERROR_PASSED);

    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("Poll-Error Controller: Test passed!");
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{POLL_ERROR_READY}");
    std::io::stdout().flush().expect("Failed to flush");

    let (mut worker, _waker) = MioPollWorker::create_and_register_os_sources().unwrap();

    let (sender, mut receiver) =
        tokio::sync::broadcast::channel::<RRTEvent<PollerEvent>>(16);

    let raw_fd = worker.poll_handle.as_raw_fd();
    drop(unsafe { std::os::unix::io::OwnedFd::from_raw_fd(raw_fd) });

    let result = worker.block_until_ready_then_dispatch(&sender);

    assert_eq!(result, Continuation::Restart);

    match receiver.try_recv().unwrap() {
        RRTEvent::Worker(PollerEvent::Stdin(StdinEvent::Error)) => {}
        other => panic!("Expected StdinEvent::Error, got {other:?}"),
    }

    println!("{POLL_ERROR_PASSED}");
    std::io::stdout().flush().expect("Failed to flush");

    std::process::exit(0);
}

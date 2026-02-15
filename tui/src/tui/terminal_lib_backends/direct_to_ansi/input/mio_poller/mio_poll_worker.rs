// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR epoll sigaction signalfd

//! mio-specific worker implementation for the Resilient Reactor Thread pattern.
//!
//! This module provides:
//! - [`MioPollWorker`]: Implements [`RRTWorker`] for terminal input handling
//! - [`MioPollWorkerFactory`]: Implements [`RRTFactory`] to create the worker
//!
//! These types integrate with the generic RRT infrastructure in
//! [`crate::core::resilient_reactor_thread`].
//!
//! [`RRTFactory`]: crate::core::resilient_reactor_thread::RRTFactory
//! [`RRTWorker`]: crate::core::resilient_reactor_thread::RRTWorker

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::PasteCollectionState,
                    stateful_parser::StatefulInputParser},
            SourceKindReady, SourceRegistry,
            dispatcher::dispatch_with_tx,
            handler_stdin::STDIN_READ_BUFFER_SIZE,
            mio_poll_waker::MioPollWaker};
use crate::{Continuation,
            core::resilient_reactor_thread::{RRTEvent, RRTFactory, RRTWorker}};
use miette::{Diagnostic, Report};
use mio::{Events, Interest, Poll, Waker, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::ErrorKind, os::fd::AsRawFd as _};
use tokio::sync::broadcast::Sender;

/// Capacity for the [`mio::Events`] buffer.
const EVENTS_CAPACITY: usize = 8;

/// mio-based worker for terminal input handling.
///
/// Implements [`RRTWorker`] to integrate with the generic RRT infrastructure. Each
/// call to [`poll_once()`] blocks until stdin data or signals are ready, processes them,
/// and returns whether to continue or stop.
///
/// # Resources Managed
///
/// | Resource              | Purpose                                    |
/// | :-------------------- | :----------------------------------------- |
/// | [`poll_handle`]       | Efficient I/O multiplexing via epoll       |
/// | [`sources`]           | stdin and SIGWINCH signal handles          |
/// | [`stdin_buffer`]      | Raw bytes read from stdin                  |
/// | [`parser`]            | VT100 input sequence parser                |
/// | [`paste_state`]       | Bracketed paste mode state machine         |
///
/// [`parser`]: Self::vt_100_input_seq_parser
/// [`paste_state`]: Self::paste_collection_state
/// [`poll_handle`]: Self::poll_handle
/// [`poll_once()`]: Self::poll_once
/// [`sources`]: Self::sources
/// [`stdin_buffer`]: Self::stdin_unparsed_byte_buffer
#[allow(missing_debug_implementations)]
pub struct MioPollWorker {
    /// [`mio`] poll instance for efficient I/O multiplexing.
    pub poll_handle: Poll,

    /// Buffer for events returned by [`Poll::poll()`].
    pub ready_events_buffer: Events,

    /// Registry of event sources (stdin, signals).
    pub sources: SourceRegistry,

    /// Buffer for reading unparsed bytes from stdin.
    pub stdin_unparsed_byte_buffer: [u8; STDIN_READ_BUFFER_SIZE],

    /// Stateful VT100 input sequence parser.
    pub vt_100_input_seq_parser: StatefulInputParser,

    /// Paste state machine for bracketed paste handling.
    pub paste_collection_state: PasteCollectionState,
}

impl RRTWorker for MioPollWorker {
    type Event = PollerEvent;

    /// Performs one iteration of the poll loop.
    ///
    /// Blocks until [`stdin`] or signals are ready, then processes all ready events.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Continue`]: Successfully processed or retryable error.
    /// - [`Continuation::Stop`]: Thread should exit (e.g., no receivers left).
    /// - [`Continuation::Restart`]: OS resources corrupted (non-[`EINTR`] poll error).
    ///
    /// See [EINTR handling] for how interrupted syscalls are retried.
    ///
    /// [EINTR handling]: super#eintr-handling
    /// [`stdin`]: std::io::stdin
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation {
        // Breaks borrow so dispatch can use `&mut self`.
        fn collect_ready_tokens(events: &Events) -> Vec<mio::Token> {
            events.iter().map(mio::event::Event::token).collect()
        }

        // Block until stdin or signals become ready.
        let poll_result = self.poll_handle.poll(&mut self.ready_events_buffer, None);

        // Handle poll errors.
        if let Err(err) = poll_result {
            // EINTR - retry (signal interrupted syscall).
            if err.kind() == ErrorKind::Interrupted {
                return Continuation::Continue;
            }

            // Non-EINTR poll error - OS resources likely corrupted. Notify
            // consumers and request restart via fresh F::create().
            drop(tx.send(PollerEvent::Stdin(StdinEvent::Error).into()));
            return Continuation::Restart;
        }

        // Dispatch ready events.
        for token in collect_ready_tokens(&self.ready_events_buffer) {
            let continuation = dispatch_with_tx(token, self, tx);
            if continuation == Continuation::Stop {
                return Continuation::Stop;
            }
        }

        Continuation::Continue
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │ Diagnostic error types for worker creation failures      │
// ╰──────────────────────────────────────────────────────────╯

/// Failed to create [`mio::Poll`] (epoll/kqueue creation failed).
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to create mio::Poll")]
#[diagnostic(
    code(r3bl_tui::mio::poll_creation),
    help("This usually means the system ran out of file descriptors")
)]
pub struct PollCreationError(#[source] pub std::io::Error);

/// Failed to create [`mio::Waker`] (eventfd/pipe creation failed).
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to create mio::Waker")]
#[diagnostic(
    code(r3bl_tui::mio::waker_creation),
    help("This usually means the system ran out of file descriptors")
)]
pub struct WakerCreationError(#[source] pub std::io::Error);

/// Failed to register stdin with mio.
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to register stdin with mio")]
#[diagnostic(
    code(r3bl_tui::mio::stdin_registration),
    help("Ensure stdin is a valid file descriptor")
)]
pub struct StdinRegistrationError(#[source] pub std::io::Error);

/// Failed to create SIGWINCH signal handler.
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to create SIGWINCH handler")]
#[diagnostic(
    code(r3bl_tui::mio::signal_creation),
    help("Signal handler creation failed - check system signal limits")
)]
pub struct SignalCreationError(#[source] pub std::io::Error);

/// Failed to register signals with mio.
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to register signals with mio")]
#[diagnostic(code(r3bl_tui::mio::signal_registration))]
pub struct SignalRegistrationError(#[source] pub std::io::Error);

// ╭──────────────────────────────────────────────────────────╮
// │ Factory                                                  │
// ╰──────────────────────────────────────────────────────────╯

/// Factory that creates [`MioPollWorker`] and [`MioPollWaker`] together.
///
/// Implements [`RRTFactory`] to integrate with the generic RRT infrastructure.
/// The [`create()`] method creates both the worker and waker from the same [`mio::Poll`]
/// instance, implementing [two-phase setup] where the waker needs the poll's registry.
///
/// [`create()`]: RRTFactory::create
/// [two-phase setup]: crate::core::resilient_reactor_thread#two-phase-setup
#[allow(missing_debug_implementations)]
pub struct MioPollWorkerFactory;

impl RRTFactory for MioPollWorkerFactory {
    type Event = PollerEvent;
    type Worker = MioPollWorker;
    type Waker = MioPollWaker;

    /// Creates a new mio poll worker and waker pair.
    ///
    /// This method:
    /// 1. Creates a new [`mio::Poll`] instance
    /// 2. Creates a [`mio::Waker`] from the poll's registry (for shutdown signaling)
    /// 3. Registers stdin and SIGWINCH with the poll
    /// 4. Returns both the worker (for the thread) and waker (for the global state)
    ///
    /// # Errors
    ///
    /// Returns [`Report`] if any OS resource creation or registration fails.
    fn create() -> Result<(Self::Worker, Self::Waker), Report> {
        // Create mio::Poll (epoll on Linux, kqueue on macOS).
        let poll_handle = Poll::new().map_err(PollCreationError)?;

        // Create waker from poll's registry (must be created BEFORE registering sources).
        let waker = Waker::new(
            poll_handle.registry(),
            SourceKindReady::ReceiverDropWaker.to_token(),
        )
        .map_err(WakerCreationError)?;

        let mio_registry = poll_handle.registry();

        // Register stdin with mio.
        let stdin = std::io::stdin();
        mio_registry
            .register(
                &mut SourceFd(&stdin.as_raw_fd()),
                SourceKindReady::Stdin.to_token(),
                Interest::READABLE,
            )
            .map_err(StdinRegistrationError)?;

        // Register SIGWINCH with signal-hook-mio.
        let mut signals = Signals::new([SIGWINCH]).map_err(SignalCreationError)?;
        mio_registry
            .register(
                &mut signals,
                SourceKindReady::Signals.to_token(),
                Interest::READABLE,
            )
            .map_err(SignalRegistrationError)?;

        let worker = MioPollWorker {
            poll_handle,
            ready_events_buffer: Events::with_capacity(EVENTS_CAPACITY),
            sources: SourceRegistry { stdin, signals },
            stdin_unparsed_byte_buffer: [0u8; STDIN_READ_BUFFER_SIZE],
            vt_100_input_seq_parser: StatefulInputParser::default(),
            paste_collection_state: PasteCollectionState::Inactive,
        };

        Ok((worker, MioPollWaker(waker)))
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR epoll sigaction signalfd

//! mio-specific worker implementation for the Resilient Reactor Thread pattern.
//!
//! This module provides:
//! - [`MioPollWorker`]: Implements [`ThreadWorker`] for terminal input handling
//! - [`MioPollWorkerFactory`]: Implements [`ThreadWorkerFactory`] to create the worker
//!
//! These types integrate with the generic RRT infrastructure in
//! [`crate::core::resilient_reactor_thread`].
//!
//! [`ThreadWorker`]: crate::core::resilient_reactor_thread::ThreadWorker
//! [`ThreadWorkerFactory`]: crate::core::resilient_reactor_thread::ThreadWorkerFactory

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::PasteCollectionState,
                    stateful_parser::StatefulInputParser},
            SourceKindReady, SourceRegistry,
            dispatcher::dispatch_with_tx,
            handler_stdin::STDIN_READ_BUFFER_SIZE,
            mio_poll_waker::MioPollWaker};
use crate::{Continuation,
            core::resilient_reactor_thread::{ThreadWorker, ThreadWorkerFactory}};
use mio::{Events, Interest, Poll, Waker, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::ErrorKind, os::fd::AsRawFd as _};
use tokio::sync::broadcast::Sender;

/// Capacity for the [`mio::Events`] buffer.
const EVENTS_CAPACITY: usize = 8;

/// mio-based worker for terminal input handling.
///
/// Implements [`ThreadWorker`] to integrate with the generic RRT infrastructure. Each
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
/// [`poll_once()`]: Self::poll_once
/// [`poll_handle`]: Self::poll_handle
/// [`sources`]: Self::sources
/// [`stdin_buffer`]: Self::stdin_unparsed_byte_buffer
/// [`parser`]: Self::vt_100_input_seq_parser
/// [`paste_state`]: Self::paste_collection_state
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

impl ThreadWorker for MioPollWorker {
    type Event = PollerEvent;

    /// Performs one iteration of the poll loop.
    ///
    /// Blocks until stdin or signals are ready, then processes all ready events. Returns
    /// [`Continuation::Stop`] if the thread should exit (e.g., no receivers left),
    /// otherwise returns [`Continuation::Continue`].
    ///
    /// # EINTR Handling
    ///
    /// If [`poll()`] is interrupted by a signal ([`ErrorKind::Interrupted`]), this method
    /// returns [`Continuation::Continue`] to retry. This is the standard Unix pattern for
    /// handling [`EINTR`].
    ///
    /// [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`poll()`]: mio::Poll::poll
    fn poll_once(&mut self, tx: &Sender<Self::Event>) -> Continuation {
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

            // Fatal error - notify consumers and exit.
            drop(tx.send(PollerEvent::Stdin(StdinEvent::Error)));
            return Continuation::Stop;
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

/// Error type for [`MioPollWorkerFactory::setup()`] failures.
#[derive(Debug, thiserror::Error)]
pub enum MioPollSetupError {
    /// Failed to create [`mio::Poll`] (epoll/kqueue creation failed).
    #[error("Failed to create mio::Poll: {0}")]
    PollCreation(#[source] std::io::Error),

    /// Failed to create [`mio::Waker`] (eventfd/pipe creation failed).
    #[error("Failed to create mio::Waker: {0}")]
    WakerCreation(#[source] std::io::Error),

    /// Failed to register stdin with mio.
    #[error("Failed to register stdin with mio: {0}")]
    StdinRegistration(#[source] std::io::Error),

    /// Failed to create SIGWINCH signal handler.
    #[error("Failed to create SIGWINCH handler: {0}")]
    SignalCreation(#[source] std::io::Error),

    /// Failed to register signals with mio.
    #[error("Failed to register signals with mio: {0}")]
    SignalRegistration(#[source] std::io::Error),
}

/// Factory that creates [`MioPollWorker`] and [`MioPollWaker`] together.
///
/// Implements [`ThreadWorkerFactory`] to integrate with the generic RRT infrastructure.
/// The [`setup()`] method creates both the worker and waker from the same [`mio::Poll`]
/// instance, solving the chicken-egg problem where the waker needs the poll's registry.
///
/// [`setup()`]: Self::setup
#[allow(missing_debug_implementations)]
pub struct MioPollWorkerFactory;

impl ThreadWorkerFactory for MioPollWorkerFactory {
    type Event = PollerEvent;
    type Worker = MioPollWorker;
    type Waker = MioPollWaker;
    type SetupError = MioPollSetupError;

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
    /// Returns [`MioPollSetupError`] if any OS resource creation or registration fails.
    fn setup() -> Result<(Self::Worker, Self::Waker), Self::SetupError> {
        // Create mio::Poll.
        let poll_handle = Poll::new().map_err(MioPollSetupError::PollCreation)?;

        // Create waker from poll's registry (must be created BEFORE registering sources).
        let waker = Waker::new(
            poll_handle.registry(),
            SourceKindReady::ReceiverDropWaker.to_token(),
        )
        .map_err(MioPollSetupError::WakerCreation)?;

        let mio_registry = poll_handle.registry();

        // Register stdin with mio.
        let stdin = std::io::stdin();
        mio_registry
            .register(
                &mut SourceFd(&stdin.as_raw_fd()),
                SourceKindReady::Stdin.to_token(),
                Interest::READABLE,
            )
            .map_err(MioPollSetupError::StdinRegistration)?;

        // Register SIGWINCH with signal-hook-mio.
        let mut signals =
            Signals::new([SIGWINCH]).map_err(MioPollSetupError::SignalCreation)?;
        mio_registry
            .register(
                &mut signals,
                SourceKindReady::Signals.to_token(),
                Interest::READABLE,
            )
            .map_err(MioPollSetupError::SignalRegistration)?;

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

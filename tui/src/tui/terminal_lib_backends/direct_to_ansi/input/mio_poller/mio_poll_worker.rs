// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words EINTR epoll sigaction signalfd fcntl getfl setfl NONBLOCK

//! mio-specific worker implementation for the Resilient Reactor Thread pattern.
//!
//! This module provides [`MioPollWorker`], which implements [`RRTWorker`] for terminal
//! input handling - including both resource creation
//! ([`create_and_register_os_sources()`]) and the blocking poll loop
//! ([`block_until_ready_then_dispatch()`]).
//!
//! This type integrates with the generic RRT infrastructure in
//! [`crate::core::resilient_reactor_thread`].
//!
//! [`block_until_ready_then_dispatch()`]:
//!     crate::RRTWorker::block_until_ready_then_dispatch
//! [`create_and_register_os_sources()`]: crate::RRTWorker::create_and_register_os_sources
//! [`RRTWorker`]: crate::RRTWorker

use super::{super::{channel_types::{PollerEvent, StdinEvent},
                    paste_state_machine::PasteCollectionState,
                    stateful_parser::StatefulInputParser},
            MioSoftwareInterrupt, SourceKindReady,
            dispatcher::dispatch_with_sender,
            handler_stdin::STDIN_READ_BUFFER_SIZE,
            sources::SourceRegistry};
// Imported specifically for the intra-doc links in the struct documentation.
#[allow(unused_imports)]
use super::handler_stdin::consume_stdin_input_with_sender;
use crate::{Continuation,
            core::resilient_reactor_thread::{RRTEvent, RRTWorker}};
use miette::Diagnostic;
use mio::{Events, Interest, Poll, unix::SourceFd};
use signal_hook::consts::SIGWINCH;
use signal_hook_mio::v1_0::Signals;
use std::{io::ErrorKind, os::fd::AsRawFd as _};
use tokio::sync::broadcast::Sender;

/// Capacity for the [`mio::Events`] buffer.
const EVENTS_CAPACITY: usize = 8;

/// [`mio`]-based worker for terminal input handling.
///
/// Implements [`RRTWorker`] to integrate with the generic RRT infrastructure. Each call
/// to [`block_until_ready_then_dispatch()`] blocks until [`stdin`] data or signals are
/// ready, processes them, and returns whether to continue or stop.
///
/// This struct works hand in hand with [`mio_poller::consume_stdin_input_with_sender`].
/// Read the [Why We Need Non-Blocking Read] section for more details.
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
/// ## How this affects [`stdout`] as well
///
/// Because [`stdin`] and [`stdout`] share the same underlying file description on Linux,
/// setting `O_NONBLOCK` on [`stdin`] accidentally makes [`stdout`] non-blocking as well.
/// See the [How this affects stdout as well] section for details on how this is handled,
/// and see [`FullBufferWaitingStdout`] / [`OutputDevice::new_stdout()`] for the
/// implementation of the fix.
///
/// [`block_until_ready_then_dispatch()`]: MioPollWorker::block_until_ready_then_dispatch
/// [`FullBufferWaitingStdout`]: crate::core::terminal_io::FullBufferWaitingStdout
/// [`mio_poller::consume_stdin_input_with_sender`]:
///     super::handler_stdin::consume_stdin_input_with_sender
/// [`OutputDevice::new_stdout()`]: crate::core::terminal_io::OutputDevice::new_stdout
/// [`parser`]: field@MioPollWorker::vt_100_input_seq_parser
/// [`paste_state`]: field@MioPollWorker::paste_collection_state
/// [`poll_handle`]: field@MioPollWorker::poll_handle
/// [`sources`]: field@MioPollWorker::sources
/// [`stdin_buffer`]: field@MioPollWorker::stdin_unparsed_byte_buffer
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [How this affects stdout as well]:
///     consume_stdin_input_with_sender#how-this-affects-stdout-as-well
/// [Why We Need Non-Blocking Read]:
///     consume_stdin_input_with_sender#why-we-need-non-blocking-read
#[derive(Debug)]
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

    /// Original [`stdin`] file status flags to restore on [`Drop`].
    ///
    /// [`Drop`]: #method.drop
    /// [`stdin`]: std::io::stdin
    pub original_stdin_flags: Option<rustix::fs::OFlags>,
}

impl RRTWorker for MioPollWorker {
    type Event = PollerEvent;
    type Interrupt = MioSoftwareInterrupt;

    /// Creates a new mio poll worker and [`MioSoftwareInterrupt`] pair.
    ///
    /// This method:
    /// 1. Creates a new [`mio::Poll`] instance
    /// 2. Creates a [`MioSoftwareInterrupt`] from the poll's registry (for shutdown
    ///    signaling)
    /// 3. Registers stdin and [`SIGWINCH`] with the poll (and crucially, sets stdin to
    ///    non-blocking mode which is required by [`consume_stdin_input_with_sender()`])
    /// 4. Returns both the worker (for the thread) and a [`MioSoftwareInterrupt`]
    ///    wrapping the [`mio::Waker`] (for the global state)
    ///
    /// The [`MioSoftwareInterrupt`] is tightly coupled to this worker's [`mio::Poll`] -
    /// it was created from the same poll's registry. If the poll is dropped, calling
    /// [`trigger_software_interrupt()`] has no effect. This is why
    /// [`create_and_register_os_sources()`] returns both together.
    ///
    /// # Errors
    ///
    /// Returns [`miette::Report`] if any OS resource creation or registration fails.
    ///
    /// [`consume_stdin_input_with_sender()`]:
    ///     super::handler_stdin::consume_stdin_input_with_sender
    /// [`create_and_register_os_sources()`]: RRTWorker::create_and_register_os_sources
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker`]: mio::Waker
    /// [`trigger_software_interrupt()`]:
    ///     crate::RRTSoftwareInterrupt::trigger_software_interrupt
    fn create_and_register_os_sources() -> miette::Result<(Self, Self::Interrupt)> {
        // Create mio::Poll (epoll on Linux, kqueue on macOS).
        let poll_handle = Poll::new().map_err(PollCreationError)?;
        let mio_registry = poll_handle.registry();

        // Create & register the synthetic software interrupt.

        // Create waker from poll's registry (must be created BEFORE registering sources).
        let software_interrupt = MioSoftwareInterrupt::create_and_register_synthetic_software_interrupt_source(
            mio_registry,
            SourceKindReady::SoftwareInterrupt.to_token(),
        )?;

        // DATA PLANE: Register real hardware/OS sources (stdin, signals).

        // Register stdin with mio.
        let stdin = std::io::stdin();
        let original_stdin_flags = if let Ok(flags) = rustix::fs::fcntl_getfl(&stdin) {
            let _ = rustix::fs::fcntl_setfl(&stdin, flags | rustix::fs::OFlags::NONBLOCK);
            Some(flags)
        } else {
            None
        };

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

        Ok((
            MioPollWorker {
                poll_handle,
                ready_events_buffer: Events::with_capacity(EVENTS_CAPACITY),
                sources: SourceRegistry { stdin, signals },
                stdin_unparsed_byte_buffer: [0u8; STDIN_READ_BUFFER_SIZE],
                vt_100_input_seq_parser: StatefulInputParser::default(),
                paste_collection_state: PasteCollectionState::Inactive,
                original_stdin_flags,
            },
            software_interrupt,
        ))
    }

    /// Blocks until at least one I/O source is ready, then dispatches events - see
    /// [`MioPollWorker::block_until_ready_then_dispatch_impl()`] for details.
    ///
    /// It's not possible to link to a trait method implementation on a struct (the link
    /// just goes to the trait's method definition) - which is why this method just
    /// delegates to a separate `block_until_ready_then_dispatch_impl()` method where the
    /// real implementation lives, which we can link to directly.
    ///
    /// [`MioPollWorker::block_until_ready_then_dispatch_impl()`]: Self::block_until_ready_then_dispatch_impl
    fn block_until_ready_then_dispatch(
        &mut self,
        sender: &Sender<RRTEvent<Self::Event>>,
    ) -> Continuation {
        self.block_until_ready_then_dispatch_impl(sender)
    }
}

impl MioPollWorker {
    /// Performs one iteration of the poll loop.
    ///
    /// Blocks until [`stdin`] or signals are ready, then processes all ready events.
    ///
    /// # Returns
    ///
    /// - [`Continuation::Continue`]: Successfully processed or retryable error.
    /// - [`Continuation::Stop`]: Worker-domain stop (e.g., EOF/fatal worker condition).
    /// - [`Continuation::Restart`]: OS resources corrupted (non-[`EINTR`] poll error).
    ///
    /// See [`EINTR` handling] for how interrupted syscalls are retried.
    ///
    /// [`EINTR` handling]: super#eintr-handling
    /// [`EINTR`]: super#eintr-handling
    /// [`stdin`]: std::io::stdin
    pub fn block_until_ready_then_dispatch_impl(
        &mut self,
        sender: &Sender<RRTEvent<PollerEvent>>,
    ) -> Continuation {
        // Block until stdin or signals become ready.
        let poll_result = self.poll_handle.poll(&mut self.ready_events_buffer, None);

        // Handle poll errors.
        if let Err(err) = poll_result {
            // EINTR - retry (signal interrupted syscall).
            if err.kind() == ErrorKind::Interrupted {
                return Continuation::Continue;
            }

            // Non-EINTR poll error - OS resources likely corrupted. Notify
            // consumers and request restart via fresh create().
            drop(sender.send(PollerEvent::Stdin(StdinEvent::Error).into()));
            return Continuation::Restart;
        }

        // Dispatch ready events.
        let ready_tokens = Self::collect_ready_tokens(&self.ready_events_buffer);
        for token in ready_tokens {
            let continuation = dispatch_with_sender(token, self, sender);
            if continuation == Continuation::Stop {
                return Continuation::Stop;
            }
        }

        Continuation::Continue
    }

    /// Collects tokens into a Vec so that [`ready_events_buffer`] is no longer borrowed
    /// when [`dispatch_with_sender`] takes `&mut self`.
    ///
    /// [`ready_events_buffer`]: field@MioPollWorker::ready_events_buffer
    pub fn collect_ready_tokens(events: &Events) -> Vec<mio::Token> {
        events.iter().map(mio::event::Event::token).collect()
    }
}

impl Drop for MioPollWorker {
    /// Restores the original [`stdin`] file status flags when the worker is dropped.
    ///
    /// This is an [`RAII`] guard that ensures we don't permanently leave the terminal's
    /// [`stdin`] in non-blocking mode if the poller thread panics or the application
    /// exits. Failing to do so would break the user's shell after the application exits.
    ///
    /// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
    /// [`stdin`]: std::io::stdin
    fn drop(&mut self) {
        if let Some(original_flags) = self.original_stdin_flags {
            let _ = rustix::fs::fcntl_setfl(&self.sources.stdin, original_flags);
        }
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

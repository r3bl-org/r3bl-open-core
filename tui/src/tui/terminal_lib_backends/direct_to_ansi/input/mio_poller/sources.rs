// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Source registry and types for the [`mio`] poller event loop.

use mio::Token;
use signal_hook_mio::v1_0::Signals;
use std::io::Stdin;

/// Registry of all event sources monitored by [`mio::Poll`].
///
/// This struct centralizes the management of heterogeneous event sources ([`stdin`],
/// [signals]) that are registered with [`mio`] for I/O multiplexing. Each source has a
/// corresponding [`Token`] in [`SourceKindReady`] for dispatch routing.
///
/// # What is a "Source"?
///
/// In [`mio`] terminology, a "source" is anything registered with [`Poll`] to be
/// monitored for readiness. When a source becomes readable, [`Poll::poll()`] returns
/// an event with the source's [`Token`], and we must consume data from that source.
///
/// # Design Rationale
///
/// While a [`HashMap<Token, Source>`] might seem appealing, the sources have different
/// types ([`Stdin`] vs [`Signals`]) with different consumption patterns:
/// - **[`Stdin`]**: Call [`read()`] to get bytes.
/// - **[`Signals`]**: Call [`pending()`] to drain signal queue.
///
/// This struct provides type safety while formalizing the token→source relationship.
///
/// # Adding New Sources
///
/// To add a new event source:
/// 1. Add a new field to this struct.
/// 2. Add a new variant and token constant to [`SourceKindReady`].
/// 3. Register the source in [`MioPollerThread::setup()`].
/// 4. Add a handler function in [`handler_stdin`] or [`handler_signals`].
/// 5. Add a match arm in [`dispatch()`].
///
/// [`dispatch()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::dispatcher::dispatch
/// [`handler_signals`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_signals
/// [`handler_stdin`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin
/// [`HashMap<Token, Source>`]: std::collections::HashMap
/// [`MioPollerThread::setup()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::poller_thread::MioPollerThread::setup
/// [`Poll::poll()`]: mio::Poll::poll
/// [`Poll`]: mio::Poll
/// [`Signals`]: signal_hook_mio::v1_0::Signals
/// [`Stdin`]: std::io::Stdin
/// [`Token`]: mio::Token
/// [`pending()`]: signal_hook_mio::v1_0::Signals::pending
/// [`read()`]: std::io::Read::read
/// [`stdin`]: std::io::stdin
/// [signals]: signal_hook_mio::v1_0::Signals
#[allow(missing_debug_implementations)]
pub struct SourceRegistry {
    /// [`Stdin`] handle registered with [`MioPollerThread::poll_handle`].
    ///
    /// See [What is a "Source"?] for [`mio`] terminology.
    ///
    /// - **Token**: [`SourceKindReady::Stdin`].[`to_token()`].
    /// - **Handler**: [`consume_stdin_input()`].
    ///
    /// [What is a "Source"?]: SourceRegistry#what-is-a-source
    /// [`consume_stdin_input()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::consume_stdin_input
    /// [`MioPollerThread::poll_handle`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::poller_thread::MioPollerThread::poll_handle
    /// [`to_token()`]: SourceKindReady::to_token
    pub stdin: Stdin,

    /// [`SIGWINCH`] signal handler registered with [`MioPollerThread::poll_handle`].
    ///
    /// See [What is a "Source"?] for [`mio`] terminology. [`signal_hook_mio`] provides
    /// an adapter that creates an internal pipe becoming readable when [`SIGWINCH`]
    /// arrives.
    ///
    /// - **Token**: [`SourceKindReady::Signals`].[`to_token()`].
    /// - **Handler**: [`consume_pending_signals()`].
    ///
    /// [What is a "Source"?]: SourceRegistry#what-is-a-source
    /// [`consume_pending_signals()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_signals::consume_pending_signals
    /// [`MioPollerThread::poll_handle`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::poller_thread::MioPollerThread::poll_handle
    /// [`signal_hook_mio`]: signal_hook_mio
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`to_token()`]: SourceKindReady::to_token
    pub signals: Signals,
}

/// Identifies which event source became ready.
///
/// This enum is the single source of truth for [`mio`] [`Token`] ↔ source mapping.
/// Each variant (except [`Unknown`]) has an associated token used for registration
/// and dispatch.
///
/// # How Tokens Work
///
/// When [`Poll::poll()`] returns, each event carries a [`Token`] identifying which
/// registered source became ready. Use [`from_token()`] to convert a token to this
/// enum, then match on the variant to dispatch to the appropriate handler.
///
/// [`from_token()`]: SourceKindReady::from_token
/// [`Poll::poll()`]: mio::Poll::poll
/// [`Token`]: mio::Token
/// [`Unknown`]: SourceKindReady::Unknown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKindReady {
    /// [`SourceRegistry::stdin`] has data available to read.
    Stdin,
    /// [`SourceRegistry::signals`] received [`SIGWINCH`].
    ///
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    Signals,
    /// Unknown token - should not happen in normal operation.
    Unknown,
}

impl SourceKindReady {
    /// Returns the [`Token`] associated with this source kind.
    ///
    /// Used when registering sources with [`mio::Registry`]. This is the inverse
    /// of [`from_token()`].
    ///
    /// # Panics
    ///
    /// Panics if called on [`SourceKindReady::Unknown`].
    ///
    /// [`from_token()`]: SourceKindReady::from_token
    /// [`Token`]: mio::Token
    /// [`mio::Registry`]: mio::Registry
    #[must_use]
    pub const fn to_token(self) -> Token {
        match self {
            Self::Stdin => Token(0),
            Self::Signals => Token(1),
            Self::Unknown => panic!("Unknown source has no token"),
        }
    }

    /// Converts a [`Token`] to the corresponding [`SourceKindReady`] variant.
    ///
    /// This is the inverse of [`to_token()`]. Used when dispatching ready events
    /// from [`Poll::poll()`].
    ///
    /// [`Poll::poll()`]: mio::Poll::poll
    /// [`Token`]: mio::Token
    /// [`to_token()`]: SourceKindReady::to_token
    #[must_use]
    pub const fn from_token(token: Token) -> Self {
        match token.0 {
            0 => Self::Stdin,
            1 => Self::Signals,
            _ => Self::Unknown,
        }
    }
}

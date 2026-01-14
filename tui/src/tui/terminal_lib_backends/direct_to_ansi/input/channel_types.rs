// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared type definitions for the [`DirectToAnsi`] input device.
//!
//! The [`mio_poller`] thread monitors multiple sources and sends [`PollerEvent`]s through
//! a broadcast channel:
//! - **[`stdin`] fd**: Keyboard/mouse input, [`EOF`], and read errors → [`StdinEvent`]
//! - **signal fd**: Terminal signals like [`SIGWINCH`] → [`SignalEvent`]
//!
//! [`DirectToAnsi`]: crate::direct_to_ansi
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`SIGWINCH`]: signal_hook::consts::SIGWINCH
//! [`mio_poller`]: super::mio_poller
//! [`stdin`]: std::io::stdin

use crate::{InputEvent, Size};

/// Event from the [`mio_poller`] thread, sent through a broadcast channel.
///
/// The poller monitors multiple file descriptors and produces semantic events
/// grouped by source. Requires [`Clone`] because [`tokio::sync::broadcast`] clones
/// messages for each receiver.
///
/// See [`DirectToAnsiInputDevice`'s Architecture section] for architecture details.
///
/// [`DirectToAnsiInputDevice`'s Architecture section]: super::DirectToAnsiInputDevice#architecture
/// [`mio_poller`]: super::mio_poller
#[derive(Debug, Clone, PartialEq)]
pub enum PollerEvent {
    /// Events from the [`stdin`] file descriptor.
    ///
    /// [`stdin`]: std::io::stdin
    Stdin(StdinEvent),
    /// Events from signal handlers.
    Signal(SignalEvent),
}

/// Events originating from the [`stdin`] file descriptor.
///
/// These events are produced when the [`mio_poller`] detects that [`stdin`] is ready
/// for reading and subsequently reads/parses the input.
///
/// [`mio_poller`]: super::mio_poller
/// [`stdin`]: std::io::stdin
#[derive(Debug, Clone, PartialEq)]
pub enum StdinEvent {
    /// Parsed input event (keyboard/mouse) ready for consumption.
    Input(InputEvent),
    /// [`EOF`] reached (`0` bytes read from [`stdin`]).
    ///
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    /// [`stdin`]: std::io::stdin
    Eof,
    /// Error occurred during [`stdin`] read (details logged in poller thread).
    ///
    /// [`stdin`]: std::io::stdin
    Error,
}

/// Events originating from OS signal handlers.
///
/// These events are produced when the [`mio_poller`] detects activity on the
/// signal file descriptor.
///
/// [`mio_poller`]: super::mio_poller
#[derive(Debug, Clone, PartialEq)]
pub enum SignalEvent {
    /// Terminal resize signal ([`SIGWINCH`]) received.
    ///
    /// Contains [`Some(size)`] if [`get_size()`] succeeded, or [`None`] if the
    /// size query failed (rare—typically means no [TTY], e.g., during [SSH]
    /// disconnect or terminal crash).
    ///
    /// The consumer should handle [`None`] by either:
    /// - Retrying [`get_size()`] themselves
    /// - Using a cached/default size
    /// - Ignoring the resize event
    ///
    /// [SSH]: https://en.wikipedia.org/wiki/Secure_Shell
    /// [TTY]: https://en.wikipedia.org/wiki/Tty_(Unix)
    /// [`InputEvent::Resize`]: crate::InputEvent::Resize
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`Some(size)`]: Option::Some
    /// [`get_size()`]: crate::core::term::get_size
    Resize(Option<Size>),
}

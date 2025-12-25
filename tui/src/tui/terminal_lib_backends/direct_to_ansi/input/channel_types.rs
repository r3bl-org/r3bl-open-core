// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared type definitions for the [`DirectToAnsi`] input device.
//!
//! [`DirectToAnsi`]: crate::direct_to_ansi

use crate::InputEvent;

/// Sender end of the broadcast channel for input events.
///
/// Carries [`StdinReaderMessage`] variants: keyboard/mouse input from [`stdin`], resize
/// events from [`SIGWINCH`], EOF, and errors.
///
/// The sender can be cloned to create additional receivers via [`Sender::subscribe`].
///
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`Sender::subscribe`]: tokio::sync::broadcast::Sender::subscribe
/// [`stdin`]: std::io::stdin
pub type StdinReaderMessageSender = tokio::sync::broadcast::Sender<StdinReaderMessage>;

/// Receiver end of the broadcast channel for input events.
///
/// Carries [`StdinReaderMessage`] variants: keyboard/mouse input from [`stdin`], resize
/// events from [`SIGWINCH`], EOF, and errors.
///
/// Multiple receivers can exist simultaneously—each receives all messages sent after
/// it was created. If a receiver lags behind (buffer fills up), it will receive
/// [`RecvError::Lagged`] indicating how many messages were skipped.
///
/// [`RecvError::Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`stdin`]: std::io::stdin
pub type StdinReaderMessageReceiver =
    tokio::sync::broadcast::Receiver<StdinReaderMessage>;

/// Message from the stdin reader thread, sent through a broadcast channel.
///
/// Requires [`Clone`] because [`tokio::sync::broadcast`] clones messages for each
/// receiver. See [`global_input_resource`] for architecture details.
///
/// [`global_input_resource`]: super::global_input_resource
#[derive(Debug, Clone)]
pub enum StdinReaderMessage {
    /// Parsed input event ready for consumption.
    Event(InputEvent),
    /// EOF reached (0 bytes read).
    Eof,
    /// Error occurred during read (error details are logged in reader thread).
    Error,
    /// Terminal resize signal (`SIGWINCH`) received.
    ///
    /// The reader thread detected a window size change. The async side should
    /// query the new terminal size via [`get_size()`] and emit an
    /// [`InputEvent::Resize`].
    ///
    /// [`InputEvent::Resize`]: crate::InputEvent::Resize
    /// [`get_size()`]: crate::core::term::get_size
    Resize,
}

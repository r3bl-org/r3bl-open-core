// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared type definitions for the `DirectToAnsi` input device.

use crate::InputEvent;

/// Control flow signal for the mio poller thread's main loop.
///
/// Used by [`MioPoller`] methods to indicate what the main loop should do next.
/// This operates at the thread level, controlling whether to continue polling
/// or terminate the thread entirely.
///
/// [`MioPoller`]: super::mio_poller::MioPoller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadLoopContinuation {
    /// Continue to the next iteration of the event loop.
    Continue,
    /// Return from the thread function (used for EOF, fatal errors, or receiver dropped).
    Return,
}

/// Capacity of the broadcast channel for input events.
///
/// When the buffer is full, the oldest message is dropped to make room for new ones.
/// Slow consumers will receive [`Lagged`] on their next [`recv()`] call, indicating how
/// many messages they missed.
///
/// `4_096` is generous for terminal input (you'd never have that many pending
/// keypresses), but it's cheap (each [`ReaderThreadMessage`] is small) and provides
/// headroom for debug/logging consumers that might occasionally lag.
///
/// [`Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
/// [`recv()`]: tokio::sync::broadcast::Receiver::recv
pub const CHANNEL_CAPACITY: usize = 4_096;

/// Sender end of the broadcast channel for input events.
///
/// Carries [`ReaderThreadMessage`] variants: keyboard/mouse input from [`stdin`], resize
/// events from [`SIGWINCH`], EOF, and errors.
///
/// The sender can be cloned to create additional receivers via [`Sender::subscribe`].
///
/// [`stdin`]: std::io::stdin
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`Sender::subscribe`]: tokio::sync::broadcast::Sender::subscribe
pub type InputEventSender = tokio::sync::broadcast::Sender<ReaderThreadMessage>;

/// Receiver end of the broadcast channel for input events.
///
/// Carries [`ReaderThreadMessage`] variants: keyboard/mouse input from [`stdin`], resize
/// events from [`SIGWINCH`], EOF, and errors.
///
/// Multiple receivers can exist simultaneously—each receives all messages sent after
/// it was created. If a receiver lags behind (buffer fills up), it will receive
/// [`RecvError::Lagged`] indicating how many messages were skipped.
///
/// [`stdin`]: std::io::stdin
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`RecvError::Lagged`]: tokio::sync::broadcast::error::RecvError::Lagged
pub type InputEventReceiver = tokio::sync::broadcast::Receiver<ReaderThreadMessage>;

/// Result from the paste state machine indicating how to proceed.
///
/// Used by [`apply_paste_state_machine`] to communicate back to the reader loop.
/// Each call returns this result to indicate:
/// - An event is ready to emit
/// - The event was absorbed (e.g., collecting paste data)
///
/// [`apply_paste_state_machine`]: super::paste_state_machine::apply_paste_state_machine
#[allow(missing_debug_implementations)]
pub enum PasteStateResult {
    /// Emit this event to the caller.
    Emit(InputEvent),
    /// Event absorbed by the state machine (e.g., paste in progress).
    Absorbed,
}

/// Message from the stdin reader thread, sent through a broadcast channel.
///
/// Requires [`Clone`] because [`tokio::sync::broadcast`] clones messages for each
/// receiver. See [`global_input_resource`] for architecture details.
///
/// [`global_input_resource`]: mod@super::global_input_resource
#[derive(Debug, Clone)]
pub enum ReaderThreadMessage {
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
    /// [`get_size()`]: crate::core::term::get_size
    /// [`InputEvent::Resize`]: crate::InputEvent::Resize
    Resize,
}

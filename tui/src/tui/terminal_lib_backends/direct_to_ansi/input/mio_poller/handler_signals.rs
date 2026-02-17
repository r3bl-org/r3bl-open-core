// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event handlers for signal processing.

use super::{super::channel_types::PollerEvent, MioPollWorker};
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent, get_size,
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use signal_hook::consts::SIGWINCH;
use tokio::sync::broadcast::Sender;

/// Handles [`SIGWINCH`] signal (terminal resize), using explicit `sender` parameter.
///
/// Drains all pending signals, queries the new terminal size, and sends a single
/// resize event to the channel. Multiple coalesced signals result in one event
/// since we query the current size at send time.
///
/// If [`get_size()`] fails (rare - typically means TTY disconnected), the signal
/// is silently dropped since there is no useful size to report.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `sender` as a parameter.
///
/// # Returns
///
/// - [`Continuation::Continue`]: Successfully processed.
/// - [`Continuation::Stop`]: Receiver dropped.
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`RRTWorker`]: crate::core::resilient_reactor_thread::RRTWorker
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`get_size()`]: crate::get_size
pub fn consume_pending_signals_with_sender(
    worker: &mut MioPollWorker,
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    // Drain all pending signals and check if any SIGWINCH arrived.
    let sigwinch_arrived = worker.sources.signals.pending().any(|sig| sig == SIGWINCH);

    if sigwinch_arrived {
        // Query terminal size. If it fails, drop the signal - there's no useful size to
        // report (typically means TTY disconnected).
        let Some(size) = get_size().ok() else {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message =
                        "mio-poller-thread: SIGWINCH received but get_size() failed"
                );
            });
            return Continuation::Continue;
        };

        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: SIGWINCH received", ?size);
        });

        if sender
            .send(PollerEvent::Signal(size.into()).into())
            .is_err()
        {
            // Receiver dropped - exit gracefully.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "mio-poller-thread: receiver dropped, exiting");
            });
            return Continuation::Stop;
        }
    }

    Continuation::Continue
}

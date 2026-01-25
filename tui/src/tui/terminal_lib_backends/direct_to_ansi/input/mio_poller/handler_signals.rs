// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event handlers for signal processing.

use super::{super::channel_types::{PollerEvent, SignalEvent},
            MioPollWorker};
use crate::{Continuation, get_size, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use signal_hook::consts::SIGWINCH;
use tokio::sync::broadcast::Sender;

/// Handles [`SIGWINCH`] signal (terminal resize), using explicit `tx` parameter.
///
/// Drains all pending signals, queries the new terminal size, and sends a single
/// resize event to the channel. Multiple coalesced signals result in one event
/// since we query the current size at send time.
///
/// The event contains [`Some(size)`] if [`get_size()`] succeeded, or [`None`] if
/// the query failed (rare—typically means TTY disconnected). The consumer decides
/// how to handle the [`None`] case.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `tx` as a parameter.
///
/// # Returns
///
/// - [`Continuation::Continue`]: Successfully processed.
/// - [`Continuation::Stop`]: Receiver dropped.
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`RRTWorker`]: crate::core::resilient_reactor_thread::RRTWorker
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
/// [`Some(size)`]: Option::Some
/// [`get_size()`]: crate::get_size
pub fn consume_pending_signals_with_tx(
    worker: &mut MioPollWorker,
    tx: &Sender<PollerEvent>,
) -> Continuation {
    // Drain all pending signals and check if any SIGWINCH arrived.
    let sigwinch_arrived = worker.sources.signals.pending().any(|sig| sig == SIGWINCH);

    if sigwinch_arrived {
        // Query terminal size - wrap in Option so consumer knows if it failed.
        let maybe_size = get_size().ok();

        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(
                message = "mio-poller-thread: SIGWINCH received",
                ?maybe_size
            );
        });

        if tx
            .send(PollerEvent::Signal(SignalEvent::Resize(maybe_size)))
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

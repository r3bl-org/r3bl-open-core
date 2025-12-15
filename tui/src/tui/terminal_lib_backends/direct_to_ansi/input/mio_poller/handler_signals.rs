// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event handlers for signal processing.

use super::poller_thread::MioPollerThread;
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::types::{ReaderThreadMessage,
                                                                       ThreadLoopContinuation}};
use signal_hook::consts::SIGWINCH;

/// Handles [`SIGWINCH`] signal (terminal resize).
///
/// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
///
/// Drains all pending signals and sends a single resize event to the channel.
/// Multiple coalesced signals result in one event since resize is idempotent—the
/// consumer queries the current terminal size regardless of how many signals arrived.
///
/// # Returns
///
/// - [`ThreadLoopContinuation::Continue`]: Successfully processed.
/// - [`ThreadLoopContinuation::Return`]: Receiver dropped.
pub fn consume_pending_signals(poller: &mut MioPollerThread) -> ThreadLoopContinuation {
    // Drain all pending signals and check if any SIGWINCH arrived.
    // Multiple signals may coalesce between polls, but we only need one Resize event.
    let sigwinch_arrived = poller.sources.signals.pending().any(|sig| sig == SIGWINCH);

    if sigwinch_arrived {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: SIGWINCH received");
        });
        if poller
            .tx_parsed_input_events
            .send(ReaderThreadMessage::Resize)
            .is_err()
        {
            // Receiver dropped - exit gracefully.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "mio-poller-thread: receiver dropped, exiting");
            });
            return ThreadLoopContinuation::Return;
        }
    }

    ThreadLoopContinuation::Continue
}

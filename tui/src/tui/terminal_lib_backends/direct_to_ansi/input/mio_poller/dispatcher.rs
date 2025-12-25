// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event dispatching for the [`mio`] poller event loop.

use super::{ThreadLoopContinuation, handler_receiver_drop::handle_receiver_drop_waker,
            handler_signals::consume_pending_signals,
            handler_stdin::consume_stdin_input, poller_thread::MioPollerThread,
            sources::SourceKindReady};
use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
use mio::Token;

/// Dispatches to the appropriate handler based on the [`Token`].
///
/// This centralizes the token → handler mapping, making it easier to add new
/// sources—just add a variant and its match arm here.
///
/// # Arguments
///
/// - `token`: The [`mio::Token`] identifying which source became ready.
/// - `poller`: The [`MioPollerThread`] containing the state for handlers.
///
/// # Returns
///
/// - [`ThreadLoopContinuation::Continue`]: Event handled, continue polling.
/// - [`ThreadLoopContinuation::Return`]: Exit condition met.
pub fn dispatch(token: Token, poller: &mut MioPollerThread) -> ThreadLoopContinuation {
    use SourceKindReady::{ReceiverDropWaker, Signals, Stdin, Unknown};
    match SourceKindReady::from_token(token) {
        Stdin => consume_stdin_input(poller),
        Signals => consume_pending_signals(poller),
        ReceiverDropWaker => handle_receiver_drop_waker(poller),
        Unknown => handle_unknown(token),
    }
}

#[must_use]
pub fn handle_unknown(token: Token) -> ThreadLoopContinuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::warn!(
            message = "mio_poller thread: unknown token",
            token = ?token
        );
    });
    ThreadLoopContinuation::Continue
}

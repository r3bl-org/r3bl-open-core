// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event dispatching for the [`mio`] poller event loop.

use super::{super::channel_types::PollerEvent, MioPollWorker,
            handler_receiver_drop::handle_receiver_drop_waker_with_tx,
            handler_signals::consume_pending_signals_with_tx,
            handler_stdin::consume_stdin_input_with_tx, sources::SourceKindReady};
use crate::{Continuation, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use mio::Token;
use tokio::sync::broadcast::Sender;

/// Dispatches to the appropriate handler based on the [`Token`], using explicit `tx`.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `tx` as a parameter to `poll_once()`.
///
/// [`RRTWorker`]: crate::core::resilient_reactor_thread::RRTWorker
pub fn dispatch_with_tx(
    token: Token,
    worker: &mut MioPollWorker,
    tx: &Sender<PollerEvent>,
) -> Continuation {
    use SourceKindReady::{ReceiverDropWaker, Signals, Stdin, Unknown};
    match SourceKindReady::from_token(token) {
        Stdin => consume_stdin_input_with_tx(worker, tx),
        Signals => consume_pending_signals_with_tx(worker, tx),
        ReceiverDropWaker => handle_receiver_drop_waker_with_tx(tx),
        Unknown => handle_unknown(token),
    }
}

#[must_use]
pub fn handle_unknown(token: Token) -> Continuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::warn!(
            message = "mio_poller thread: unknown token",
            token = ?token
        );
    });
    Continuation::Continue
}

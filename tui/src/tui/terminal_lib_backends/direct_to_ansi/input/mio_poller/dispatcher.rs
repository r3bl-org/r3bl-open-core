// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event dispatching for the [`mio`] poller event loop.

use super::{MioPollWorker,
            handler_signals::consume_pending_signals_with_sender,
            handler_software_interrupt::handle_software_interrupt_with_sender,
            handler_stdin::consume_stdin_input_with_sender, sources::SourceKindReady,
            super::channel_types::PollerEvent};
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_MIO_POLLER};
use mio::Token;
use tokio::sync::broadcast::Sender;

/// Dispatches to the appropriate handler based on the [`Token`], using explicit
/// `sender` parameter.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `sender` as a parameter to
/// [`block_until_ready_then_dispatch()`].
///
/// [`block_until_ready_then_dispatch()`]: crate::RRTWorker::block_until_ready_then_dispatch
/// [`RRTWorker`]: crate::RRTWorker
pub fn dispatch_with_sender(
    token: Token,
    worker: &mut MioPollWorker,
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    use SourceKindReady::{Signals, SoftwareInterrupt, Stdin, Unknown};
    match SourceKindReady::from_token(token) {
        Stdin => consume_stdin_input_with_sender(worker, sender),
        Signals => consume_pending_signals_with_sender(worker, sender),
        SoftwareInterrupt => handle_software_interrupt_with_sender(sender),
        Unknown => handle_unknown(token),
    }
}

#[must_use]
pub fn handle_unknown(token: Token) -> Continuation {
    DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
        tracing::warn!(
            message = "mio_poller thread: unknown token",
            token = ?token
        );
    });
    Continuation::Continue
}

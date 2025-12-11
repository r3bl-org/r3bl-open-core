// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event dispatching for the [`mio`] poller event loop.

use super::{handler_signals::consume_pending_signals,
            handler_stdin::consume_stdin_input, poller::MioPoller,
            sources::SourceKindReady};
use crate::tui::{DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                 terminal_lib_backends::direct_to_ansi::input::types::ThreadLoopContinuation};
use mio::Token;

/// Dispatches to the appropriate handler for the given source kind.
///
/// This centralizes the token → handler mapping, making it easier to add new
/// sources—just add a variant and its match arm here.
///
/// # Arguments
///
/// - `source_kind`: Which source kind became ready ([`SourceKindReady`]).
/// - `poller`: The [`MioPoller`] containing the state for handlers.
/// - `token`: The original [`Token`] for diagnostic logging on unknown tokens.
///
/// # Returns
///
/// - [`ThreadLoopContinuation::Continue`]: Event handled, continue polling.
/// - [`ThreadLoopContinuation::Return`]: Exit condition met.
pub fn dispatch(
    source_kind: SourceKindReady,
    poller: &mut MioPoller,
    token: Token,
) -> ThreadLoopContinuation {
    match source_kind {
        SourceKindReady::Stdin => consume_stdin_input(poller),
        SourceKindReady::Signals => consume_pending_signals(poller),
        SourceKindReady::Unknown => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::warn!(
                    message = "mio-poller-thread: unknown token",
                    token = ?token
                );
            });
            ThreadLoopContinuation::Continue
        }
    }
}

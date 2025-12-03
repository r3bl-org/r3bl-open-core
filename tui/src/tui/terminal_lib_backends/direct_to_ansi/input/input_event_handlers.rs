// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event handlers for stdin channel receives and `SIGWINCH` signals.
//!
//! These functions process the results of async I/O operations and convert them
//! into [`WaitAction`] control flow decisions.

#[cfg(unix)]
use super::types::WaitAction;
use crate::{InputEvent, core::term::get_size, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};

#[cfg(unix)]
use super::singleton::DirectToAnsiInputResource;
#[cfg(unix)]
use super::stdin_reader_thread::StdinReadResult;

/// Handles the result of receiving from the stdin channel.
///
/// This function processes [`StdinReadResult`] from the dedicated stdin reader thread.
/// The dedicated thread architecture ensures true cancel safety in [`tokio::select!`],
/// unlike [`tokio::io::stdin()`] which uses a blocking thread pool.
///
/// [`tokio::io::stdin()`]: tokio::io::stdin
/// [`tokio::select!`]: tokio::select
#[cfg(unix)]
pub fn handle_stdin_channel_result(
    core: &mut DirectToAnsiInputResource,
    result: Option<StdinReadResult>,
) -> WaitAction {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "direct-to-ansi: stdin channel received",
            result = ?result
        );
    });

    match result {
        Some(StdinReadResult::Data(data)) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "direct-to-ansi: stdin data received",
                    bytes_read = data.len()
                );
            });
            core.parse_buffer.append(&data);
            WaitAction::Continue
        }
        Some(StdinReadResult::Eof) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "direct-to-ansi: stdin EOF");
            });
            WaitAction::Shutdown
        }
        Some(StdinReadResult::Error(kind)) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "direct-to-ansi: stdin error", error_kind = ?kind);
            });
            WaitAction::Shutdown
        }
        None => {
            // Channel closed - stdin reader thread exited.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "direct-to-ansi: stdin channel closed");
            });
            WaitAction::Shutdown
        }
    }
}

/// Handles the result of a `SIGWINCH` signal.
#[cfg(unix)]
pub fn handle_sigwinch_result(sigwinch_result: Option<()>) -> WaitAction {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "direct-to-ansi: SIGWINCH branch selected",
            result = ?sigwinch_result
        );
    });

    match sigwinch_result {
        Some(()) => {
            // Signal received successfully, query terminal size.
            if let Ok(size) = get_size() {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "direct-to-ansi: returning Resize",
                        size = ?size
                    );
                });
                return WaitAction::Emit(InputEvent::Resize(size));
            }
            // If size query failed, continue to next iteration.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "direct-to-ansi: get_size() failed, continuing"
                );
            });
            WaitAction::Continue
        }
        None => {
            // Signal stream closed - unexpected but shouldn't cause shutdown.
            tracing::warn!(
                message =
                    "direct-to-ansi: SIGWINCH receiver returned None (stream closed)"
            );
            WaitAction::Continue
        }
    }
}

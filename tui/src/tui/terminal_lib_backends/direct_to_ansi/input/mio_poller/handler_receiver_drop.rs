// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Handler for [`ReceiverDropWaker`] events (thread exit check).
//!
//! [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker

use super::super::channel_types::PollerEvent;
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use tokio::sync::broadcast::Sender;

/// Handles [`ReceiverDropWaker`] event using explicit `sender` - check if thread
/// should exit.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `sender` as a parameter.
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`RRTWorker`]: crate::core::resilient_reactor_thread::RRTWorker
/// [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker
#[must_use]
pub fn handle_receiver_drop_waker_with_sender(
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    let receiver_count = sender.receiver_count();

    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: receiver drop waker triggered",
            receiver_count
        );
    });

    // Check if we should self-terminate (no receivers left).
    if receiver_count == 0 {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: no receivers left, exiting");
        });
        return Continuation::Stop;
    }

    // Still have receivers - keep running.
    Continuation::Continue
}

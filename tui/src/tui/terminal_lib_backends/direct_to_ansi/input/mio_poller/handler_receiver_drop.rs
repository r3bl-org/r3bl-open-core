// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Handler for [`ReceiverDropWaker`] events (thread exit check).
//!
//! [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker

use super::super::channel_types::PollerEvent;
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_MIO_POLLER};
use tokio::sync::broadcast::Sender;

/// Handles [`ReceiverDropWaker`] event using explicit `sender` - check if thread
/// should exit.
///
/// This variant is used by [`MioPollWorker`] which implements the generic
/// [`RRTWorker`] trait and receives `sender` as a parameter.
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker
/// [`RRTWorker`]: crate::RRTWorker
#[must_use]
pub fn handle_receiver_drop_waker_with_sender(
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    let receiver_count = sender.receiver_count();

    DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: receiver drop waker triggered",
            receiver_count
        );
    });

    // BUG: fix-rrt-subscribe-race-condition.md
    // Check if we should self-terminate (no receivers left).
    // IMPORTANT: Do NOT call tracing here. The thread must exit as fast as possible
    // to avoid a race with subscribe() — if a new subscriber sees liveness=Running
    // while this thread is still exiting, it won't spawn a replacement thread,
    // leaving the new subscriber with no stdin reader. See:
    // task/fix-make-log-file-writing-multithreaded.md
    if receiver_count == 0 {
        return Continuation::Stop;
    }

    // Still have receivers - keep running.
    Continuation::Continue
}

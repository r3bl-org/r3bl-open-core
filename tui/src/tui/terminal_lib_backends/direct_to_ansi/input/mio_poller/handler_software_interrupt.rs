// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Handler for synthetic software interrupt events. See
//! [`handle_software_interrupt_with_sender()`] for details.

use super::super::channel_types::PollerEvent;
use crate::{Continuation, core::resilient_reactor_thread::RRTEvent,
            tui::DEBUG_TUI_SHOW_MIO_POLLER};
use tokio::sync::broadcast::Sender;

/// Handles a synthetic software interrupt event using the provided `sender`.
///
/// This handler is called when a [`SubscriberGuard`] is dropped, triggering a software
/// interrupt. The framework uses this to check if it should shut down (if no subscribers
/// remain).
///
/// This variant is used by [`MioPollWorker`] which implements the generic [`RRTWorker`]
/// trait and receives `sender` as a parameter.
///
/// [`MioPollWorker`]: super::MioPollWorker
/// [`RRTWorker`]: crate::RRTWorker
/// [`SubscriberGuard`]: crate::SubscriberGuard
#[must_use]
pub fn handle_software_interrupt_with_sender(
    sender: &Sender<RRTEvent<PollerEvent>>,
) -> Continuation {
    let receiver_count = sender.receiver_count();

    DEBUG_TUI_SHOW_MIO_POLLER.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: software interrupt triggered",
            receiver_count
        );
    });

    // Lifecycle decisions are centralized in run_worker_loop().
    Continuation::Continue
}

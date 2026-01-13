// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Handler for [`ReceiverDropWaker`] events (thread exit check).
//!
//! [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker

use super::{poller_thread_state::ShutdownDecision, poller_thread::MioPollerThread};
use crate::{Continuation, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};

/// Handles [`ReceiverDropWaker`] event — check if thread should exit.
///
/// Called when [`SubscriberGuard::drop()`] wakes the thread via [`mio::Waker::wake()`].
/// Checks if all receivers have been dropped (i.e., [`receiver_count()`] `== 0`).
///
/// This function is the **exit check** in the thread lifecycle protocol. It handles
/// the inherent race condition where a new subscriber can appear between the wake
/// signal and this check. See [`PollerThreadState`] for comprehensive
/// documentation:
///
/// - [The Inherent Race Condition] — why we check instead of exiting blindly
/// - [What Happens If We Exit Blindly] — the zombie device scenario
/// - [Why Thread Reuse Is Safe] — resource safety table
/// - [Related Tests] — integration tests that validate this behavior
///
/// # Returns
///
/// - [`Continuation::Continue`]: Still have receivers, keep running.
/// - [`Continuation::Stop`]: No receivers left, thread should exit.
///
/// [Related Tests]: super::PollerThreadState#related-tests
/// [The Inherent Race Condition]: super::PollerThreadState#the-inherent-race-condition
/// [What Happens If We Exit Blindly]: super::PollerThreadState#what-happens-if-we-exit-blindly
/// [Why Thread Reuse Is Safe]: super::PollerThreadState#why-thread-reuse-is-safe
/// [`SubscriberGuard::drop()`]: crate::direct_to_ansi::input::input_device_impl::subscriber::SubscriberGuard#impl-Drop-for-SubscriberGuard
/// [`PollerThreadState`]: super::PollerThreadState
/// [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
pub fn handle_receiver_drop_waker(poller: &mut MioPollerThread) -> Continuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: receiver drop waker triggered",
            receiver_count = poller.thread_state.broadcast_tx.receiver_count()
        );
    });

    // Check if we should self-terminate (no receivers left).
    if poller.thread_state.should_self_terminate() == ShutdownDecision::ShutdownNow {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: no receivers left, exiting");
        });
        return Continuation::Stop;
    }

    // Still have receivers - keep running.
    Continuation::Continue
}

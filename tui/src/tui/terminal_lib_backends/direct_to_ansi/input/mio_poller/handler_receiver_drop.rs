// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Handler for [`ReceiverDropWaker`] events (thread exit check).
//!
//! [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker

use super::{poller_thread::MioPollerThread, poller_thread_lifecycle_state::ShutdownDecision};
use crate::{Continuation, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};

/// Handles [`ReceiverDropWaker`] event — check if thread should exit.
///
/// Called when [`PollerSubscriptionHandle::drop()`] wakes the thread via
/// [`mio::Waker::wake()`]. Checks if all receivers have been dropped (i.e.,
/// [`receiver_count()`] `== 0`).
///
/// This function is the **exit check** in the thread lifecycle protocol. It handles
/// the inherent race condition where a new subscriber can appear between the wake
/// signal and this check. See [`PollerThreadLifecycleState`] for comprehensive
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
/// [Related Tests]: super::PollerThreadLifecycleState#related-tests
/// [The Inherent Race Condition]: super::PollerThreadLifecycleState#the-inherent-race-condition
/// [What Happens If We Exit Blindly]: super::PollerThreadLifecycleState#what-happens-if-we-exit-blindly
/// [Why Thread Reuse Is Safe]: super::PollerThreadLifecycleState#why-thread-reuse-is-safe
/// [`PollerSubscriptionHandle::drop()`]: crate::direct_to_ansi::input::input_device::PollerSubscriptionHandle
/// [`PollerThreadLifecycleState`]: super::PollerThreadLifecycleState
/// [`ReceiverDropWaker`]: super::sources::SourceKindReady::ReceiverDropWaker
/// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
pub fn handle_receiver_drop_waker(poller: &mut MioPollerThread) -> Continuation {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "mio-poller-thread: receiver drop waker triggered",
            receiver_count = poller.state.tx_poller_event.receiver_count()
        );
    });

    // Check if we should self-terminate (no receivers left).
    if poller.state.should_self_terminate() == ShutdownDecision::ShutdownNow {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "mio-poller-thread: no receivers left, exiting");
        });
        return Continuation::Stop;
    }

    // Still have receivers - keep running.
    Continuation::Continue
}

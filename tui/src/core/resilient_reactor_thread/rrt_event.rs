// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Two-tier event model: [`RRTEvent`] and [`ShutdownReason`].

/// Wrapper enum that separates domain events from framework infrastructure events.
/// Subscribers receive this type from the broadcast channel and must handle both tiers.
///
/// See [self-healing restart details] for how this event model integrates with the
/// restart lifecycle.
///
/// Events have two producers with clean separation:
///
/// | Producer                   | Event                | Example                                                               |
/// | :------------------------- | :------------------- | :-------------------------------------------------------------------- |
/// | [`RRTWorker`] (domain)     | [`Worker(E)`]        | `PollerEvent::Stdin(StdinEvent::Eof)`                                 |
/// | Framework (infrastructure) | [`Shutdown(reason)`] | [`ShutdownReason::RestartPolicyExhausted`], [`ShutdownReason::Panic`] |
///
/// Your [`RRTWorker`] trait implementation never sends [`Shutdown`]. The framework never
/// sends domain events. Each tier owns its own signals.
///
/// - [`Worker(E)`]: Your domain event, produced by your [`RRTWorker`] trait
///   implementation.
/// - [`Shutdown(reason)`]: The framework is shutting down the thread. Subscribers should
///   take corrective action (e.g., try subscribing again later, or propagate the shutdown
///   to the application).
///
/// [`RRTWorker`]: super::RRTWorker
/// [`Shutdown(reason)`]: Self::Shutdown
/// [`Shutdown`]: Self::Shutdown
/// [`Worker(E)`]: Self::Worker
/// [self-healing restart details]: super#self-healing-restart-details
#[derive(Debug, Clone)]
pub enum RRTEvent<E> {
    /// Domain event produced by your [`RRTWorker`] trait implementation.
    ///
    /// [`RRTWorker`]: super::RRTWorker
    Worker(E),
    /// The framework is shutting down the thread. Subscribers should take
    /// corrective action.
    Shutdown(ShutdownReason),
}

/// Converts a domain event into an [`RRTEvent::Worker`] for sending through the channel.
impl<E> From<E> for RRTEvent<E> {
    fn from(event: E) -> Self { Self::Worker(event) }
}

/// Reason the framework is shutting down the dedicated thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownReason {
    /// Your [`RRTWorker`] trait implementation returned [`Continuation::Restart`] more
    /// times than the [`RestartPolicy`] allows.
    ///
    /// [`Continuation::Restart`]: crate::Continuation::Restart
    /// [`RRTWorker`]: super::RRTWorker
    /// [`RestartPolicy`]: super::RestartPolicy
    RestartPolicyExhausted {
        /// Total restart attempts made before giving up.
        attempts: u8,
    },

    /// Your [`RRTWorker`] trait implementation panicked inside [`poll_once()`]. The
    /// framework caught the panic (via [`catch_unwind`]) and is notifying subscribers so
    /// they can take corrective action (e.g., call [`subscribe()`] to relaunch a fresh
    /// thread).
    ///
    /// No restart is attempted after a panic - the thread exits after sending this event.
    /// Unlike [`RestartPolicyExhausted`], which indicates transient resource issues, a
    /// panic signals a logic bug that self-healing cannot fix.
    ///
    /// [`RRTWorker`]: super::RRTWorker
    /// [`RestartPolicyExhausted`]: Self::RestartPolicyExhausted
    /// [`catch_unwind`]: std::panic::catch_unwind
    /// [`poll_once()`]: super::RRTWorker::poll_once
    /// [`subscribe()`]: super::RRT::subscribe
    Panic,
}

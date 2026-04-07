// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Non-clonable interrupt handle wrapper for the Resilient Reactor Thread (RRT)
//! pattern. See [`InterruptHandle`] for details.

use super::RRTSoftwareInterrupt;

/// Non-clonable, non-copyable wrapper around a user-supplied [`RRTSoftwareInterrupt`]
/// implementation.
///
/// # Why This Wrapper Exists
///
/// This newtype exists to make the [zombie interrupt bug] structurally impossible. The
/// framework-internal contract is:
///
/// 1. The framework wraps the user's freshly-created [`RRTSoftwareInterrupt`] in an
///    [`InterruptHandle`] immediately after
///    [`RRTWorker::create_and_register_os_sources()`] returns it.
/// 2. The wrapped handle is then moved into the [`ThreadState::Running`] variant and
///    lives there for the entire generation of the dedicated thread.
/// 3. Because [`InterruptHandle`] does **not** implement [`Clone`] (or [`Copy`]), nothing
///    outside the [`ThreadLifecycleMonitor`] can ever obtain a standalone copy of the
///    interrupt handle. Subscribers that need to interrupt the dedicated thread must go
///    through [`ThreadLifecycleMonitor::interrupt_if_running()`], which acquires the
///    state lock and reads the **current** interrupt handle on every call.
/// 4. When the framework relaunches the thread (`Restarting → Running`), the old
///    [`InterruptHandle`] is dropped along with the old [`Running`] variant, and a new
///    one is constructed for the new generation. There is never a "leaked" handle that
///    points at a dead [`mio::Poll`].
///
/// # Make Illegal States Unrepresentable
///
/// This is the type-system enforcement that replaces the old design's "Waker slot"
/// (`Arc<Mutex<Option<W::Waker>>>`) indirection. The old design relied on convention:
/// subscribers were *expected* to hold a "Waker slot reader" (a clone of the slot), not a
/// clone of the waker itself. A future maintainer who optimized by "capturing the waker
/// at subscribe time" would have silently reintroduced the bug. Under the
/// [`InterruptHandle`] model, that optimization is a compile error - the interrupt handle
/// simply cannot be cloned.
///
/// # See Also
///
/// - [`ThreadLifecycleMonitor::interrupt_if_running()`]'s [Relationship to
///   `InterruptHandle`] section explains how this type and that method together form a
///   closed loop that the type system enforces. [`InterruptHandle`] is the *structural*
///   half of the guarantee; `interrupt_if_running()` is the *API* half. Neither alone is
///   enough.
///
/// [`mio::Poll`]: mio::Poll
/// [`RRTWorker::create_and_register_os_sources()`]:
///     super::RRTWorker::create_and_register_os_sources
/// [`Running`]: super::ThreadState::Running
/// [`ThreadLifecycleMonitor::interrupt_if_running()`]:
///     super::ThreadLifecycleMonitor::interrupt_if_running
/// [`ThreadLifecycleMonitor`]: super::ThreadLifecycleMonitor
/// [`ThreadState::Running`]: super::ThreadState::Running
/// [Relationship to `InterruptHandle`]:
///     super::ThreadLifecycleMonitor::interrupt_if_running
/// [zombie interrupt bug]: super#the-zombie-interrupt-bug
#[derive(Debug)]
pub struct InterruptHandle<K: RRTSoftwareInterrupt>(K);

impl<K: RRTSoftwareInterrupt> InterruptHandle<K> {
    /// Wraps a freshly-created interrupt handle. Called by the framework only - the
    /// `pub(super)` visibility prevents user code from constructing one out of
    /// thin air or smuggling a cloned interrupt handle in through the back door.
    pub(super) fn new(interrupt: K) -> Self { Self(interrupt) }

    /// Wakes the dedicated thread by calling the underlying
    /// [`RRTSoftwareInterrupt::trigger_software_interrupt()`].
    ///
    /// [`RRTSoftwareInterrupt::trigger_software_interrupt()`]:
    ///     super::RRTSoftwareInterrupt::trigger_software_interrupt
    pub fn trigger_software_interrupt(&self) { self.0.trigger_software_interrupt(); }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread monitor implementation for the Resilient Reactor Thread (RRT) pattern. See
//! [`ThreadLifecycleMonitor`] for details.
use super::{RRTWorker, ThreadState};
use crate::core::common::Monitor;
use std::sync::{LockResult, MutexGuard, atomic::AtomicU8};

/// # Thread Lifecycle Monitor
///
/// This struct wraps a generic [`Monitor`] to manage the lifecycle of a dedicated thread.
/// It uses the typestate pattern and a state machine to eliminate race conditions by
/// making them unrepresentable in the type system.
///
/// The internal [`Monitor`] provides the underlying synchronization (mutual exclusion and
/// signaling). See its documentation for architectural details on the [`monitor pattern`]
/// and lock discipline.
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for details.
///
/// [`Monitor`]: crate::core::common::Monitor
/// [`PoisonError`]: std::sync::PoisonError
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
#[derive(Debug)]
pub struct ThreadLifecycleMonitor<W: RRTWorker> {
    /// The definitive lifecycle state of the dedicated thread. By combining the
    /// typestate pattern and a state machine, this robust mechanism eliminates race
    /// conditions by making them unrepresentable in the type system.
    pub(super) monitor: Monitor<ThreadState<W>>,

    /// Per-thread-generation counter. Incremented each time a new thread is spawned.
    ///
    /// This counter is shared between the [`RRT`] singleton and all
    /// [`SubscriberGuard`]s. No [`Mutex`] needed - atomic operations are sufficient
    /// for a single counter.
    ///
    /// [`Condvar`]: std::sync::Condvar
    /// [`Mutex`]: std::sync::Mutex
    /// [`RRT`]: crate::resilient_reactor_thread::RRT
    /// [`SubscriberGuard`]: crate::resilient_reactor_thread::SubscriberGuard
    pub(super) thread_generation: AtomicU8,
}

impl<W: RRTWorker> ThreadLifecycleMonitor<W> {
    /// Creates a new [`ThreadLifecycleMonitor`] with the given initial state.
    pub const fn new(state: ThreadState<W>) -> Self {
        Self {
            monitor: Monitor::new(state),
            thread_generation: AtomicU8::new(0),
        }
    }

    /// Locks the internal state mutex.
    ///
    /// # Panics
    ///
    /// Panics if the internal state mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn lock<'this>(&'this self) -> MutexGuard<'this, ThreadState<W>> {
        self.monitor.lock()
    }

    /// This is a **poison-safe** alternative to [`Self::lock()`] specifically designed
    /// for **cleanup paths** (like `Drop` implementations).
    ///
    /// # Poison Safety
    ///
    /// In a TUI application, the highest risk is a **Double Panic Abort**. This happens
    /// if a panic occurs while another panic is already being processed (e.g., inside a
    /// `drop()` call). If you use [`Self::lock()`] in a cleanup path and the mutex is
    /// poisoned, it will trigger a second panic, causing the Rust runtime to **Abort**
    /// the process immediately. This prevents any further cleanup and **bricks the user's
    /// terminal** (leaving it in raw mode with no echo and a hidden cursor).
    ///
    /// Use this method when **restoring the user's terminal is more important than the
    /// integrity of the internal state**. This is the **Resilience over Integrity**
    /// philosophy. It returns a raw [`LockResult`], allowing you to use a pattern like:
    /// `lock_raw().unwrap_or_else(|e| e.into_inner())` to retrieve the (possibly dirty)
    /// state and proceed with terminal restoration rather than aborting the process.
    ///
    /// # Errors
    ///
    /// Returns a [`PoisonError`] if the internal mutex is poisoned. This allows you to
    /// handle poisoning deliberately (e.g., via [`into_inner()`]) to ensure terminal
    /// restoration can proceed even during a panic.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`into_inner()`]: std::sync::PoisonError::into_inner
    /// [`PoisonError`]: std::sync::PoisonError
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn lock_raw<'this>(&'this self) -> LockResult<MutexGuard<'this, ThreadState<W>>> {
        self.monitor.lock_raw()
    }

    /// Blocks the current thread until the dedicated thread reaches a stable state
    /// ([`Running`] or [`Stopped`]).
    ///
    /// This uses the monitor's [`wait_until()`] method to efficiently sleep while the
    /// state is transient ([`Starting`], [`Stopping`], or [`Restarting`]).
    ///
    /// # Panics
    ///
    /// Panics if the internal state mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Running`]: ThreadState::Running
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    /// [`wait_until()`]: Self::wait_until
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn block_until_stable_state_reached<'this>(
        &'this self,
    ) -> MutexGuard<'this, ThreadState<W>> {
        let state_guard = self.lock();
        self.wait_until(state_guard, |state| !state.is_transient())
    }

    /// Blocks the current thread on the internal condition variable, releasing the
    /// provided [`MutexGuard`]. The lock is re-acquired before this method returns.
    ///
    /// # Panics
    ///
    /// Panics if the internal state mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn wait<'this>(
        &'this self,
        guard: MutexGuard<'this, ThreadState<W>>,
    ) -> MutexGuard<'this, ThreadState<W>> {
        self.monitor.wait(guard)
    }

    /// Blocks the current thread on the internal condition variable until the
    /// provided `condition` is met, releasing the provided [`MutexGuard`]. The lock
    /// is re-acquired before this method returns.
    ///
    /// # Panics
    ///
    /// Panics if the internal state mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
    /// in the crate root documentation for details.
    ///
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn wait_until<'this, F>(
        &'this self,
        guard: MutexGuard<'this, ThreadState<W>>,
        condition: F,
    ) -> MutexGuard<'this, ThreadState<W>>
    where
        F: FnMut(&mut ThreadState<W>) -> bool,
    {
        self.monitor.wait_until(guard, condition)
    }

    /// Interrupts the dedicated thread if it is currently in the [`Running`] state. No-op
    /// for any other state.
    ///
    /// # The Only Interrupt Path
    ///
    /// This is the **only** way for code outside the framework to trigger the dedicated
    /// thread's software interrupt. Subscribers ([`SubscriberGuard`] /
    /// [`InterruptOnDrop`]) hold an [`Arc<ThreadLifecycleMonitor<W>>`] and call this
    /// method from their [`Drop`] impl. They have no other access to the interrupt handle
    /// because [`InterruptHandle`] does not implement [`Clone`] - the only handle to the
    /// interrupt lives inside the [`Running`] variant of [`ThreadState`], protected by
    /// the [`lock()`] lock.
    ///
    /// # Behavior Per State
    ///
    /// This method's behavior depends on the current [`ThreadState`] variant. None of the
    /// no-op cases below are coincidences - each falls out of the type-system invariant
    /// that the [`InterruptHandle`] is reachable *only* inside [`Running`]:
    ///
    /// | State          | Behavior                                                             | Why                                                                                                         |
    /// | :------------- | :------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------- |
    /// | [`Running`]    | Interrupts via [`InterruptHandle::trigger_software_interrupt()`]     | Interrupts blocking I/O so thread can re-check shutdown conditions                                          |
    /// | [`Starting`]   | No-op                                                                | Interrupt handle doesn't exist yet (thread being spawned, OS resources still being created)                 |
    /// | [`Stopping`]   | No-op                                                                | Interrupt handle was consumed by transition out of [`Running`]; thread is in teardown, not blocked on I/O   |
    /// | [`Restarting`] | No-op                                                                | Same as [`Stopping`] - interrupt handle consumed; thread is recycling OS resources                          |
    /// | [`Stopped`]    | No-op                                                                | No thread exists; next [`try_subscribe()`] spawns a fresh one                                               |
    ///
    /// # Lock Discipline
    ///
    /// Holds the [`lock()`] lock for the duration of the
    /// [`InterruptHandle::trigger_software_interrupt()`] call. This is required for
    /// correctness - the [`Running`] variant must remain valid while we read the
    /// interrupt handle out of it - and would be required even if
    /// `trigger_software_interrupt()` were an expensive blocking call. As it happens,
    /// [`mio::Waker::wake()`] is a fast lockless [`syscall`], so the conventional "don't
    /// hold locks across function calls" concern doesn't apply either.
    ///
    /// # Relationship to `InterruptHandle`
    ///
    /// [`interrupt_if_running()`] and [`super::InterruptHandle`] are two halves of the
    /// "[`zombie interrupt bug`] cannot happen" guarantee. **Neither alone is enough** -
    /// together they form a closed loop that the type system enforces.
    ///
    /// - [`super::InterruptHandle`] is the *structural* half: it does not implement
    ///   [`Clone`] or [`Copy`], so the underlying interrupt handle can never escape the
    ///   [`Running`] variant as an owned copy. The compiler enforces this.
    /// - [`interrupt_if_running()`] is the *API* half: it is the only public path that
    ///   reads the interrupt handle from the variant, and it does so under the [`lock()`]
    ///   lock on every call - so subscribers always invoke the *current* generation's
    ///   interrupt handle, never a stale one.
    ///
    /// Take either constraint away and the bug comes back:
    ///
    /// - **Without [`InterruptHandle`]'s non-[`Clone`] constraint:** the underlying
    ///   `W::Interrupt` type is typically [`Arc`]-backed and trivially [`Clone`] (e.g.,
    ///   [`mio::Waker`]). A subscriber could lock the state, match on [`Running`], clone
    ///   the inner `W::Interrupt`, release the lock, and keep the clone across thread
    ///   relaunches - calling a stale interrupt handle that points at a now-dead
    ///   [`mio::Poll`].
    /// - **Without [`interrupt_if_running()`] as the sole API:** every caller would need
    ///   to reimplement the lock-then-match pattern correctly. Convention is fragile;
    ///   centralizing the interrupt path in one method makes the freshness guarantee
    ///   mechanical.
    ///
    /// See [`InterruptHandle`]'s [`Why This Wrapper Exists`] section for the
    /// corresponding view from the type's side, and the [`zombie interrupt bug`]
    /// historical context for the full rationale of why this matters.
    ///
    /// # Poison Safety
    ///
    /// This method is called during [`SubscriberGuard`]'s [`Drop`] implementation (via
    /// [`InterruptOnDrop`]). To ensure that terminal restoration is never blocked by a
    /// "Double Panic Abort", this method is **poison-safe**. It uses
    /// [`Self::lock_raw()`] to handle poisoning without panicking, allowing the
    /// interrupt signal to be attempted even if the state machine is in a dirty state.
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`Arc<ThreadLifecycleMonitor<W>>`]: std::sync::Arc
    /// [`Arc`]: std::sync::Arc
    /// [`Condvar`]: std::sync::Condvar
    /// [`Drop`]: std::ops::Drop
    /// [`interrupt_if_running()`]: ThreadLifecycleMonitor::interrupt_if_running
    /// [`InterruptHandle::trigger_software_interrupt()`]:
    ///     super::InterruptHandle::trigger_software_interrupt()
    /// [`InterruptHandle`]: super::InterruptHandle
    /// [`InterruptOnDrop`]: crate::resilient_reactor_thread::InterruptOnDrop
    /// [`lock()`]: Self::lock
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker::wake()`]: mio::Waker::wake
    /// [`mio::Waker`]: mio::Waker
    /// [`Mutex`]: std::sync::Mutex
    /// [`Restarting`]: ThreadState::Restarting
    /// [`Running`]: ThreadState::Running
    /// [`Starting`]: ThreadState::Starting
    /// [`Stopped`]: ThreadState::Stopped
    /// [`Stopping`]: ThreadState::Stopping
    /// [`SubscriberGuard`]: crate::resilient_reactor_thread::SubscriberGuard
    /// [`syscall`]: https://man7.org/linux/man-pages/man2/syscalls.2.html
    /// [`try_subscribe()`]: crate::resilient_reactor_thread::RRT::try_subscribe
    /// [`Why This Wrapper Exists`]: super::InterruptHandle#why-this-wrapper-exists
    /// [`zombie interrupt bug`]: super#the-zombie-interrupt-bug
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub fn interrupt_if_running(&self) {
        // Poison-safe lock: attempt interrupt even if state is dirty.
        let state_guard = match self.lock_raw() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message =
                        "interrupt_if_running: state lock poisoned, attempting interrupt anyway",
                    error = ?poisoned
                );
                poisoned.into_inner()
            }
        };

        if let ThreadState::Running(interrupt_handle) = &*state_guard {
            // Perform the interrupt while holding the lock to ensure the Running state
            // remains valid.
            interrupt_handle.trigger_software_interrupt();
        }

        drop(state_guard);
    }

    /// Wakes up all threads that are currently blocked on this monitor's condition
    /// variable.
    pub fn notify_all(&self) { self.monitor.notify_all(); }

    /// Wakes up one thread that is currently blocked on this monitor's condition
    /// variable.
    pub fn notify_one(&self) { self.monitor.notify_one(); }

    /// Updates the current [`ThreadState`] and logs the transition (if
    /// [`DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD`] is enabled).
    ///
    /// # Ownership & Chain of Custody
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait_until()`]. This enforces a clear "chain of custody" for the lock:
    ///
    /// 1. The caller must already hold the lock (providing the [`MutexGuard`]). They can
    ///    get this lock by calling [`lock()`].
    /// 2. The caller "gives" the guard to this method to perform the update.
    /// 3. This method "gives back" the guard so the caller can continue using it.
    ///
    /// This structurally prevents logic errors where a caller might try to use a stale
    /// guard after a transition.
    ///
    /// # Examples
    ///
    /// **Flavor 1: Reassign to the same variable**
    /// ```no_run
    /// use r3bl_tui::{RRTWorker, ThreadLifecycleMonitor, ThreadState, ok};
    /// fn test<W: RRTWorker>(monitor: ThreadLifecycleMonitor<W>) -> miette::Result<()> {
    ///     let mut guard = monitor.lock();
    ///     guard = monitor.set_state(guard, ThreadState::Starting);
    ///     ok!()
    /// }
    /// ```
    ///
    /// **Flavor 2: Create new variables per move**
    /// ```no_run
    /// use r3bl_tui::{RRTWorker, ThreadLifecycleMonitor, ThreadState, ok};
    /// fn test<W: RRTWorker>(monitor: ThreadLifecycleMonitor<W>) -> miette::Result<()> {
    ///     let guard = monitor.lock();
    ///     let guard_2 = monitor.set_state(guard, ThreadState::Starting);
    ///     ok!()
    /// }
    /// ```
    ///
    /// [`DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD`]:
    ///     crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD
    /// [`lock()`]: Self::lock()
    /// [`wait_until()`]: Self::wait_until()
    #[allow(clippy::elidable_lifetime_names)]
    pub fn set_state<'this>(
        &'this self,
        guard: MutexGuard<'this, ThreadState<W>>,
        new_state: ThreadState<W>,
    ) -> MutexGuard<'this, ThreadState<W>> {
        crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "RRT: State transition.",
                old_state = ?*guard,
                new_state = ?new_state
            );
        });

        // Delegate to the underlying monitor.
        self.monitor.set_state(guard, new_state)
    }

    /// Updates the current [`ThreadState`] by applying the provided closure `f`.
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait_until()`]. This enforces a clear "chain of custody" for the lock.
    ///
    /// [`wait_until()`]: Self::wait_until()
    #[allow(clippy::elidable_lifetime_names)]
    pub fn update_state<'this, F>(
        &'this self,
        guard: MutexGuard<'this, ThreadState<W>>,
        fun: F,
    ) -> MutexGuard<'this, ThreadState<W>>
    where
        F: FnOnce(&mut ThreadState<W>),
    {
        crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "RRT: State mutation via closure.",
                current_state = ?*guard
            );
        });

        // Delegate to the underlying monitor.
        self.monitor.update_state(guard, fun)
    }

    /// Reads the current [`ThreadState`] by applying the provided closure `f`.
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait_until()`]. This enforces a clear "chain of custody" for the lock.
    ///
    /// [`wait_until()`]: Self::wait_until()
    #[allow(clippy::elidable_lifetime_names)]
    pub fn read_state<'this, F, R>(
        &'this self,
        guard: MutexGuard<'this, ThreadState<W>>,
        f: F,
    ) -> (R, MutexGuard<'this, ThreadState<W>>)
    where
        F: FnOnce(&ThreadState<W>) -> R,
    {
        crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "RRT: State read via closure.",
                current_state = ?*guard
            );
        });

        // Delegate to the underlying monitor.
        self.monitor.read_state(guard, f)
    }
}

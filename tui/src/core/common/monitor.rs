// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Generic Monitor implementation providing a [`Mutex`] and a [`Condvar`] for thread
//! synchronization. See [`Monitor`] for details.

use std::sync::{Condvar, LockResult, Mutex, MutexGuard};

/// A synchronization primitive that combines a [`Mutex`] (protecting the state of generic
/// type `S`) and a [`Condvar`], effectively implementing the [monitor pattern].
///
/// # Monitor Pattern
///
/// The monitor pattern is a synchronization construct that allows threads to have both
/// mutual exclusion and the ability to wait (block) for a certain condition to become
/// true. Monitors also provide a mechanism for signaling other threads that their
/// condition may have been met.
///
/// In this implementation, the [`Monitor`] provides:
/// 1. **Mutual Exclusion**: Guaranteed by the internal [`Mutex`]. Only one thread can
///    access the protected state (of generic type `S`) at a time.
/// 2. **Waiting and Signaling**: Guaranteed by the internal [`Condvar`]. Threads can
///    efficiently block while waiting for a condition and be unblocked by other threads
///    calling [`notify_all()`] or [`notify_one()`].
///
/// # Chain of Custody (Friction-as-a-Feature)
///
/// Unlike [`ScopedMutex`], which uses closures to hide the [`MutexGuard`], [`Monitor`]
/// **must** return the guard to the caller. This is because [`Condvar::wait()`] needs to
/// consume and then re-acquire the lock.
///
/// To prevent deadlocks, [`Monitor`] employs a **Chain of Custody** pattern:
/// 1. The guard is returned by value (e.g., via [`lock()`]).
/// 2. The caller becomes the temporary "custodian" of the lock.
/// 3. The caller must eventually "return" the guard to the [`Monitor`] by dropping it or
///    passing it back into another method (like [`wait()`]).
///
/// This protocol ensures that while the lock is exposed, its ownership is always tracked
/// and explicit, reducing the risk of accidental double-locking or long-lived guards in
/// complex state machines.
///
/// # Comparison with [`ScopedMutex`] (Scoped Access)
///
/// | Feature             | [`ScopedMutex`] (Scoped Access) | [`Monitor`] (Chain of Custody)    |
/// | :------------------ | :------------------------------ | :-------------------------------- |
/// | **Primary Goal**    | Simple shared state             | Complex state machines            |
/// | **Synchronization** | [`Mutex`] only                  | [`Mutex`] + [`Condvar`]           |
/// | **Access Pattern**  | Closure-based                   | Guard-based (move-by-value)       |
/// | **Deadlock Safety** | Structural (via closures)       | Protocol-based (chain of custody) |
/// | **Use Case**        | Global settings, single stats   | RRT engine, thread coordination   |
///
/// - **Use [`ScopedMutex`]**: When you just need to safely read or write a shared value
///   and want to ensure the lock is never held longer than necessary. See the [Scoped
///   Access] section in [`ScopedMutex`] for details.
/// - **Use [`Monitor`]**: When you need to coordinate between threads (using [`wait()`]
///   or [`notify_all()`]).
///
/// # Why Use a Monitor?
///
/// Monitors are ideal for managing shared state machines or complex synchronization
/// requirements (like the [Resilient Reactor Thread] pattern). They eliminate race
/// conditions by centralizing state access and notification logic in one place.
///
/// # What is a condition variable?
///
/// Generally speaking, a condition variable (often abbreviated as `condvar`) is a
/// synchronization primitive used in multithreaded programming. It allows one or more
/// threads to **block** on the `condvar` by calling `wait()` until a specific condition
/// becomes true, at which point another thread can unblock them by calling `notify()` or
/// `notify_all()` on the `condvar`.
///
/// Condition variables solve the problem of **busy waiting**. Busy waiting occurs when a
/// thread constantly polls a condition in a tight loop to check if a shared resource is
/// ready. While synchronization primitives like [`spinlocks`] intentionally use busy
/// waiting for extremely short delays to avoid the overhead of context switching, using
/// this polling approach for unpredictable or long-term waits wastes valuable CPU cycles.
/// Instead of burning CPU time repeatedly asking "Are we there yet?", a thread can use a
/// condition variable to efficiently block its execution until it receives a
/// notification.
///
/// # Comparison with Java: `synchronized`, `wait()`, and `notify()`
///
/// In traditional Java, every object has a built-in mutex and a single condition
/// variable. You access this hidden [monitor pattern] using the `synchronized` keyword
/// alongside the `wait()` and `notify()` methods. If you forget to hold the lock before
/// calling `wait()`, Java catches the mistake at runtime and throws an
/// `IllegalMonitorStateException`.
///
/// # How does Rust's [`Condvar`] work?
///
/// Rust takes a more explicit approach by separating these concepts into distinct
/// [`Mutex<S>`] and [`Condvar`] types. This explicit separation closely mirrors modern
/// Java's `ReentrantLock` and `Condition` classes.
///
/// However, Rust goes a step further to protect against concurrency bugs. In Rust, a
/// [`Mutex`] physically owns the data it protects. When you lock the mutex, you receive a
/// [`MutexGuard`] token. The [`Condvar::wait()`] method requires you to pass in this
/// exact token. By leveraging the ownership system, the compiler forces you to hold the
/// lock before waiting—transforming Java's runtime crashes into strict compile-time
/// guarantees.
///
/// # Signaling
///
/// To unblock threads blocked in [`wait()`], you must call [`notify_all()`] or
/// [`notify_one()`]. For correctness, you should mutate the shared state *before* calling
/// these methods. If you notify before mutating the state, the waiting thread is
/// unblocked, checks the state, sees it hasn't changed, and goes right back to blocked
/// and the notification was wasted.
///
/// The notification itself can be sent while holding the state lock or immediately after
/// releasing it; unblocking is independent of the mutex.
///
/// # More information
///
/// - Watch this [video] for more information on monitors and [`Condvar`].
/// - [`Condvar` code example]
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for details.
///
/// [`Condvar::wait()`]: std::sync::Condvar::wait
/// [`Condvar` code example]:
///     https://github.com/nazmulidris/rust-scratch/tree/main/condvar
/// [`Condvar`]: std::sync::Condvar
/// [`lock()`]: Self::lock
/// [`LockResult`]: std::sync::LockResult
/// [`Mutex<S>`]: std::sync::Mutex
/// [`Mutex`]: std::sync::Mutex
/// [`MutexGuard`]: std::sync::MutexGuard
/// [`notify_all()`]: Self::notify_all
/// [`notify_one()`]: Self::notify_one
/// [`PoisonError`]: std::sync::PoisonError
/// [`ScopedMutex`]: crate::core::common::ScopedMutex
/// [`spinlocks`]: https://en.wikipedia.org/wiki/Spinlock
/// [`wait()`]: Self::wait
/// [monitor pattern]: https://en.wikipedia.org/wiki/Monitor_(synchronization)
/// [Resilient Reactor Thread]: crate::core::resilient_reactor_thread
/// [Scoped Access]: crate::core::common::ScopedMutex#scoped-access-friction-as-a-feature
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
/// [video]: https://www.youtube.com/watch?v=HvCptpU5r_4
#[derive(Debug)]
pub struct Monitor<S> {
    /// The shared state protected by this monitor.
    ///
    /// This state must be locked before any read, write, or [`Condvar`] wait or notify
    /// operations.
    ///
    /// [`Condvar`]: std::sync::Condvar
    pub(super) state: Mutex<S>,

    /// A synchronization primitive that allows threads to voluntarily block their
    /// execution and wait for a notification that the [`state`] has changed. See
    /// the [explanation] for more details.
    ///
    /// [`state`]: field@Self::state
    /// [explanation]: Self#what-is-a-condition-variable
    pub(super) condvar: Condvar,
}

impl<S> Monitor<S> {
    /// Creates a new [`Monitor`] with the given initial state.
    pub const fn new(state: S) -> Self {
        Self {
            state: Mutex::new(state),
            condvar: Condvar::new(),
        }
    }

    /// Locks the internal mutex guarding the state.
    ///
    /// # Lock Discipline
    ///
    /// This method returns a [`MutexGuard`] that provides exclusive access to the
    /// protected state. This state lock is held until the guard is dropped or passed to
    /// [`wait()`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding
    /// the lock).
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`wait()`]: Self::wait
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    #[allow(clippy::elidable_lifetime_names)]
    pub fn lock<'this>(&'this self) -> MutexGuard<'this, S> {
        #[allow(clippy::unwrap_used, reason = "Mutex poisoning is unrecoverable")]
        self.state.lock().unwrap()
    }

    /// Locks the internal mutex guarding the state, returning the raw
    /// [`std::sync::LockResult`].
    ///
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
    /// philosophy. It returns a raw [`std::sync::LockResult`], allowing you to use a
    /// pattern like: `lock_raw().unwrap_or_else(|e| e.into_inner())` to retrieve the
    /// (possibly dirty) state and proceed with terminal restoration rather than aborting
    /// the process.
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
    pub fn lock_raw<'this>(&'this self) -> LockResult<MutexGuard<'this, S>> {
        self.state.lock()
    }

    /// Blocks the current thread on the internal condition variable, releasing the
    /// provided [`MutexGuard`]. The lock is re-acquired before this method returns.
    ///
    /// See the [Signaling] section for important details on how to correctly unblock
    /// threads.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`notify_all()`]: Self::notify_all
    /// [`notify_one()`]: Self::notify_one
    /// [`wait()`]: Self::wait
    /// [Signaling]: Self#signaling
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub fn wait<'this>(&'this self, guard: MutexGuard<'this, S>) -> MutexGuard<'this, S> {
        #[allow(clippy::unwrap_used, reason = "Mutex poisoning is unrecoverable")]
        self.condvar.wait(guard).unwrap()
    }

    /// Blocks the current thread on the internal condition variable until the
    /// provided `condition` is met, releasing the provided [`MutexGuard`]. The lock
    /// is re-acquired before this method returns.
    ///
    /// See the [Signaling] section for important details on how to correctly unblock
    /// threads.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    ///
    /// # Poison Safety
    ///
    /// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in
    /// the crate root documentation for details.
    ///
    /// [`notify_all()`]: Self::notify_all
    /// [`notify_one()`]: Self::notify_one
    /// [`wait()`]: Self::wait
    /// [Signaling]: Self#signaling
    /// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
    ///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
    pub fn wait_until<'this, F>(
        &'this self,
        guard: MutexGuard<'this, S>,
        mut condition: F,
    ) -> MutexGuard<'this, S>
    where
        F: FnMut(&mut S) -> bool,
    {
        #[allow(clippy::unwrap_used, reason = "Mutex poisoning is unrecoverable")]
        self.condvar
            .wait_while(guard, |state| !condition(state))
            .unwrap()
    }

    /// Unblocks one thread that is currently blocked on this monitor's condition
    /// variable (via a call to [`wait()`]).
    ///
    /// See the [Signaling] section for important details on how to correctly unblock
    /// threads.
    ///
    /// [`wait()`]: Self::wait
    /// [Signaling]: Self#signaling
    pub fn notify_one(&self) { self.condvar.notify_one(); }

    /// Unblocks all threads that are currently blocked on this monitor's condition
    /// variable (via a call to [`wait()`]).
    ///
    /// See the [Signaling] section for important details on how to correctly unblock
    /// threads.
    ///
    /// [`wait()`]: Self::wait
    /// [Signaling]: Self#signaling
    pub fn notify_all(&self) { self.condvar.notify_all(); }

    /// Replaces the current state with `new_state`.
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait()`]. This enforces a clear "chain of custody" for the lock:
    ///
    /// 1. The caller must already hold the lock (providing the [`MutexGuard`]). They can
    ///    get this lock by calling [`lock()`].
    /// 2. The caller "gives" the guard to this method to perform the update.
    /// 3. This method "gives back" the guard so the caller can continue using it.
    ///
    /// This structurally prevents logic errors where a caller might try to use a stale
    /// guard after a transition.
    ///
    /// [`lock()`]: Self::lock
    /// [`wait()`]: Self::wait
    pub fn set_state<'this>(
        &'this self,
        mut guard: MutexGuard<'this, S>,
        new_state: S,
    ) -> MutexGuard<'this, S> {
        *guard = new_state;
        guard
    }

    /// Updates the current state by applying the provided closure `f`.
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait()`]. This enforces a clear "chain of custody" for the lock:
    ///
    /// 1. The caller must already hold the lock (providing the [`MutexGuard`]). They can
    ///    get this lock by calling [`lock()`].
    /// 2. The caller "gives" the guard to this method to perform the update.
    /// 3. This method "gives back" the guard so the caller can continue using it.
    ///
    /// This structurally prevents logic errors where a caller might try to use a stale
    /// guard after a transition.
    ///
    /// [`lock()`]: Self::lock
    /// [`wait()`]: Self::wait
    pub fn update_state<'this, F>(
        &'this self,
        mut guard: MutexGuard<'this, S>,
        fun: F,
    ) -> MutexGuard<'this, S>
    where
        F: FnOnce(&mut S),
    {
        fun(&mut guard);
        guard
    }

    /// Reads the current state by applying the provided closure `f`.
    ///
    /// This method follows the same "take by value, return by value" pattern as
    /// [`wait()`]. This enforces a clear "chain of custody" for the lock:
    ///
    /// 1. The caller must already hold the lock (providing the [`MutexGuard`]). They can
    ///    get this lock by calling [`lock()`].
    /// 2. The caller "gives" the guard to this method to perform the update.
    /// 3. This method "gives back" the guard so the caller can continue using it.
    ///
    /// This structurally prevents logic errors where a caller might try to use a stale
    /// guard after a transition.
    ///
    /// [`lock()`]: Self::lock
    /// [`wait()`]: Self::wait
    pub fn read_state<'this, F, R>(
        &'this self,
        guard: MutexGuard<'this, S>,
        fun: F,
    ) -> (R, MutexGuard<'this, S>)
    where
        F: FnOnce(&S) -> R,
    {
        (fun(&guard), guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_monitor_new() {
        let monitor = Monitor::new(42);
        let state_guard = monitor.lock();
        assert_eq!(*state_guard, 42);
    }

    #[test]
    fn test_monitor_mutex_delegation_sanity_check() {
        let monitor = Monitor::new(42);

        // Mutate the state.
        {
            let mut state_guard = monitor.lock();
            *state_guard = 100;
        }

        // Read the state.
        {
            let new_state_guard = monitor.lock();
            assert_eq!(*new_state_guard, 100);
        }
    }

    #[test]
    fn test_monitor_poisoning() {
        let monitor = Arc::new(Monitor::new(0));
        let monitor_clone = Arc::clone(&monitor);

        let _unused = std::thread::spawn(move || {
            let _guard = monitor_clone.lock();
            panic!("Intentional panic to poison the mutex");
        })
        .join();

        assert!(monitor.lock_raw().is_err());
    }

    #[test]
    fn test_monitor_poisoning_recovery() {
        let monitor = Arc::new(Monitor::new(0));
        let monitor_clone = Arc::clone(&monitor);

        // 1. Poison the mutex.
        let _unused = std::thread::spawn(move || {
            let mut guard = monitor_clone.lock();
            *guard = 42;
            panic!("Intentional panic to poison the mutex");
        })
        .join();

        // 2. Verify it is poisoned.
        assert!(monitor.lock_raw().is_err());

        // 3. Recover using into_inner().
        {
            let guard = match monitor.lock_raw() {
                Ok(_) => panic!("Should be poisoned"),
                Err(poisoned) => poisoned.into_inner(),
            };
            assert_eq!(*guard, 42);
            // We can still mutate the dirty state.
            drop(monitor.set_state(guard, 100));
        }

        // 4. Verify it is still poisoned but contains new value.
        {
            let guard = monitor.lock_raw().unwrap_err().into_inner();
            assert_eq!(*guard, 100);
        }
    }

    #[derive(Default)]
    struct State {
        pub barrier_ready_count: usize,
        pub value: usize,
    }

    #[test]
    fn test_monitor_wait_and_notify_one() {
        let monitor = Arc::new(Monitor::new(State::default()));
        let monitor_clone = Arc::clone(&monitor);

        // Spawn Thread A.
        let join_handle = std::thread::spawn(move || {
            let mut state_guard = monitor_clone.lock();

            // Handshake - Signal that we are about to wait.
            state_guard.barrier_ready_count += 1;
            monitor_clone.notify_all();

            // Wait for the actual condition.
            let new_state_guard =
                monitor_clone.wait_until(state_guard, |state| state.value != 0);
            let return_value = new_state_guard.value;
            drop(new_state_guard);
            return_value
        });

        // Block main thread until Thread A is ready.
        {
            // Handshake - wait for Thread A to be ready.
            let state_guard = monitor.lock();
            let new_state_guard =
                monitor.wait_until(state_guard, |state| state.barrier_ready_count == 1);
            drop(new_state_guard);
        }

        // Update state and notify blocked Thread A.
        {
            let mut state_guard = monitor.lock();
            state_guard.value = 1;
            monitor.notify_one();
        }

        let result = join_handle.join().unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_monitor_wait_and_notify_all() {
        const NUM_THREADS: usize = 3;
        let monitor = Arc::new(Monitor::new(State::default()));
        let mut join_handles = Vec::new();

        // Spawn multiple threads.
        for i in 1..=NUM_THREADS {
            let monitor_clone = Arc::clone(&monitor);
            let handle = std::thread::spawn(move || {
                let mut state_guard = monitor_clone.lock();
                // Handshake - Signal that this thread is about to wait.
                state_guard.barrier_ready_count += 1;
                monitor_clone.notify_all();

                // Wait for the actual condition.
                let state_guard =
                    monitor_clone.wait_until(state_guard, |state| state.value > 0);
                let return_value = state_guard.value + i;
                drop(state_guard);
                return_value
            });
            join_handles.push(handle);
        }

        // Handshake - Wait for ALL background threads to be ready.
        {
            let state_guard = monitor.lock();
            let state_guard = monitor.wait_until(state_guard, |state| {
                state.barrier_ready_count == NUM_THREADS
            });
            drop(state_guard);
        }

        // Update state and notify ALL blocked threads.
        {
            let mut state_guard = monitor.lock();
            state_guard.value = 100;
            monitor.notify_all();
        }

        // Collect results from all threads.
        let mut results = Vec::new();
        for handle in join_handles {
            results.push(handle.join().unwrap());
        }

        // Verify that every thread woke up and processed the notification.
        assert_eq!(results.len(), NUM_THREADS);
        for (index, result) in results.into_iter().enumerate() {
            let thread_spawn_order = index + 1;
            assert_eq!(result, 100 + thread_spawn_order);
        }
    }
}

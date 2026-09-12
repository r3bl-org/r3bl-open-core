// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{LineState, ModalGuardToken, OutputDevice, PaintMode, SafeLineState,
            disable_raw_mode};
use std::io::Write;

/// This struct acts as a "Traffic Cop" to prevent lock-inversion deadlocks between the
/// keystroke event loop task ([`Readline::readline`]) that handles user input and the
/// line control task ([`process_line_control_signal`]) that handles concurrent output,
/// pause/resume transitions, and flush signals from [`SharedWriter`]s.
///
/// A task is a [`tokio`] green thread that is backed by a thread pool of OS worker
/// threads (in a multi-threaded runtime) or a single OS thread (in a current-thread
/// runtime). In a multi-threaded runtime (the default for `#[tokio::main]`), each task
/// can be scheduled to run on a different OS thread at the same time. This is inherited
/// from the app that uses this crate (it is not specified here).
///
/// > An application's background tasks (like logging output or spinner animations) do not
/// > cause deadlocks; they simply emit messages to the [`SharedWriter`], which are then
/// > consumed by the [channel processing task]. See [Coffman Conditions][1] for more
/// > details on deadlock conditions.
///
/// # The Deadlock Story (which this struct avoids)
///
/// To understand why this manager exists, let's look at the two tasks that need to share
/// the terminal, and why they inherently collide.
///
/// > This synchronization is required even in a single-threaded runtime: [`tokio::spawn`]
/// > mandates `Send + Sync` shared state, and multi-step terminal rendering requires
/// > mutual exclusion to prevent display corruption (rendering is not atomic).
///
/// Specifically, consider the two shared resources that require mutex synchronization:
///
/// 1. **line state lock 🔒** ([`SafeLineState`]): Mutex protecting the prompt buffer,
///    cursor coordinates, and line editing state.
/// 2. **terminal output lock 🔒** ([`OutputDevice`]): Mutex protecting the shared raw
///    [`stdout`] write stream.
///
/// Both tasks need to acquire **both locks** simultaneously to perform their work, so
/// that they don't clobber the terminal output.
///
/// 1. **The Keystroke Task** (running [`Readline::readline`]): When the user presses a
///    key, it needs the **line state lock 🔒** to mutate the prompt buffer, and the
///    **terminal output lock 🔒** to immediately draw the updated prompt to the screen.
/// 2. **The Line Control Task** (running [`process_line_control_signal`]): When any
///    concurrent task sends text to the [`SharedWriter`], this task needs the **line
///    state lock 🔒** to read and clear the current prompt, and the **terminal output
///    lock 🔒** to print the incoming text and redraw the prompt below it.
///
/// Because both tasks must hold both locks to complete their operations, they are
/// vulnerable to lock inversions. Let's say we acquired these locks in opposite orders:
/// - The Keystroke Task acquired [`OutputDevice`] first, then [`SafeLineState`] second.
/// - The Line Control Task acquired [`SafeLineState`] first, then [`OutputDevice`]
///   second.
///
/// If a user typed a keystroke at the exact microsecond a concurrent task emitted text:
/// - Keystroke task locks [`OutputDevice`].
/// - Line control task locks [`SafeLineState`].
/// - Keystroke task waits for [`SafeLineState`] (Deadlock ☠️).
/// - Line control task waits for [`OutputDevice`] (Deadlock ☠️).
///
/// This is why we need this [`ReadlineLockManager`] to enforce a strict locking order. So
/// that it is impossible to acquire both locks in the wrong order.
///
/// # The Solution: Level 1 and Level 2 Locks
///
/// To mathematically prevent this "Hold and Wait" deadlock, all locks must be acquired in
/// a strict hierarchical order:
/// - **Level 1:** [`SafeLineState`]
/// - **Level 2:** [`OutputDevice`]
///
/// [`ReadlineLockManager`] enforces this by keeping the underlying [`Arc<Mutex>`] fields
/// completely private. If a developer needs both locks, they *must* use [`lock_both()`],
/// which natively guarantees the correct Level 1 -> Level 2 acquisition order.
///
/// # WARNING: Single-Lock Closures
///
/// If you only need [`SafeLineState`] (e.g., to query cursor position or buffer contents
/// without redrawing), you can use [`lock_line_state()`]. However, this MUST be a leaf
/// operation:
/// - Never acquire [`OutputDevice`] inside [`lock_line_state()`] (use [`lock_both()`]
///   instead).
///
/// Background components that only need [`OutputDevice`] (like [`Spinner`] or
/// [`SharedWriter`]) hold their own cloned [`OutputDevice`] handle and write to it
/// directly (they do not go through [`ReadlineLockManager`]).
///
/// # Why use [`RAII`] instead of Typestate pattern?
///
/// We chose an [`RAII`] Guard pattern ([`ModalTerminalGuard`]) combined with this dual
/// lock hierarchy rather than a strict typestate pattern by value, due to the extreme
/// complexities of async terminal programming. A pure typestate approach struggles with
/// the following realities of an **async** [`readline`]:
///
/// 1. **Multi-Axis Concurrency (Async I/O):** Background tasks (like [`Spinner`] and
///    [`SharedWriter`]) can request terminal pauses independently of the main thread.
///    Typestate by value assumes a linear, single-owner progression of states. It cannot
///    ergonomically model independent, concurrent state transitions without exploding
///    into dozens of state combination structs.
/// 2. **Ergonomics in Async Loops:** Transitioning typestates inside a continuous
///    `read_line()` `loop {}` requires constant `self` re-assignment, which is fragile
///    when dealing with async `?` early returns.
/// 3. **Terminal State Corruption:** If an async typestate drops or yields incorrectly,
///    multiple threads might write [`ANSI`] escape codes concurrently. A central lock
///    manager prevents this more reliably.
/// 4. **Clean Shutdowns & Panics:** If the async executor panics halfway through,
///    [`RAII`] `Drop` guarantees the terminal is safely restored from raw mode back to
///    cooked mode, which typestates by value cannot natively guarantee on early returns.
///
/// # The Lifetime Tether Pattern
///
/// When [`ModalTerminalGuard`] pauses the terminal, it yields a `&mut OutputDevice`. This
/// borrow is primarily a structural convenience and a lifetime tether rather than a
/// strict memory lock. Because [`OutputDevice`] is cheaply cloneable, you *could*
/// technically clone it and write to the terminal concurrently, bypassing this guard and
/// causing screen corruption.
///
/// To truly prevent all concurrent writes at the memory level, modal functions like
/// [`choose()`] would need to hold a [`MutexGuard`] for the terminal. However, because
/// [`choose()`] is an `async` function that runs for a long time, holding a lock open
/// across `.await` points is a severe anti-pattern in async Rust (it causes deadlocks and
/// blocks other threads). Therefore, yielding a `&mut OutputDevice` tether is the best
/// architectural compromise we can make to safely bind the lifetime of the UI modal to
/// the pause state of the background [`Readline`] task.
///
/// [1]: https://en.wikipedia.org/wiki/Coffman_conditions
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`Arc<Mutex>`]: std::sync::Arc
/// [`choose()`]: crate::choose
/// [`lock_both()`]: Self::lock_both
/// [`lock_line_state()`]: Self::lock_line_state
/// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
/// [`MutexGuard`]: std::sync::MutexGuard
/// [`OutputDevice`]: crate::OutputDevice
/// [`process_line_control_signal`]: crate::process_line_control_signal
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`Readline::readline`]: crate::Readline::readline
/// [`readline`]: https://man7.org/linux/man-pages/man3/readline.3.html
/// [`ReadlineLockManager`]: Self
/// [`SafeLineState`]: crate::SafeLineState
/// [`SharedWriter`]: crate::SharedWriter
/// [`Spinner`]: crate::Spinner
/// [`stdout`]: std::io::stdout
/// [`tokio::spawn`]: tokio::spawn
/// [`tokio`]: tokio
/// [channel processing task]: super::line_control_task::process_line_control_signal
#[allow(missing_debug_implementations)]
pub struct ReadlineLockManager {
    /// Level 1 lock: prompt buffer, cursor coordinates, and line editor state.
    line_state_level_1: SafeLineState,

    /// Level 2 lock: shared raw terminal [`stdout`] write stream.
    ///
    /// [`stdout`]: std::io::stdout
    output_device_level_2: OutputDevice,
}

impl ReadlineLockManager {
    /// Creates a new manager to orchestrate safe lock acquisition for the given shared
    /// line state and output device. These arguments are [`Arc`] wrapped structs, so
    /// their ownership is NOT moved here.
    ///
    /// [`Arc`]: std::sync::Arc
    pub fn new(line_state: SafeLineState, output_device: OutputDevice) -> Self {
        Self {
            line_state_level_1: line_state,
            output_device_level_2: output_device,
        }
    }

    /// Safely acquires both locks in the strictly correct [Coffman hierarchy][1]:
    /// [`SafeLineState`] (Level 1) first, then [`OutputDevice`] (Level 2). See [struct
    /// docs] for more details.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Coffman_conditions
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`SafeLineState`]: crate::SafeLineState
    /// [struct docs]: Self
    pub fn lock_both<R>(
        &self,
        fn_once: impl FnOnce(&mut LineState, &mut dyn Write) -> R,
    ) -> R {
        self.line_state_level_1.write(|line_state| {
            self.output_device_level_2
                .write(|term| fn_once(line_state, term))
        })
    }

    /// Acquires only the [`SafeLineState`] lock.
    ///
    /// **WARNING:** This must be a leaf operation. Do not attempt to acquire
    /// [`OutputDevice`] inside this closure.
    ///
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`SafeLineState`]: crate::SafeLineState
    pub fn lock_line_state<R>(&self, fn_once: impl FnOnce(&mut LineState) -> R) -> R {
        self.line_state_level_1.write(fn_once)
    }

    /// Provides exclusive mutable access to the internal [`OutputDevice`].
    ///
    /// Used **ONLY** by [`ModalTerminalGuard::as_mut_tuple`] to provide `(&mut
    /// OutputDevice, &mut InputDevice)` to modal sub-applications (such as
    /// [`crate::choose()`]). The [`ModalGuardToken`] witness guarantees this method can't
    /// be called by anyone else.
    ///
    /// # Safety and Invariant Preservation
    ///
    /// Calling this method requires an exclusive `&mut self` borrow of
    /// [`ReadlineLockManager`] (which in turn requires an exclusive `&mut Readline`
    /// borrow) and a [`ModalGuardToken`] witness. This statically prevents concurrent
    /// access from [`Readline::readline()`].
    ///
    /// [`ModalGuardToken`]: crate::ModalGuardToken
    /// [`ModalTerminalGuard::as_mut_tuple`]: crate::ModalTerminalGuard::as_mut_tuple
    /// [`MutexGuard`]: std::sync::MutexGuard
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`Readline::readline()`]: crate::Readline::readline
    /// [`Readline`]: crate::Readline
    /// [`ReadlineLockManager`]: Self
    pub(crate) fn exclusive_output_device(
        &mut self,
        _token: ModalGuardToken,
    ) -> &mut OutputDevice {
        &mut self.output_device_level_2
    }

    /// Performs poison-safe emergency terminal restoration during [`Readline`] drop.
    ///
    /// Bypasses the lock ledger to avoid panicking on poisoned locks during stack
    /// unwinding. Acquires [`SafeLineState`] (Level 1) first, then [`OutputDevice`]
    /// (Level 2) second, strictly respecting the lock hierarchy.
    ///
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`Readline`]: crate::Readline
    /// [`SafeLineState`]: crate::SafeLineState
    pub(crate) fn poison_safe_terminal_restore_on_drop(&self) {
        self.line_state_level_1.lock_raw_poison_safe(|line_state| {
            self.output_device_level_2.lock_raw_poison_safe(|term| {
                // We don't care about the result of this operation.
                drop(line_state.exit(term));

                // We don't care about the result of this operation.
                // disable_raw_mode() is also poison-safe.
                if self.output_device_level_2.paint_mode != PaintMode::Mock {
                    drop(disable_raw_mode());
                }
            });
        });
    }

    /// Provides reference to the internal [`SafeLineState`].
    ///
    /// This method is only available for tests and documentation builds (e.g., simulating
    /// mutex poisoning in double-panic prevention tests).
    ///
    /// [`SafeLineState`]: crate::SafeLineState
    #[cfg(test)]
    pub(crate) fn line_state_for_testing(&self) -> &SafeLineState {
        &self.line_state_level_1
    }
}

// cspell:words Coffman typestates

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{LineState, ModalGuardToken, OutputDevice, PaintMode, SafeLineState,
            disable_raw_mode};
use std::io::Write;

/// This struct acts as a "Traffic Cop" to prevent lock-inversion deadlocks between the
/// main keystroke event loop ([`Readline::readline`]) and the background channel
/// processing task ([`process_line_control_signal`]). (Note: User-spawned tasks like
/// logging or spinners do not cause deadlocks directly; they simply emit messages to the
/// [`SharedWriter`], which are then consumed by the channel processing task). See
/// [Coffman Conditions][1] for more details on deadlock conditions.
///
/// ## The Deadlock Story
///
/// To understand why this manager exists, you have to understand the two tasks that need
/// to share the terminal, and why they inherently collide:
///
/// 1. **The Main Keystroke Task**: When the user presses a key, this task needs to mutate
///    the prompt buffer (stored in [`SafeLineState`]) and then immediately draw the
///    updated prompt to the screen (via [`OutputDevice`]).
/// 2. **The Line Control Task**: When a background thread (like a logger or spinner)
///    sends text to the [`SharedWriter`], this internal background task receives it and
///    must write it to the screen. But to prevent the text from irreversibly corrupting
///    the user's half-typed prompt, it must first *read* the current prompt (from
///    [`SafeLineState`]), clear the screen, print the text (via [`OutputDevice`]), and
///    then redraw the prompt below it.
///
/// Because both tasks need both locks to do their jobs, they are vulnerable to lock
/// inversions. Historically, they acquired these locks in opposite orders:
/// - The Keystroke Task locked [`OutputDevice`] first, then [`SafeLineState`] second.
/// - The Line Control Task locked [`SafeLineState`] first, then [`OutputDevice`] second.
///
/// If a user typed a keystroke at the exact microsecond a background thread emitted text:
/// - Keystroke task locks [`OutputDevice`].
/// - Line Control task locks [`SafeLineState`].
/// - Keystroke task waits for [`SafeLineState`] (Deadlock).
/// - Line Control task waits for [`OutputDevice`] (Deadlock).
///
/// ## The Solution: Level 1 and Level 2 Locks
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
/// ## WARNING: Single-Lock Closures
///
/// If you only need [`OutputDevice`] (e.g., for [`Spinner`]) or only [`SafeLineState`],
/// you can use [`lock_output_device()`] or [`lock_line_state()`]. However, these MUST be
/// leaf operations. You are strictly forbidden from dynamically capturing another lock
/// inside these closures.
///
/// [1]: https://en.wikipedia.org/wiki/Coffman_conditions
/// [`Arc<Mutex>`]: std::sync::Arc
/// [`lock_both()`]: Self::lock_both
/// [`lock_line_state()`]: Self::lock_line_state
/// [`lock_output_device()`]: Self::lock_output_device
/// [`OutputDevice`]: crate::OutputDevice
/// [`process_line_control_signal`]:
///     crate::manage_shared_writer_output::process_line_control_signal
/// [`Readline::readline`]: crate::Readline::readline
/// [`ReadlineLockManager`]: Self
/// [`SafeLineState`]: crate::SafeLineState
/// [`SharedWriter`]: crate::SharedWriter
/// [`Spinner`]: crate::Spinner
#[allow(missing_debug_implementations)]
pub struct ReadlineLockManager {
    line_state: SafeLineState,
    output_device: OutputDevice,
}

impl ReadlineLockManager {
    pub fn new(line_state: SafeLineState, output_device: OutputDevice) -> Self {
        Self {
            line_state,
            output_device,
        }
    }

    /// Safely acquires both locks in the strictly correct [Coffman hierarchy][1]:
    /// [`SafeLineState`] (Level 1) first, then [`OutputDevice`] (Level 2). See
    /// [struct docs] for more details.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Coffman_conditions
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`SafeLineState`]: crate::SafeLineState
    /// [struct docs]: Self
    pub fn lock_both<R>(&self, f: impl FnOnce(&mut LineState, &mut dyn Write) -> R) -> R {
        self.line_state
            .write(|line| self.output_device.write(|term| f(line, term)))
    }

    /// Acquires only the [`SafeLineState`] lock.
    ///
    /// **WARNING:** This must be a leaf operation. Do not attempt to acquire
    /// [`OutputDevice`] inside this closure.
    ///
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`SafeLineState`]: crate::SafeLineState
    pub fn lock_line_state<R>(&self, f: impl FnOnce(&mut LineState) -> R) -> R {
        self.line_state.write(f)
    }

    /// Acquires only the [`OutputDevice`] lock.
    ///
    /// **WARNING:** This must be a leaf operation. Do not attempt to acquire
    /// [`SafeLineState`] inside this closure.
    ///
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`SafeLineState`]: crate::SafeLineState
    pub fn lock_output_device<R>(&self, f: impl FnOnce(&mut dyn Write) -> R) -> R {
        self.output_device.write(|term| f(term))
    }

    /// Provides exclusive mutable access to the internal [`OutputDevice`].
    ///
    /// Used by [`ModalTerminalGuard`] to provide `(&mut OutputDevice, &mut InputDevice)`
    /// to modal sub-applications (such as [`crate::choose()`]).
    ///
    /// # Safety and Invariant Preservation
    ///
    /// Calling this method requires an exclusive `&mut self` borrow of
    /// [`ReadlineLockManager`] (which in turn requires an exclusive `&mut Readline`
    /// borrow) and a [`ModalGuardToken`] witness. This statically prevents concurrent
    /// access from [`Readline::readline()`].
    ///
    /// [`ModalGuardToken`]: crate::ModalGuardToken
    /// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
    /// [`OutputDevice`]: crate::OutputDevice
    /// [`Readline::readline()`]: crate::Readline::readline
    /// [`Readline`]: crate::Readline
    /// [`ReadlineLockManager`]: Self
    pub(crate) fn exclusive_output_device(
        &mut self,
        _token: ModalGuardToken,
    ) -> &mut OutputDevice {
        &mut self.output_device
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
        self.line_state.lock_raw_poison_safe(|line_state| {
            self.output_device.lock_raw_poison_safe(|term| {
                // We don't care about the result of this operation.
                drop(line_state.exit(term));

                // We don't care about the result of this operation.
                // disable_raw_mode() is also poison-safe.
                if self.output_device.paint_mode != PaintMode::Mock {
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
    pub(crate) fn line_state_for_testing(&self) -> &SafeLineState { &self.line_state }
}

// cspell:words Coffman

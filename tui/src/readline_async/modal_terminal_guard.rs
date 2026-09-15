// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InputDevice, LineStateControlSignal, OutputDevice, PauseStateTransition,
            Readline};

/// An [`RAII`] guard that grants exclusive access to the terminal devices
/// ([`OutputDevice`] and [`InputDevice`]) by temporarily suspending [`Readline`]
/// operations.
///
/// When a modal component (like [`crate::choose()`]) needs full control of the terminal,
/// it acquires a [`ModalTerminalGuard`]. While the guard is held:
///
/// 1. The [`Readline`]'s pause state is transitioned to [`PauseState::PausedByModal`] (or
///    [`PauseState::PausedByBoth`] if a spinner was already running).
/// 2. Any prompt currently rendered on screen is cleared.
/// 3. Background writes to [`crate::SharedWriter`] are safely buffered in the pause
///    buffer instead of being output to the screen, preventing modal display corruption.
/// 4. When the [`ModalTerminalGuard`] is dropped at the end of the modal session, the
///    pause state is transitioned back (to [`PauseState::NotPaused`] or
///    [`PauseState::PausedBySpinner`]). If the terminal is fully resumed, a
///    [`LineStateControlSignal::Flush`] signal is sent to flush all buffered background
///    messages and redraw the prompt.
///
/// For more details on the pause, resume, and modal architecture, see the [`Readline`]
/// documentation.
///
/// # Why We Use Lifetime Tether Instead of Strict Memory Lock
///
/// This guard acts primarily as a [**lifetime tether**][Lifetime Tether Pattern] and
/// pause coordinator rather than an absolute memory lock:
///
/// - **Why not a [`MutexGuard`]?** Holding a lock across long-running `.await` points
///   (such as [`choose()`]) is problematic in async Rust because it can cause deadlocks.
/// - **Limitation:** Because [`OutputDevice`] is cloneable, cloning it can technically
///   bypass this guard and write to the terminal concurrently, leading to display
///   corruption.
///
/// For the complete design rationale, see the [Lifetime Tether Pattern] documentation in
/// [`ReadlineLockManager`].
///
/// [`choose()`]: crate::choose
/// [`InputDevice`]: crate::InputDevice
/// [`LineStateControlSignal::Flush`]: crate::LineStateControlSignal::Flush
/// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
/// [`MutexGuard`]: std::sync::MutexGuard
/// [`OutputDevice`]: crate::OutputDevice
/// [`PauseState::NotPaused`]: crate::PauseState::NotPaused
/// [`PauseState::PausedByBoth`]: crate::PauseState::PausedByBoth
/// [`PauseState::PausedByModal`]: crate::PauseState::PausedByModal
/// [`PauseState::PausedBySpinner`]: crate::PauseState::PausedBySpinner
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`Readline`]: crate::Readline
/// [`ReadlineLockManager`]: crate::ReadlineLockManager
/// [Lifetime Tether Pattern]:
///     crate::ReadlineLockManager#the-lifetime-tether-pattern
#[allow(missing_debug_implementations)]
pub struct ModalTerminalGuard<'a> {
    readline: &'a mut Readline,
}

impl<'a> ModalTerminalGuard<'a> {
    /// Acquires exclusive control over the terminal I/O from [`Readline`].
    ///
    /// Transitions the [`PauseState`] to modal-paused, clears the prompt from the screen,
    /// and ensures no background line processing interferes with the modal session.
    ///
    /// [`PauseState`]: crate::PauseState
    pub fn acquire(readline: &'a mut Readline) -> Self {
        readline.lock_manager.lock_both(|line_state, term| {
            if line_state.pause_state.pause_modal() == PauseStateTransition::Paused {
                drop(line_state.clear_and_render_and_flush(term));
            }
        });
        Self { readline }
    }

    /// Provides mutable references to both the output device and input device as a tuple.
    ///
    /// Designed for seamless use with [`crate::choose()`]. To understand why this yields
    /// a `&mut OutputDevice` rather than a `MutexGuard`, see the [Lifetime Tether
    /// Pattern].
    ///
    /// [Lifetime Tether Pattern]: crate::ReadlineLockManager#the-lifetime-tether-pattern
    pub fn as_mut_tuple(&mut self) -> (&mut OutputDevice, &mut InputDevice) {
        (
            self.readline
                .lock_manager
                .exclusive_output_device(ModalGuardToken(())),
            &mut self.readline.input_device,
        )
    }
}

impl Drop for ModalTerminalGuard<'_> {
    fn drop(&mut self) {
        let is_resumed = self.readline.lock_manager.lock_line_state(|line_state| {
            line_state.pause_state.resume_modal() == PauseStateTransition::Resumed
        });

        if is_resumed && let Some(ref sender) = self.readline.line_control_sender {
            drop(sender.try_send(LineStateControlSignal::Flush));
        }
    }
}

// XMARK: Witness token usage to restrict access of a method w/out making it private

/// A witness token that proves a [`ModalTerminalGuard`] is held.
///
/// This token cannot be constructed outside this module (its single tuple field is
/// private). It is passed to `ReadlineLockManager::exclusive_output_device` to prove
/// that the terminal is suspended.
///
/// [`ModalTerminalGuard`]: crate::ModalTerminalGuard
#[derive(Debug)]
pub struct ModalGuardToken(());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChannelCapacity, OutputDeviceExt, PauseState, vp_height, vp_width};
    use smallvec::smallvec;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_modal_terminal_guard_lifecycle() {
        let (output_device, _) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(smallvec![]);
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = vp_width(80) + vp_height(24);

        let (mut readline, _) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .expect("conversion error");

        // Initial state is NotPaused.
        readline.lock_manager.lock_line_state(|line_state| {
            assert_eq!(line_state.pause_state, PauseState::NotPaused);
        });

        // Acquire guard: transitions to PausedByModal.
        {
            let mut guard = ModalTerminalGuard::acquire(&mut readline);
            guard.readline.lock_manager.lock_line_state(|line_state| {
                assert_eq!(line_state.pause_state, PauseState::PausedByModal);
            });
            let (_out, _in) = guard.as_mut_tuple();
        }

        // After dropping guard: transitions back to NotPaused.
        readline.lock_manager.lock_line_state(|line_state| {
            assert_eq!(line_state.pause_state, PauseState::NotPaused);
        });
    }

    #[tokio::test]
    async fn test_modal_terminal_guard_with_spinner() {
        let (output_device, _) = OutputDevice::new_mock();
        let input_device = InputDevice::new_mock(smallvec![]);
        let (shutdown_sender, _) = broadcast::channel::<()>(1);
        let test_size = vp_width(80) + vp_height(24);

        let (mut readline, _) = Readline::try_new(
            "> ".into(),
            output_device,
            input_device,
            shutdown_sender,
            ChannelCapacity::Minimal,
            test_size,
        )
        .expect("conversion error");

        // Simulate spinner pausing the readline.
        readline.lock_manager.lock_line_state(|line_state| {
            line_state.pause_state.pause_spinner();
            assert_eq!(line_state.pause_state, PauseState::PausedBySpinner);
        });

        // Acquire guard: transitions to PausedByBoth.
        {
            let guard = ModalTerminalGuard::acquire(&mut readline);
            guard.readline.lock_manager.lock_line_state(|line_state| {
                assert_eq!(line_state.pause_state, PauseState::PausedByBoth);
            });
        }

        // After dropping guard: transitions back to PausedBySpinner.
        readline.lock_manager.lock_line_state(|line_state| {
            assert_eq!(line_state.pause_state, PauseState::PausedBySpinner);
        });
    }
}

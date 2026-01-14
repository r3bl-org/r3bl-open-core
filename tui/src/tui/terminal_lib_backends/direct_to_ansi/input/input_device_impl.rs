// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize EINTR SIGWINCH kqueue epoll wakeup eventfd bcast
// cspell:words reinit

//! Implementation details for [`DirectToAnsiInputDevice`].
//!
//! This module uses the **Resilient Reactor Thread (RRT)** infrastructure:
//!
//! - **[`SINGLETON`]** (container): Static [`ThreadSafeGlobalState`], lives for process
//!   lifetime
//! - **[`ThreadState`]** (payload): Created when thread spawns, destroyed when it exits
//!
//! The container persists, but the payload comes and goes with the thread lifecycle.
//!
//! Module contents: [`global_input_resource`] (operations + [`SINGLETON`]).
//!
//! See [`DirectToAnsiInputDevice`] for the big picture (architecture, lifecycle, I/O
//! pipeline).
//!
//! [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice
//! [`SINGLETON`]: global_input_resource::SINGLETON
//! [`ThreadSafeGlobalState`]: crate::core::resilient_reactor_thread::ThreadSafeGlobalState
//! [`ThreadState`]: crate::core::resilient_reactor_thread::ThreadState
//! [`global_input_resource`]: mod@global_input_resource

use super::{channel_types::PollerEvent,
            mio_poller::{MioPollWaker, MioPollWorkerFactory}};
use crate::core::resilient_reactor_thread::{LivenessState, SubscriberGuard,
                                            ThreadSafeGlobalState};
use miette::Report;

/// Type alias for the input device's subscriber guard.
///
/// This is the RAII guard returned by [`allocate()`] and [`subscribe_to_existing()`].
/// Holding this guard keeps you subscribed to input events; dropping it triggers the
/// cleanup protocol that may cause the thread to exit.
///
/// [`allocate()`]: global_input_resource::allocate
/// [`subscribe_to_existing()`]: global_input_resource::subscribe_to_existing
pub type InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>;

/// Operations on the process-global input resource singleton.
///
/// This module encapsulates the global [`SINGLETON`] static and provides all operations
/// on it. The singleton is `pub` for doc links, but callers should use these functions
/// rather than accessing it directly.
///
/// See [`ThreadState`] for what the singleton holds when active.
///
/// [`SINGLETON`]: global_input_resource::SINGLETON
/// [`ThreadState`]: crate::core::resilient_reactor_thread::ThreadState
pub mod global_input_resource {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// **Static container** (lives for process lifetime) that holds a
    /// [`ThreadState`] **payload** (ephemeral, follows thread lifecycle). The payload is
    /// created when [`allocate()`] spawns a thread, and removed when the thread exits.
    ///
    /// Lifecycle states:
    /// - **Inert** (`None`) until [`allocate()`] spawns the poller thread
    /// - **Active** (`Some`) while thread is running
    /// - **Dormant** (`Some` with terminated liveness) when all [`SubscriberGuard`]s
    ///   drop and thread exits
    /// - **Reactivates** on next [`allocate()`] call (spawns fresh thread, replaces
    ///   payload)
    ///
    /// This is NOT "allocate once, lives forever" — supports full restart cycles.
    ///
    /// Use the functions in this module ([`allocate()`], [`is_thread_running()`], etc.)
    /// rather than accessing this directly.
    ///
    /// See [`ThreadState`] for what the payload contains when active.
    ///
    /// # Why `ThreadSafeGlobalState`?
    ///
    /// The RRT infrastructure handles all the complexity of:
    /// - Deferred initialization (syscalls can't be `const`)
    /// - Thread lifecycle management (spawn, exit, restart)
    /// - Race condition handling (fast-path thread reuse)
    /// - Waker coupling with Poll
    ///
    /// # Usage
    ///
    /// - Use [`allocate()`] to subscribe to input events & signals.
    /// - See [Architecture] for why global state is necessary.
    /// - See [`MioPollWorker`] for worker details.
    ///
    /// [Architecture]: super::super::DirectToAnsiInputDevice#architecture
    /// [`MioPollWorker`]: super::super::mio_poller::MioPollWorker
    /// [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
    /// [`ThreadState`]: crate::core::resilient_reactor_thread::ThreadState
    /// [`allocate()`]: allocate
    /// [`is_thread_running()`]: is_thread_running
    pub static SINGLETON: ThreadSafeGlobalState<MioPollWaker, PollerEvent> =
        ThreadSafeGlobalState::new();

    /// Subscribe your async consumer to the global input resource, in order to receive
    /// input events.
    ///
    /// The global `static` singleton [`SINGLETON`] contains one
    /// [`broadcast::Sender`]. This channel acts as a bridge between the sync
    /// [`MioPollWorker`] and the many async consumers. We don't need to capture the
    /// broadcast channel itself in the singleton, only the sender, since it is trivial
    /// to create new receivers from it.
    ///
    /// # Returns
    ///
    /// A new [`SubscriberGuard`] that independently receives all input events and
    /// resize signals.
    ///
    /// # Multiple Async Consumers
    ///
    /// Each caller gets their own receiver via [`broadcast::Sender::subscribe()`]. Here
    /// are examples of callers:
    /// - TUI app that receives all input events.
    /// - Logger receives all input events (independently).
    /// - Debug recorder receives all input events (independently).
    ///
    /// # Thread Spawning
    ///
    /// On first call, this spawns the [`mio`] poller thread via the RRT infrastructure
    /// which uses [`mio::Poll`] to wait on both [`stdin`] data and [`SIGWINCH`] signals.
    /// See the [Thread Lifecycle] section in [`ThreadState`] for details on thread
    /// lifetime and exit conditions.
    ///
    /// # Two Allocation Paths
    ///
    /// | Condition                | Path          | What Happens                            |
    /// | ------------------------ | ------------- | --------------------------------------- |
    /// | `liveness == Running`    | **Fast path** | Reuse existing thread + [`ThreadState`] |
    /// | `liveness == Terminated` | **Slow path** | Replace all, spawn new thread           |
    ///
    /// ## Fast Path (Thread Reuse)
    ///
    /// If the thread is still running, we **reuse everything**:
    /// - Same [`ThreadState`] (same broadcast channel, same liveness tracker)
    /// - Same [`mio::Poll`] + [`Waker`] (still registered, still valid)
    /// - Same thread (continues serving the new subscriber)
    ///
    /// This handles the [race condition] where a new subscriber appears before the
    /// thread checks [`receiver_count()`]. See [`ThreadState`] for complete
    /// documentation on [The Inherent Race Condition] and [Why Thread Reuse Is Safe].
    ///
    /// ## Slow Path (Thread Restart)
    ///
    /// If the thread has terminated, the existing [`ThreadState`] is **orphaned**
    /// — no thread is feeding events into its broadcast channel. We must **replace
    /// everything**:
    /// - New [`ThreadState`] (fresh broadcast channel + liveness tracker + waker)
    /// - New [`mio::Poll`] (old one was dropped with old thread)
    /// - New thread (spawned to serve the new subscriber)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// 1. Worker setup fails (OS resource creation failed).
    /// 2. Thread spawning fails (system thread limits).
    /// 3. The [`SINGLETON`] mutex is poisoned.
    ///
    /// [The Inherent Race Condition]: crate::core::resilient_reactor_thread::ThreadState#the-inherent-race-condition
    /// [Thread Lifecycle]: crate::core::resilient_reactor_thread#thread-lifecycle
    /// [Why Thread Reuse Is Safe]: crate::core::resilient_reactor_thread::ThreadState#why-thread-reuse-is-safe
    /// [`MioPollWorker`]: super::super::mio_poller::MioPollWorker
    /// [`SIGWINCH`]: signal_hook::consts::SIGWINCH
    /// [`SINGLETON`]: SINGLETON
    /// [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
    /// [`ThreadState`]: crate::core::resilient_reactor_thread::ThreadState
    /// [`Waker`]: mio::Waker
    /// [`broadcast::Sender::subscribe()`]: tokio::sync::broadcast::Sender::subscribe
    /// [`broadcast::Sender`]: tokio::sync::broadcast::Sender
    /// [`mio::Poll`]: mio::Poll
    /// [`receiver_count()`]: tokio::sync::broadcast::Sender::receiver_count
    /// [`stdin`]: std::io::stdin
    /// [race condition]: crate::core::resilient_reactor_thread::ThreadState#the-inherent-race-condition
    pub fn allocate() -> Result<InputSubscriberGuard, Report> {
        SINGLETON.allocate::<MioPollWorkerFactory>()
    }

    /// Checks if the [`mio_poller`] thread is currently running.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// - [`LivenessState::Running`] if the thread is running.
    /// - [`LivenessState::Terminated`] if [`SINGLETON`] is uninitialized or the thread
    ///   has exited.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on how threads
    /// spawn and exit.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[must_use]
    pub fn is_thread_running() -> LivenessState { SINGLETON.is_thread_running() }

    /// Queries how many receivers are subscribed to the input broadcast channel.
    ///
    /// This is useful for testing thread lifecycle behavior and debugging.
    ///
    /// # Returns
    ///
    /// The number of active receivers, or `0` if [`SINGLETON`] is uninitialized.
    ///
    /// The [`mio_poller`] thread exits gracefully when this count reaches `0` (all
    /// receivers dropped). See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for
    /// details.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[must_use]
    pub fn get_receiver_count() -> usize { SINGLETON.get_receiver_count() }

    /// Returns the current thread generation number.
    ///
    /// Each time a new [`mio_poller`] thread is spawned, the generation increments. This
    /// allows tests to verify whether a thread was reused or relaunched:
    ///
    /// - **Same generation**: Thread was reused (device B subscribed before thread
    ///   exited).
    /// - **Different generation**: Thread was relaunched (a new thread was spawned).
    ///
    /// # Returns
    ///
    /// The current generation number, or `0` if [`SINGLETON`] is uninitialized.
    ///
    /// See [Device Lifecycle] in [`DirectToAnsiInputDevice`] for details on thread
    /// spawn/exit/relaunch.
    ///
    /// [Device Lifecycle]: crate::DirectToAnsiInputDevice#device-lifecycle
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: crate::direct_to_ansi::input::mio_poller
    #[must_use]
    pub fn get_thread_generation() -> u8 { SINGLETON.get_thread_generation() }

    /// Subscribe to input events from an existing thread.
    ///
    /// This is a lightweight operation that creates a new subscriber to the existing
    /// broadcast channel. Use this for additional consumers (logging, debugging, etc.)
    /// after a [`DirectToAnsiInputDevice`] has been created.
    ///
    /// When the returned handle is dropped, it notifies the [`mio_poller`] thread to
    /// check if it should exit (when all subscribers are dropped, the thread exits).
    ///
    /// # Panics
    ///
    /// - If the [`SINGLETON`] mutex is poisoned (another thread panicked while holding
    ///   the lock).
    /// - If no device exists yet. Call [`allocate`] first.
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`mio_poller`]: super::super::mio_poller
    #[must_use]
    pub fn subscribe_to_existing() -> InputSubscriberGuard {
        SINGLETON.subscribe_to_existing()
    }
}

/// Comprehensive testing is performed in PTY integration tests:
/// - [`test_pty_input_device`]
/// - [`test_pty_mouse_events`]
/// - [`test_pty_keyboard_modifiers`]
/// - [`test_pty_utf8_text`]
/// - [`test_pty_terminal_events`]
///
/// [`test_pty_input_device`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test::test_pty_input_device
/// [`test_pty_keyboard_modifiers`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test::test_pty_keyboard_modifiers
/// [`test_pty_mouse_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test::test_pty_mouse_events
/// [`test_pty_terminal_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test::test_pty_terminal_events
/// [`test_pty_utf8_text`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test::test_pty_utf8_text
#[cfg(test)]
mod tests {
    use super::super::{paste_state_machine::PasteCollectionState,
                       protocol_conversion::convert_input_event};
    use crate::{ByteOffset, InputEvent,
                core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                           VT100KeyCodeIR,
                                                           VT100KeyModifiersIR,
                                                           VT100PasteModeIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence
        // types. These tests use the parser functions directly since they don't
        // need the device.

        // Test 1: Parse UTF-8 text (simplest case).
        let buffer: &[u8] = b"A";
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(buffer) {
            assert_eq!(bytes_consumed, ByteOffset(1));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert UTF-8 text event");
            }
        } else {
            panic!("Failed to parse UTF-8 text 'A'");
        }

        // Test 2: ESC key (single byte).
        let esc_buffer: [u8; 1] = [0x1B];
        assert_eq!(esc_buffer.len(), 1);
        assert_eq!(esc_buffer[0], 0x1B);

        // Test 3: CSI sequence for keyboard (Up Arrow: ESC [ A).
        let csi_buffer: [u8; 3] = [0x1B, 0x5B, 0x41];
        if let Some((vt100_event, bytes_consumed)) = parse_keyboard_sequence(&csi_buffer)
        {
            assert_eq!(bytes_consumed, ByteOffset(3));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }
    }

    #[tokio::test]
    async fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker.
        // Use a local paste_state to test the state machine logic directly.
        let mut paste_state = PasteCollectionState::Inactive;

        // Verify initial state is Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Simulate receiving Paste(Start) event.
        let start_event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        match (&mut paste_state, &start_event) {
            (
                state @ PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
            ) => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting.
        assert!(matches!(paste_state, PasteCollectionState::Accumulating(_)));

        // Simulate receiving keyboard events (the pasted text).
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(*ch),
                modifiers: VT100KeyModifiersIR::default(),
            };
            match (&mut paste_state, &keyboard_event) {
                (
                    PasteCollectionState::Accumulating(buffer),
                    VT100InputEventIR::Keyboard {
                        code: VT100KeyCodeIR::Char(ch),
                        ..
                    },
                ) => {
                    buffer.push(*ch);
                }
                _ => panic!("State machine should accumulate text while collecting"),
            }
        }

        // Simulate receiving Paste(End) event.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let collected_text = match (&mut paste_state, &end_event) {
            (
                state @ PasteCollectionState::Accumulating(_),
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => {
                if let PasteCollectionState::Accumulating(text) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    text
                } else {
                    panic!("Should have collected text")
                }
            }
            _ => panic!("State machine should handle Paste(End)"),
        };

        // Verify we collected the correct text.
        assert_eq!(collected_text, "Hello");

        // Verify we're back to Inactive state.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines.
        let mut paste_state = PasteCollectionState::Inactive;

        // Start collection.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            PasteCollectionState::Accumulating(_) => panic!(),
        }

        // Accumulate "Line1\nLine2".
        for ch in "Line1\nLine2".chars() {
            match &mut paste_state {
                PasteCollectionState::Accumulating(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::Inactive => panic!(),
            }
        }

        // End collection.
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "Line1\nLine2");
    }

    #[tokio::test]
    async fn test_paste_state_machine_orphaned_end() {
        // Test: Orphaned End marker (without Start) should be handled gracefully.
        let mut paste_state = PasteCollectionState::Inactive;

        // Should be Inactive initially.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Receive End marker without Start - should emit empty paste.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let result = match (&mut paste_state, &end_event) {
            (
                PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End).
        let mut paste_state = PasteCollectionState::Inactive;

        // Start.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between).
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "");
    }
}

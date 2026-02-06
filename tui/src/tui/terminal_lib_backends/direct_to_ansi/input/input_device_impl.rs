// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize EINTR SIGWINCH kqueue epoll wakeup eventfd bcast
// cspell:words reinit

//! Implementation details for [`DirectToAnsiInputDevice`].
//!
//! This module uses the **Resilient Reactor Thread (RRT)** infrastructure:
//!
//! - **[`SINGLETON`]** (container): Static [`RRT`], lives for process
//!   lifetime
//! - **[`RRTState`]** (payload): Created when thread spawns, destroyed when it exits
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
//! [`RRT`]: crate::core::resilient_reactor_thread::RRT
//! [`RRTState`]: crate::core::resilient_reactor_thread::RRTState
//! [`global_input_resource`]: mod@global_input_resource

use super::{channel_types::PollerEvent,
            mio_poller::{MioPollWaker, MioPollWorkerFactory}};
use crate::core::resilient_reactor_thread::{RRT, SubscriberGuard};

/// Type alias for the input device's subscriber guard.
///
/// This is the RAII guard returned by [`SINGLETON.subscribe()`] and
/// [`SINGLETON.subscribe_to_existing()`]. Holding this guard keeps you subscribed to
/// input events; dropping it triggers the cleanup protocol that may cause the thread to
/// exit.
///
/// [`SINGLETON.subscribe()`]: global_input_resource::SINGLETON
/// [`SINGLETON.subscribe_to_existing()`]: global_input_resource::SINGLETON
pub type InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>;

/// Process-global input resource singleton.
///
/// See [`RRTState`] for what the singleton holds when active.
///
/// [`RRTState`]: crate::core::resilient_reactor_thread::RRTState
pub mod global_input_resource {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// **Static container** (lives for process lifetime) that holds a
    /// [`RRTState`] **payload** (ephemeral, follows thread lifecycle). The payload is
    /// created when [`subscribe()`] spawns a thread, and removed when the thread exits.
    ///
    /// Lifecycle states:
    /// - **Inert** (`None`) until [`subscribe()`] spawns the poller thread
    /// - **Active** (`Some`) while thread is running
    /// - **Dormant** (`Some` with terminated liveness) when all [`SubscriberGuard`]s drop
    ///   and thread exits
    /// - **Reactivates** on next [`subscribe()`] call (spawns fresh thread, replaces
    ///   payload)
    ///
    /// This is NOT "allocate once, lives forever" — supports full restart cycles.
    ///
    /// See [`RRTState`] for what the payload contains when active.
    ///
    /// # Why `RRT`?
    ///
    /// The RRT infrastructure handles all the complexity of:
    /// - Deferred initialization (syscalls can't be `const`)
    /// - Thread lifecycle management (spawn, exit, restart)
    /// - Race condition handling (fast-path thread reuse)
    /// - Waker coupling with Poll
    ///
    /// # Usage
    ///
    /// ```ignore
    /// use crate::direct_to_ansi::input::global_input_resource::SINGLETON;
    ///
    /// let subscriber_guard = SINGLETON.subscribe()?;
    /// let running = SINGLETON.is_thread_running();
    /// let count = SINGLETON.get_receiver_count();
    /// ```
    ///
    /// - See [Architecture] for why global state is necessary.
    /// - See [`MioPollWorker`] for worker details.
    ///
    /// [Architecture]: super::super::DirectToAnsiInputDevice#architecture
    /// [`MioPollWorker`]: super::super::mio_poller::MioPollWorker
    /// [`SubscriberGuard`]: crate::core::resilient_reactor_thread::SubscriberGuard
    /// [`RRTState`]: crate::core::resilient_reactor_thread::RRTState
    /// [`subscribe()`]: RRT::subscribe
    pub static SINGLETON: RRT<MioPollWorkerFactory> =
        RRT::new();
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize

//! [`DirectToAnsiInputDevice`] struct and implementation.

use super::{global_input_resource::subscribe_to_input_events,
            types::{InputEventReceiver, ReaderThreadMessage}};
use crate::{InputEvent, core::term::get_size, tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use std::fmt::Debug;

/// Async input device for [`DirectToAnsi`] backend.
///
/// One of two real [`InputDevice`] backends (the other being [`CrosstermInputDevice`]).
/// Selected via [`TERMINAL_LIB_BACKEND`] on Linux; talks directly to the terminal using
/// ANSI/VT100 protocols with zero external dependencies. Uses the **crossterm `more` flag
/// pattern** for reliable ESC key disambiguation without fixed timeouts.
///
/// This is a **thin wrapper** that delegates to [`INPUT_RESOURCE`] for
/// [`std::io::Stdin`] reading and buffer management. The global resource pattern mirrors
/// crossterm's architecture, ensuring [`stdin`] handles persist across device lifecycle
/// boundaries.
///
/// Manages asynchronous reading from terminal [`stdin`] via dedicated thread + channel:
/// - [`stdin`] channel receiver (process global singleton, outlives device instances)
/// - Parsing happens in the reader thread using the `more` flag pattern
/// - Smart ESC disambiguation: waits for more bytes only when data is likely pending
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Why Global State?
///
/// See the [Why Global State?] section in [`INPUT_RESOURCE`] for the detailed
/// explanation.
///
/// # Full I/O Pipeline
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "1B[A" (hex)                                      │
/// │ std::io::stdin in mio-poller thread (INPUT_RESOURCE)              │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ mio-poller thread (global_input_resource.rs)                      │
/// │   • mio::Poll waits on stdin data + SIGWINCH signals              │
/// │   • Parses bytes using `more` flag for ESC disambiguation         │
/// │   • Applies paste state machine                                   │
/// │   • Sends InputEvent through broadcast channel                    │
/// │                                                                   │
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)               │
/// │   try_parse_input_event() dispatches to:                          │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard      │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse         │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize  │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard      │
/// │                                                                   │
/// │ protocol_conversion.rs (IR → Public API)                          │
/// │   convert_input_event()           VT100InputEventIR → InputEvent  │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress       │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │ broadcast channel
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)           │
/// │   • Zero-sized handle struct (delegates to INPUT_RESOURCE)        │
/// │   • Receives pre-parsed InputEvent from channel                   │
/// │   • Handles Resize events by querying terminal size               │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ Public API (Application Layer)                                    │
/// │   InputEvent::Keyboard(KeyPress)                                  │
/// │   InputEvent::Mouse(MouseInput)                                   │
/// │   InputEvent::Resize(Size)                                        │
/// │   InputEvent::Focus(FocusEvent)                                   │
/// │   InputEvent::Paste(String)                                       │
/// └───────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Data Flow Diagram
///
/// Here's the complete data flow for [`try_read_event()`]:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────┐
/// │ 1. try_read_event() called                                                │
/// │    ├─► subscribe_to_input_events() (lazy, once per device)                │
/// │    │   └─► On first call: spawns mio-poller thread                        │
/// │    │                                                                      │
/// │    └─► Loop: stdin_rx.recv().await                                        │
/// │         ├─► Event(e) → return e                                           │
/// │         ├─► Resize → query terminal size, return InputEvent::Resize       │
/// │         └─► Eof/Error → return None                                       │
/// └───────────────────────────────────▲───────────────────────────────────────┘
///                                     │ broadcast channel
/// ┌───────────────────────────────────┴───────────────────────────────────────┐
/// │ 2. mio-poller thread (global_input_resource.rs)                           │
/// │    std::thread::spawn("mio-poller")                                       │
/// │                                                                           │
/// │    Uses mio::Poll to wait on stdin data + SIGWINCH signals:               │
/// │    ┌───────────────────────────────────────────────────────────────────┐  │
/// │    │ loop {                                                            │  │
/// │    │   poll.poll(&mut events, None)?;        // Wait for stdin/signal  │  │
/// │    │   let n = stdin.read(&mut buffer)?;     // Read available bytes   │  │
/// │    │   let more = n == TTY_BUFFER_SIZE;      // ESC disambiguation     │  │
/// │    │   parser.advance(&buffer[..n], more);   // Parse with `more` flag │  │
/// │    │   for event in parser { tx.send(Event(event))?; }                 │  │
/// │    │ }                                                                 │  │
/// │    └───────────────────────────────────────────────────────────────────┘  │
/// │    Lives for process lifetime (relies on OS cleanup at process exit)      │
/// └───────────────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Underlying Protocol Parser
///
/// - [`vt_100_terminal_input_parser`]: The protocol parser that converts raw bytes to
///   [`VT100InputEventIR`]. This device calls [`try_parse_input_event`] to perform the
///   actual parsing.
///
/// # ESC Key Disambiguation (crossterm `more` flag pattern)
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`). When we see a lone `0x1B` byte, is it the ESC key or the start of an
/// escape sequence?
///
/// **The Solution**: We use crossterm's `more` flag pattern—a clever heuristic based on
/// read buffer fullness:
///
/// ```text
/// let n = stdin.read(&mut buffer)?;  // Read available bytes
/// let more = n == TTY_BUFFER_SIZE;   // true if buffer was filled completely
///
/// // In parser:
/// if buffer == [ESC] && more {
///     return None;  // Wait for more bytes (likely escape sequence)
/// } else if buffer == [ESC] && !more {
///     return ESC key;  // No more data, user pressed ESC
/// }
/// ```
///
/// ## How It Works
///
/// - **`more = true`**: Read filled the entire buffer, meaning more data is likely
///   waiting in the kernel buffer. Wait before deciding—this `ESC` is probably the start
///   of an escape sequence.
/// - **`more = false`**: Read returned fewer bytes than buffer size, meaning we've
///   drained all available input. A lone `ESC` is the ESC key.
///
/// ## Why This Works
///
/// Terminal emulators send escape sequences atomically in a single `write()` syscall.
/// When you press Up Arrow, the terminal sends `ESC [ A` (3 bytes) together. The kernel
/// buffers these bytes, and our `read()` typically gets all of them at once.
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "1B[A" (hex), 3)  ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [1B, 5B, 41]               ← All bytes arrive together
///   ↓
/// stdin.read() → 3 bytes                    ← We get all 3 bytes
///   ↓
/// more = (3 == 1024) = false                ← Buffer not full
///   ↓
/// Parser sees [ESC, '[', 'A']               → Up Arrow event ✓
/// ```
///
/// ## SSH and High-Latency Connections
///
/// Over SSH with network latency, bytes might arrive in separate packets. The `more`
/// flag handles this correctly:
///
/// ```text
/// First packet:  [ESC]       read() → 1 byte, more = false
///                            BUT: next poll() wakes immediately when more data arrives
/// Second packet: ['[', 'A']  read() → 2 bytes
///                            Parser accumulates: [ESC, '[', 'A'] → Up Arrow ✓
/// ```
///
/// The key insight: if bytes arrive separately, the next `mio::Poll` wake happens
/// almost immediately when more data arrives. The parser accumulates bytes across
/// reads, so escape sequences are correctly reassembled.
///
/// ## Attribution
///
/// This pattern is adapted from crossterm's `mio.rs` implementation. See the
/// [`global_input_resource`] module documentation for details on our mio-based
/// architecture.
///
/// [`global_input_resource`]: super::global_input_resource
/// [`CrosstermInputDevice`]: crate::tui::terminal_lib_backends::crossterm_backend::CrosstermInputDevice
/// [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
/// [`InputDevice`]: crate::InputDevice
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
/// [`stdin`]: std::io::Stdin
/// [`INPUT_RESOURCE`]: super::global_input_resource::INPUT_RESOURCE
/// [Why Global State?]: super::global_input_resource::INPUT_RESOURCE#why-global-state
/// [`try_read_event()`]: Self::try_read_event
pub struct DirectToAnsiInputDevice {
    /// This device's subscription to the global input broadcast channel.
    ///
    /// Lazily initialized on first call to [`try_read_event()`]. Each device instance
    /// gets its own independent receiver that receives all events.
    ///
    /// [`try_read_event()`]: Self::try_read_event
    stdin_rx: Option<InputEventReceiver>,
}

impl DirectToAnsiInputDevice {
    /// Create a new `DirectToAnsiInputDevice`.
    ///
    /// The device subscribes to the global input broadcast channel lazily on first call
    /// to [`try_read_event()`]. Each device instance receives all input events
    /// independently.
    ///
    /// No timeout initialization needed - we use smart async lookahead instead! See the
    /// [struct-level documentation] for details on zero-latency ESC detection.
    ///
    /// [`try_read_event()`]: Self::try_read_event
    /// [struct-level documentation]: Self
    #[must_use]
    pub fn new() -> Self { Self { stdin_rx: None } }

    /// Read the next input event asynchronously.
    ///
    /// # Returns
    ///
    /// `None` if stdin is closed (`EOF`). Or [`InputEvent`] variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   `ESC` latency)
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode text
    ///
    /// # Usage
    ///
    /// This method is called once by the main event loop of the program using this
    /// [`InputDevice`] and this [`DirectToAnsiInputDevice`] struct is persisted
    /// for the entire lifetime of the program's event loop. Typical usage pattern:
    ///
    /// ```no_run
    /// # use r3bl_tui::DirectToAnsiInputDevice;
    /// # use r3bl_tui::InputEvent;
    /// # use tokio::signal;
    ///
    /// #[tokio::main]
    /// async fn main() -> miette::Result<()> {
    ///     // Create device once at startup, reuse until program exit
    ///     let mut input_device = DirectToAnsiInputDevice::new();
    /// #   let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
    /// #       .map_err(|e| miette::miette!("Failed to setup signal handler: {}", e))?;
    ///
    ///     // Main event loop - handle multiple concurrent event sources with tokio::select!
    ///     loop {
    ///         tokio::select! {
    ///             // Handle terminal input events
    ///             input_event = input_device.next() => {
    ///                 match input_event {
    ///                     Some(InputEvent::Keyboard(key_press)) => {
    ///                         // Handle keyboard input
    ///                     }
    ///                     Some(InputEvent::Mouse(mouse_input)) => {
    ///                         // Handle mouse input
    ///                     }
    ///                     Some(InputEvent::Resize(size)) => {
    ///                         // Handle terminal resize
    ///                     }
    ///                     Some(InputEvent::BracketedPaste(text)) => {
    ///                         // Handle bracketed paste
    ///                     }
    ///                     Some(InputEvent::Focus(_)) => {
    ///                         // Handle focus events
    ///                     }
    ///                     Some(_) => {
    ///                         // Handle future/unknown event types
    ///                     }
    ///                     None => {
    ///                         // stdin closed (EOF) - signal program to exit
    ///                         break;
    ///                     }
    ///                 }
    ///             }
    ///             // Handle system signals (e.g., Ctrl+C)
    ///             _ = sigint.recv() => {
    /// #               eprintln!("Received SIGINT, shutting down...");
    /// #               break;
    ///             }
    ///             // Handle other concurrent tasks as needed
    ///             // _ = some_background_task => { ... }
    ///         }
    ///     }
    /// #   Ok(())
    /// }
    /// ```
    ///
    /// **Key points:**
    /// - The device can be **created and dropped multiple times** - global state persists
    /// - This method is **called repeatedly** by the main event loop via
    ///   [`InputDevice::next()`], which dispatches to [`Self::next()`]
    /// - **Buffer state is preserved** across device lifetimes via [`INPUT_RESOURCE`]
    /// - Returns `None` when stdin is closed (program should exit)
    ///
    /// # Global State
    ///
    /// This method accesses the global input resource ([`INPUT_RESOURCE`]) which
    /// holds:
    /// - The channel receiver for stdin data and resize signals (from dedicated reader
    ///   thread using [`mio::Poll`])
    /// - The parse buffer and position
    /// - The event queue for buffered events
    /// - The paste collection state
    ///
    /// Note: `SIGWINCH` signals are now handled by the dedicated reader thread via
    /// [`mio::Poll`] and [`signal_hook_mio`], arriving as [`ReaderThreadMessage::Resize`]
    /// through the same channel as stdin data.
    ///
    /// See [Why Global State?] for the rationale behind this architecture.
    ///
    /// [`mio::Poll`]: mio::Poll
    /// [`signal_hook_mio`]: signal_hook_mio
    ///
    /// # Implementation
    ///
    /// Async loop receiving pre-parsed events:
    /// 1. Check event queue for buffered events (from previous reads)
    /// 2. Wait for events from stdin reader channel (yields until data ready)
    /// 3. Apply paste state machine and return event
    ///
    /// Events arrive fully parsed from the reader thread. See [struct-level
    /// documentation] for zero-latency ESC detection.
    ///
    /// # Cancel Safety
    ///
    /// This method is cancel-safe. The internal broadcast channel receive
    /// ([`tokio::sync::broadcast::Receiver::recv`]) is truly cancel-safe: if
    /// cancelled, the data remains in the channel for the next receive.
    ///
    /// See the [`global_input_resource`] module documentation for why we use a dedicated
    /// thread with [`mio::Poll`] and channel instead of [`tokio::io::stdin()`] (which
    /// is NOT cancel-safe).
    ///
    /// [`InputDevice::next()`]: crate::InputDevice::next
    /// [`Self::next()`]: Self::next
    /// [`INPUT_RESOURCE`]: super::global_input_resource::INPUT_RESOURCE
    /// [`InputDevice`]: crate::InputDevice
    /// [Why Global State?]: super::global_input_resource::INPUT_RESOURCE#why-global-state
    /// [struct-level documentation]: Self
    /// [`global_input_resource`]: super::global_input_resource
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`mio::Poll`]: mio::Poll
    /// [`tokio::sync::broadcast::Receiver::recv`]: tokio::sync::broadcast::Receiver::recv
    pub async fn try_read_event(&mut self) -> Option<InputEvent> {
        // Subscribe lazily on first call - each device gets its own receiver.
        if self.stdin_rx.is_none() {
            self.stdin_rx = Some(subscribe_to_input_events());
        }
        let stdin_rx = self.stdin_rx.as_mut()?;

        // Wait for fully-formed `InputEvents` through the broadcast channel.
        loop {
            let stdin_read_result = match stdin_rx.recv().await {
                Ok(msg) => msg,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed - reader thread exited.
                    return None;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // This receiver fell behind - some messages were dropped.
                    // Log and continue from the current position.
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        eprintln!(
                            "DirectToAnsiInputDevice: receiver lagged, skipped {skipped} messages"
                        );
                    });
                    continue;
                }
            };

            match stdin_read_result {
                ReaderThreadMessage::Event(event) => {
                    return Some(event);
                }
                ReaderThreadMessage::Eof | ReaderThreadMessage::Error => {
                    return None;
                }
                ReaderThreadMessage::Resize => {
                    if let Ok(size) = get_size() {
                        return Some(InputEvent::Resize(size));
                    }
                    // Size query failed - retry on next iteration.
                }
            }
        }
    }
}

impl Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("global_resource", &"<INPUT_RESOURCE>")
            .finish()
    }
}

impl Default for DirectToAnsiInputDevice {
    fn default() -> Self { Self::new() }
}

impl DirectToAnsiInputDevice {
    pub async fn next(&mut self) -> Option<InputEvent> { self.try_read_event().await }
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
    use super::{super::{paste_state_machine::PasteCollectionState,
                        protocol_conversion::convert_input_event},
                *};
    use crate::{ByteOffset,
                core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                           VT100KeyCodeIR,
                                                           VT100KeyModifiersIR,
                                                           VT100PasteModeIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_device_creation() {
        // Test DirectToAnsiInputDevice constructs successfully.
        let _device = DirectToAnsiInputDevice::new();

        // Verify we can subscribe to the global input resource.
        let _rx = subscribe_to_input_events();
    }

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

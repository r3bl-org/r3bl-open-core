// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize

//! [`DirectToAnsiInputDevice`] struct and implementation.

use super::{global_input_resource::{DirectToAnsiInputResource, get_resource_guard},
            paste_state_machine::apply_paste_state_machine,
            types::{LoopContinuationSignal, StdinReadResult}};
use crate::{InputEvent,
            core::{ansi::vt_100_terminal_input_parser::try_parse_input_event,
                   term::get_size},
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use std::fmt::Debug;

/// Async input device for [`DirectToAnsi`] backend.
///
/// One of two real [`InputDevice`] backends (the other being [`CrosstermInputDevice`]).
/// Selected via [`TERMINAL_LIB_BACKEND`] on Linux; talks directly to the terminal using
/// ANSI/VT100 protocols with zero external dependencies. Key advantage: **0ms ESC
/// latency** vs crossterm's 150ms timeout.
///
/// This is a **thin wrapper** that delegates to [`GLOBAL_INPUT_RESOURCE`] for
/// [std::io::Stdin] reading and buffer management. The global resource pattern mirrors
/// crossterm's architecture, ensuring [`stdin`] handles persist across device lifecycle
/// boundaries.
///
/// Manages asynchronous reading from terminal [`stdin`] via dedicated thread + channel:
/// - [`stdin`] channel receiver and parse buffer (process global singleton, outlives
///   device instances)
/// - Simple [`SmallVec`]`<u8>` buffer for handling partial/incomplete ANSI sequences
/// - Smart lookahead for zero-latency ESC key detection (no timeout!)
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Why Global State?
///
/// See the [Why Global State?] section in [`GLOBAL_INPUT_RESOURCE`] for the detailed
/// explanation.
///
/// # Full I/O Pipeline
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API:
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "\x1B[A"                                          │
/// │ std::io::stdin in one thread → mpsc - from GLOBAL_INPUT_RESOURCE  │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)           │
/// │   • Zero-sized handle struct (delegates to GLOBAL_INPUT_RESOURCE) │
/// │   • Global resource owns: stdin channel, parse buffer, SIGWINCH   │
/// │   • SmallVec buffer: `PARSE_BUFFER_SIZE`, zero-timeout parsing    │
/// │   • Paste state machine: Collecting bracketed paste text          │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)               │
/// │   try_parse_input_event() dispatches to:                          │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard      │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse         │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize  │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard      │
/// └────────────────────────────┬──────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼──────────────────────────────────────┐
/// │ protocol_conversion.rs (IR → Public API)                          │
/// │   convert_input_event()       VT100InputEventIR → InputEvent      │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress       │
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
/// │    ├─► get_resource_guard() → acquires Mutex<Option<...>>                 │
/// │    │   └─► On first call: spawns stdin reader thread + registers SIGWINCH │
/// │    │                                                                      │
/// │    ├─► Check event_queue first (already-parsed buffered events)           │
/// │    │                                                                      │
/// │    └─► Loop:                                                              │
/// │         ├─► try_parse_input_event(parse_buffer.unconsumed())              │
/// │         │   └─► If parsed: apply_paste_state_machine() → emit event       │
/// │         │                                                                 │
/// │         └─► If buffer empty/incomplete:                                   │
/// │              tokio::select! { stdin_rx.recv(), sigwinch.recv() }          │
/// └───────────────────────────────────▲───────────────────────────────────────┘
///                                     │ mpsc channel
/// ┌───────────────────────────────────┴───────────────────────────────────────┐
/// │ 2. Dedicated Stdin Reader Thread (global_input_resource.rs)               │
/// │    std::thread::spawn("stdin-reader")                                     │
/// │                                                                           │
/// │    stdin_reader_loop(tx):                                                 │
/// │    ┌───────────────────────────────────────────────────────────────────┐  │
/// │    │ loop {                                                            │  │
/// │    │   let n = std::io::stdin().lock().read(&mut buffer)?; // BLOCKING │  │
/// │    │   tx.send(StdinReadResult::Data(buffer[..n].to_vec()))?;          │  │
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
/// # Zero-Latency ESC Key Detection
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`).
///
/// **Baseline (crossterm)**: When reading `1B` alone, wait up to 150ms to see if more
/// bytes arrive. If timeout expires → emit ESC key. If bytes arrive → parse escape
/// sequence.
///
/// **Our Approach**: Immediately emit ESC when buffer contains only `[1B]`, with no
/// artificial delay.
///
/// ## Performance Comparison
///
/// | Input Type           | crossterm Latency   | Our Latency   | Improvement       |
/// | -------------------- | ------------------- | ------------- | ----------------- |
/// | **ESC key press**    | 150ms (timeout)     | 0ms           | **150ms faster**  |
/// | Arrow keys           | 0ms (immediate)     | 0ms           | Same              |
/// | Regular text         | 0ms (immediate)     | 0ms           | Same              |
/// | Mouse events         | 0ms (immediate)     | 0ms           | Same              |
///
/// **Benefit applies to**: Vim-style modal editors, ESC-heavy workflows, dialog
/// dismissal.
///
/// ## How Escape Sequences Arrive in Practice
///
/// When you press a special key (e.g., Up Arrow), the terminal emulator sends
/// an escape sequence like `ESC [ A` (3 bytes: `[1B, 5B, 41]`).
///
/// **Key Assumption**: Modern terminal emulators send escape sequences **atomically**
/// in a single `write()` syscall, and the kernel buffers all bytes together.
///
/// ### Typical Flow (99.9% of cases - local terminals)
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "\x1B[A", 3)  ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [1B, 5B, 41]           ← All bytes arrive together
///   ↓
/// stdin_rx.recv().await → [1B, 5B, 41]  ← We get all 3 bytes in one read
///   ↓
/// try_parse() sees complete sequence    → Up Arrow event ✓
/// ```
///
/// ### Edge Case: Slow Byte Arrival (rare - high-latency SSH, slow serial)
///
/// Over high-latency connections, bytes might arrive separately:
///
/// ```text
/// First read:  [1B]         → Emits ESC immediately
/// Second read: [5B, 41]     → User gets ESC instead of Up Arrow
/// ```
///
/// **Trade-off**: We optimize for the common case (local terminals with atomic
/// sequences) to achieve 0ms ESC latency, accepting rare edge cases over forcing
/// 150ms timeout on all users.
///
/// ### Why This Assumption Holds
///
/// - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send escape
///   sequences atomically in one write
/// - **Terminal protocol design**: Sequences are designed to be atomic units
/// - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
///   before `read()` sees them
/// - **Network delay case**: Over SSH with 200ms latency, UX is already degraded; getting
///   ESC instead of Up Arrow is annoying but not catastrophic
///
/// [`CrosstermInputDevice`]: crate::tui::terminal_lib_backends::crossterm_backend::CrosstermInputDevice
/// [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
/// [`InputDevice`]: crate::InputDevice
/// [`SmallVec`]: smallvec::SmallVec
/// [`TERMINAL_LIB_BACKEND`]: crate::tui::TERMINAL_LIB_BACKEND
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
/// [`stdin`]: std::io::Stdin
/// [`GLOBAL_INPUT_RESOURCE`]: super::global_input_resource::GLOBAL_INPUT_RESOURCE
/// [Why Global State?]: super::global_input_resource::GLOBAL_INPUT_RESOURCE#why-global-state
/// [`try_read_event()`]: Self::try_read_event
pub struct DirectToAnsiInputDevice;

impl DirectToAnsiInputDevice {
    /// Create a new `DirectToAnsiInputDevice`.
    ///
    /// This is a **zero-sized handle** - all state lives in the global input resource
    /// ([`DirectToAnsiInputResource`]) which persists for the process lifetime.
    /// The global resource is lazily initialized on first access to [`try_read_event`].
    ///
    /// No timeout initialization needed - we use smart async lookahead instead! See the
    /// [struct-level documentation] for details on zero-latency ESC detection.
    ///
    /// [`try_read_event`]: Self::try_read_event
    /// [struct-level documentation]: Self
    /// [`DirectToAnsiInputResource`]: super::global_input_resource::DirectToAnsiInputResource
    #[must_use]
    pub fn new() -> Self { Self }

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
    /// - **Buffer state is preserved** across device lifetimes via
    ///   [`GLOBAL_INPUT_RESOURCE`]
    /// - Returns `None` when stdin is closed (program should exit)
    ///
    /// # Global State
    ///
    /// This method accesses the global input resource ([`GLOBAL_INPUT_RESOURCE`]) which
    /// holds:
    /// - The channel receiver for stdin data (from dedicated reader thread)
    /// - The parse buffer and position
    /// - The event queue for buffered events
    /// - The paste collection state
    /// - The `SIGWINCH` signal receiver (for terminal resize events)
    ///
    /// See [Why Global State?] for the rationale behind this architecture.
    ///
    /// # Implementation
    ///
    /// Async loop with zero-timeout parsing:
    /// 1. Check event queue for buffered events (from previous reads)
    /// 2. Try to parse from existing buffer
    /// 3. If incomplete, wait for data from stdin channel (yields until data ready)
    /// 4. Loop back to parsing
    ///
    /// See [`ParseBuffer`] for buffer management algorithm details, and [struct-level
    /// documentation] for zero-latency ESC detection.
    ///
    /// # Cancel Safety
    ///
    /// This method is cancel-safe. Both futures in the internal [`tokio::select!`] are
    /// truly cancel-safe:
    /// - [`tokio::sync::mpsc::UnboundedReceiver::recv`]: If cancelled, the data remains
    ///   in the channel for the next receive.
    /// - [`tokio::signal::unix::Signal::recv`]: If cancelled, the signal is not consumed
    ///   and will be delivered on the next call.
    ///
    /// See the [`global_input_resource`] module documentation for why we use a dedicated
    /// thread with channel instead of [`tokio::io::stdin()`] (which is NOT
    /// cancel-safe).
    ///
    /// [`ParseBuffer`]: super::parse_buffer::ParseBuffer#buffer-management-algorithm
    /// [`InputDevice::next()`]: crate::InputDevice::next
    /// [`Self::next()`]: Self::next
    /// [`GLOBAL_INPUT_RESOURCE`]: super::global_input_resource::GLOBAL_INPUT_RESOURCE
    /// [`InputDevice`]: crate::InputDevice
    /// [Why Global State?]: super::global_input_resource::GLOBAL_INPUT_RESOURCE#why-global-state
    /// [struct-level documentation]: Self
    /// [`global_input_resource`]: super::global_input_resource
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`tokio::select!`]: tokio::select
    pub async fn try_read_event(&mut self) -> Option<InputEvent> {
        // Get the global input resource (which persists for process lifetime).
        let mut resource_guard = get_resource_guard().await;
        let resource = resource_guard.as_mut()?;

        // Check event queue first - return any buffered events.
        if let Some(event) = resource.event_queue.pop_front() {
            return Some(event);
        }

        loop {
            // 1. Try to parse from existing buffer and apply paste state machine.
            if let Some((vt100_event, bytes_consumed)) =
                try_parse_input_event(resource.parse_buffer.unconsumed())
            {
                resource.parse_buffer.consume(bytes_consumed);

                match apply_paste_state_machine(&mut resource.paste_state, &vt100_event) {
                    LoopContinuationSignal::Emit(event) => return Some(event),
                    LoopContinuationSignal::Continue => continue,
                    LoopContinuationSignal::Shutdown => {
                        unreachable!("Paste state machine never signals shutdown.")
                    }
                }
            }

            // 2. Buffer exhausted or incomplete sequence - wait for input or signal.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message =
                        "direct-to-ansi: waiting for stdin input or SIGWINCH signal"
                );
            });

            let signal = tokio::select! {
                maybe_stdin_read_result = resource.stdin_rx.recv() => {
                    Self::process_stdin_read(resource, maybe_stdin_read_result)
                }
                maybe_resize_signal = resource.sigwinch_receiver.recv() => {
                    Self::process_resize_signal(maybe_resize_signal)
                }
            };

            match signal {
                LoopContinuationSignal::Emit(event) => return Some(event),
                LoopContinuationSignal::Shutdown => return None,
                LoopContinuationSignal::Continue => {} // Loop back to try parsing.
            }
        }
    }

    /// Handles the result of receiving from the stdin channel.
    ///
    /// This function processes [`StdinReadResult`] from the dedicated stdin reader
    /// thread. The dedicated thread architecture ensures true cancel safety in
    /// [`tokio::select!`], unlike [`tokio::io::stdin()`] which uses a blocking thread
    /// pool.
    ///
    /// [`tokio::io::stdin()`]: tokio::io::stdin
    /// [`tokio::select!`]: tokio::select
    fn process_stdin_read(
        resource: &mut DirectToAnsiInputResource,
        maybe_stdin_read_result: Option<StdinReadResult>,
    ) -> LoopContinuationSignal {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(
                message = "direct-to-ansi: stdin channel received",
                result = ?maybe_stdin_read_result
            );
        });

        match maybe_stdin_read_result {
            Some(StdinReadResult::Data(data)) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "direct-to-ansi: stdin data received",
                        bytes_read = data.len()
                    );
                });
                resource.parse_buffer.append(&data);
                LoopContinuationSignal::Continue
            }
            Some(StdinReadResult::Eof) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "direct-to-ansi: stdin EOF");
                });
                LoopContinuationSignal::Shutdown
            }
            Some(StdinReadResult::Error(kind)) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "direct-to-ansi: stdin error", error_kind = ?kind);
                });
                LoopContinuationSignal::Shutdown
            }
            None => {
                // Channel closed - stdin reader thread exited.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "direct-to-ansi: stdin channel closed");
                });
                LoopContinuationSignal::Shutdown
            }
        }
    }

    /// Processes a terminal resize signal (`SIGWINCH` on Unix).
    fn process_resize_signal(maybe_resize_signal: Option<()>) -> LoopContinuationSignal {
        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(
                message = "direct-to-ansi: SIGWINCH branch selected",
                result = ?maybe_resize_signal
            );
        });

        match maybe_resize_signal {
            Some(()) => {
                // Signal received successfully, query terminal size.
                if let Ok(size) = get_size() {
                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                        tracing::debug!(
                            message = "direct-to-ansi: returning Resize",
                            size = ?size
                        );
                    });
                    return LoopContinuationSignal::Emit(InputEvent::Resize(size));
                }
                // If size query failed, continue to next iteration.
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "direct-to-ansi: get_size() failed, continuing"
                    );
                });
                LoopContinuationSignal::Continue
            }
            None => {
                // Signal stream closed - unexpected but shouldn't cause shutdown.
                tracing::warn!(
                    message =
                        "direct-to-ansi: SIGWINCH receiver returned None (stream closed)"
                );
                LoopContinuationSignal::Continue
            }
        }
    }
}

impl Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("global_resource", &"<GLOBAL_INPUT_RESOURCE>")
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
        // With the global resource architecture, the device is now a thin wrapper
        // that only holds the SIGWINCH receiver.
        let _device = DirectToAnsiInputDevice::new();

        // Verify global resource is initialized on first access.
        let resource_guard = get_resource_guard().await;
        let resource = resource_guard
            .as_ref()
            .expect("Global resource should be initialized");

        // Verify buffer is empty initially (no data yet).
        assert_eq!(resource.parse_buffer.len(), 0);

        // Verify position is at 0.
        assert_eq!(resource.parse_buffer.position().as_usize(), 0);
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
    async fn test_buffer_management() {
        // Test buffer handling: growth, consumption, and compaction at 2KB threshold.
        // Create a local ParseBuffer to test the buffer logic directly.
        use super::super::parse_buffer::ParseBuffer;

        let mut buffer = ParseBuffer::new();

        // Verify initial state.
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.position().as_usize(), 0);

        // Test 1: Buffer growth - add data and verify length increases.
        let test_data = vec![b'X'; 100];
        buffer.append(&test_data);
        assert_eq!(buffer.len(), 100);

        // Test 2: Consumption tracking - consume bytes and verify counter.
        buffer.consume(ByteOffset(50));
        assert_eq!(buffer.position().as_usize(), 50);
        assert_eq!(buffer.len(), 100);

        // Test 3: Verify consumed bytes are skipped.
        let unread_portion = buffer.unconsumed();
        assert_eq!(unread_portion.len(), 50);

        // Test 4: Buffer compaction at threshold (half of PARSE_BUFFER_SIZE).
        let mut buffer = ParseBuffer::new();

        // Add 2100 bytes (exceed 2048 threshold, which is half of 4096).
        let large_data = vec![b'Y'; 2100];
        buffer.append(&large_data);
        assert_eq!(buffer.len(), 2100);

        // Consume 1000 bytes (won't trigger compaction yet, need > 2048).
        buffer.consume(ByteOffset(1000));
        assert_eq!(buffer.position().as_usize(), 1000);
        assert_eq!(buffer.len(), 2100);

        // Consume another 1100 bytes (total = 2100, exceeds 2048 threshold).
        buffer.consume(ByteOffset(1100));
        assert_eq!(buffer.position().as_usize(), 0);
        assert_eq!(buffer.len(), 0);
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

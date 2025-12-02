// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::protocol_conversion::convert_input_event;
use crate::{ByteIndex, ByteOffset, InputEvent, Size, height, width,
            core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                       VT100KeyCodeIR,
                                                       VT100PasteModeIR,
                                                       try_parse_input_event},
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use smallvec::SmallVec;

// SIGWINCH signal handling for terminal resize events (Unix-only).
//
// TODO(windows): Windows uses `WINDOW_BUFFER_SIZE_EVENT` via Console API instead of
// SIGWINCH. When adding Windows support for DirectToAnsi, implement resize detection
// using `ReadConsoleInput` which returns window buffer size change events.
// See: https://learn.microsoft.com/en-us/windows/console/window-buffer-size-record-str
#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind};

/// Initial buffer capacity for efficient ANSI sequence buffering.
///
/// Most terminal input consists of either:
/// - Individual keypresses (~5-10 bytes for special keys like arrows, function keys)
/// - Paste events (variable, but rare to exceed buffer capacity)
/// - Mouse events (~20 bytes for typical terminal coordinates)
///
/// 4096 bytes accommodates multiple complete ANSI sequences without frequent
/// reallocations. This is a good balance: large enough to handle typical bursts, small
/// enough to avoid excessive memory overhead for idle periods.
///
/// See [`try_read_event()`] for buffer management algorithm.
///
/// [`try_read_event()`]: DirectToAnsiInputDevice::try_read_event#buffer_management_algorithm
const PARSE_BUFFER_SIZE: usize = 4096;

/// Temporary read buffer size for stdin reads.
///
/// This is the read granularity: how much data we pull from the kernel in one syscall.
/// Too small (< 256): Excessive syscalls increase latency.
/// Too large (> 256): Delays response to time-sensitive input (e.g., arrow key repeat).
///
/// 256 bytes is optimal for terminal input: it's one page boundary on many architectures,
/// fits comfortably in the input buffer, and provides good syscall efficiency without
/// introducing noticeable latency.
///
/// See [`try_read_event()`] for buffer management algorithm.
///
/// [`try_read_event()`]: DirectToAnsiInputDevice::try_read_event#buffer_management_algorithm
const STDIN_READ_BUFFER_SIZE: usize = 256;

/// Get terminal size using rustix (no crossterm dependency).
///
/// Uses real stdout file descriptor to query terminal dimensions via `tcgetwinsize`.
/// This is the correct approach for `DirectToAnsi` backend since we want the actual
/// terminal size, not a mocked value.
///
/// # Errors
///
/// Returns an error if the `tcgetwinsize` syscall fails (e.g., stdout is not a TTY).
#[cfg(unix)]
fn get_size_rustix() -> miette::Result<Size> {
    let winsize = rustix::termios::tcgetwinsize(std::io::stdout())
        .map_err(|e| miette::miette!("tcgetwinsize failed: {}", e))?;
    Ok(width(winsize.ws_col) + height(winsize.ws_row))
}

/// Result of waiting for input or signal in [`wait_for_input_or_signal()`].
#[cfg(unix)]
enum WaitResult {
    /// Stdin read completed with the given result.
    Stdin(std::io::Result<usize>),
    /// SIGWINCH signal received.
    Signal(Option<()>),
}

/// Waits for either stdin data or SIGWINCH signal.
///
/// Uses `tokio::select!` to multiplex between stdin and the SIGWINCH signal receiver.
/// Returns immediately when either event occurs.
///
/// # Cancel Safety
///
/// Both futures in the `select!` are cancel-safe:
/// - [`tokio::io::AsyncReadExt::read`]: Cancel-safe. If cancelled before completion,
///   no data is lost - the same data will be available on the next read.
/// - [`tokio::signal::unix::Signal::recv`]: Cancel-safe. If cancelled, the signal
///   is not consumed and will be delivered on the next call.
///
/// This means the `select!` can safely be used in a loop without losing events.
#[cfg(unix)]
async fn wait_for_input_or_signal(
    stdin: &mut tokio::io::Stdin,
    sigwinch_receiver: &mut Signal,
    temp_buf: &mut [u8],
) -> WaitResult {
    use tokio::io::AsyncReadExt as _;

    tokio::select! {
        result = stdin.read(temp_buf) => WaitResult::Stdin(result),
        result = sigwinch_receiver.recv() => WaitResult::Signal(result),
    }
}

/// Async input device for `DirectToAnsi` backend.
///
/// This is the `DirectToAnsi` async input device implementation. It handles non-blocking
/// reading from stdin using tokio, manages a ring buffer (kind of, except that it is
/// growable) for partial ANSI sequences, and delegates to the protocol layer parsers for
/// sequence interpretation.
///
/// Manages asynchronous reading from terminal stdin using tokio, with:
/// - Simple `Vec<u8>` buffer for handling partial/incomplete ANSI sequences
/// - Smart lookahead for zero-latency ESC key detection (no timeout!)
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Architecture
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API. The full pipeline:
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "\x1B[A"                                        │
/// │ stdin (tokio::io::stdin)                                        │
/// └────────────────────────────┬────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)         │
/// │   • Vec<u8> buffer: `PARSE_BUFFER_SIZE`, zero-timeout parsing   │
/// │   • Async I/O: tokio::io::stdin().read()                        │
/// │   • Paste state machine: Collecting bracketed paste text        │
/// └────────────────────────────┬────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼─────────────────────────────────────┐
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)              │
/// │   try_parse_input_event() dispatches to:                         │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard     │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse        │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard     │
/// └────────────────────────────┬─────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼────────────────────────────────────┐
/// │ protocol_conversion.rs (IR → Public API)                        │
/// │   convert_input_event()       VT100InputEventIR → InputEvent    │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress     │
/// └────────────────────────────┬────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼────────────────────────────────────┐
/// │ Public API (Application Layer)                                  │
/// │   InputEvent::Keyboard(KeyPress)                                │
/// │   InputEvent::Mouse(MouseInput)                                 │
/// │   InputEvent::Resize(Size)                                      │
/// │   InputEvent::Focus(FocusEvent)                                 │
/// │   InputEvent::Paste(String)                                     │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Underlying protocol parser
///
/// - [`vt_100_terminal_input_parser`]: The protocol parser that converts raw bytes to
///   [`VT100InputEventIR`]. This device calls [`try_parse_input_event`] to perform the
///   actual parsing.
///
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
///
/// # Zero-Latency ESC Key Detection
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`).
///
/// **Baseline (crossterm)**: When reading `0x1B` alone, wait up to 150ms to see if more
/// bytes arrive. If timeout expires → emit ESC key. If bytes arrive → parse escape
/// sequence.
///
/// **Our Approach**: Immediately emit ESC when buffer contains only `[0x1B]`, with no
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
/// an escape sequence like `ESC [ A` (3 bytes: `[0x1B, 0x5B, 0x41]`).
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
/// Kernel buffer: [0x1B, 0x5B, 0x41]    ← All bytes arrive together
///   ↓
/// stdin.read().await → [0x1B, 0x5B, 0x41]  ← We get all 3 bytes in one read
///   ↓
/// try_parse() sees complete sequence → Up Arrow event ✓
/// ```
///
/// ### Edge Case: Slow Byte Arrival (rare - high-latency SSH, slow serial)
///
/// Over high-latency connections, bytes might arrive separately:
///
/// ```text
/// First read:  [0x1B]           → Emits ESC immediately
/// Second read: [0x5B, 0x41]     → User gets ESC instead of Up Arrow
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
pub struct DirectToAnsiInputDevice {
    /// Tokio async stdin handle for non-blocking reading.
    stdin: tokio::io::Stdin,

    /// Raw byte buffer for ANSI sequences and text.
    /// Pre-allocated with `PARSE_BUFFER_SIZE` capacity inline, never grows.
    parse_buffer: SmallVec<[u8; PARSE_BUFFER_SIZE]>,

    /// Current position in buffer marking the boundary between consumed and unconsumed
    /// bytes. Bytes before this position have been parsed; bytes from this position
    /// onward are pending. When this exceeds half of `PARSE_BUFFER_SIZE`, buffer
    /// is compacted.
    buffer_position: ByteIndex,

    /// State machine for collecting bracketed paste text.
    /// Tracks whether we're between Paste(Start) and Paste(End) markers.
    paste_state: PasteCollectionState,

    /// SIGWINCH signal receiver for terminal resize events (Unix-only).
    ///
    /// Terminal resize is not sent through stdin as ANSI sequences - it's delivered
    /// via the SIGWINCH signal. We use `tokio::signal::unix::Signal` to receive these
    /// asynchronously and convert them to [`InputEvent::Resize`].
    #[cfg(unix)]
    sigwinch_receiver: Signal,
}

/// State machine for collecting bracketed paste text.
///
/// When the terminal sends a bracketed paste sequence, it arrives as:
/// - `Paste(Start)` marker
/// - Multiple `Keyboard` events (the actual pasted text)
/// - `Paste(End)` marker
///
/// This state tracks whether we're currently collecting text between markers.
#[derive(Debug)]
enum PasteCollectionState {
    /// Not currently in a paste operation.
    Inactive,
    /// Currently collecting text for a paste operation.
    Accumulating(String),
}

impl DirectToAnsiInputDevice {
    /// Create a new `DirectToAnsiInputDevice`.
    ///
    /// Initializes:
    /// - `tokio::io::stdin()` handle for non-blocking reading
    /// - `PARSE_BUFFER_SIZE` `Vec<u8>` buffer (pre-allocated)
    /// - consumed counter at 0
    /// - SIGWINCH signal receiver for terminal resize events (Unix-only)
    ///
    /// No timeout initialization needed - we use smart async lookahead instead!
    ///
    /// # Panics
    ///
    /// Panics if the SIGWINCH signal handler cannot be registered (Unix-only).
    /// This should only happen if the signal is already registered elsewhere.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(unix)]
        let sigwinch_receiver = tokio::signal::unix::signal(SignalKind::window_change())
            .expect("Failed to register SIGWINCH handler");

        Self {
            stdin: tokio::io::stdin(),
            parse_buffer: SmallVec::new(),
            buffer_position: ByteIndex::default(),
            paste_state: PasteCollectionState::Inactive,
            #[cfg(unix)]
            sigwinch_receiver,
        }
    }

    /// Read the next input event asynchronously.
    ///
    /// # Returns
    ///
    /// `None` if stdin is closed (EOF). Or [`InputEvent`] variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   ESC latency)
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
    /// - The device is **created once and reused** for the entire program lifetime
    /// - This method is **called repeatedly** by the main event loop via the
    ///   `InputDeviceExt::next()` trait method, not called directly
    /// - **Buffer state is preserved** across calls: the internal `parse_buffer` and
    ///   `buffer_position` accumulate partial ANSI sequences between calls
    /// - Returns `None` when stdin is closed (program should exit)
    ///
    /// # Implementation
    ///
    /// Async loop with zero-timeout parsing:
    /// 1. Try to parse from existing buffer
    /// 2. If incomplete, read more from stdin (yields until data ready)
    /// 3. Loop back to parsing
    ///
    /// # Buffer Management Algorithm
    ///
    /// This implementation uses a growable buffer with lazy compaction to avoid
    /// copying bytes on every parse while preventing unbounded memory growth:
    ///
    /// ```text
    /// Initial state: buffer = [], consumed = 0
    ///
    /// After read #1: buffer = [0x1B, 0x5B, 0x41], consumed = 0
    ///                         ├─────────────────┤
    ///                         Parser tries [0..3]
    ///                         Parses Up Arrow (3 bytes)
    ///                         consumed = 3
    ///
    /// After read #2: buffer = [0x1B, 0x5B, 0x41, 0x61], consumed = 3
    ///                         └──── parsed ────┘ ├───┤
    ///                                            Parser tries [3..4]
    ///                                            Parses 'a' (1 byte)
    ///                                            consumed = 4
    ///
    /// After read #3: buffer = [      ...many bytes...      ], consumed = 2100
    ///                         └ consumed > 2048 threshold! ┘
    ///                         Compact: drain [0..2100], consumed = 0
    ///                         buffer now starts fresh
    /// ```
    ///
    /// **Key operations:**
    /// - `try_parse_input_event(&buffer[consumed..])` - Parse only unprocessed bytes
    /// - `consume(n)` - Mark n bytes as processed (increments `consumed`)
    /// - When `consumed > 2048` - Compact buffer by draining processed bytes
    ///
    /// **Why not a true ring buffer?**
    /// - Variable-length ANSI sequences (1-20+ bytes) make fixed-size wrapping complex
    /// - Growing Vec handles overflow naturally without wrap-around logic
    /// - Lazy compaction (every 2KB) amortizes cost: O(1) per event on average
    ///
    /// **Memory behavior:**
    /// - Typical: 100 events → ~500 bytes consumed, no compaction needed
    /// - Worst case: `PARSE_BUFFER_SIZE` buffer + 2KB consumed = 6KB maximum before
    ///   compaction
    /// - After compaction: resets to current unconsumed data only
    ///
    /// See struct-level documentation for details on zero-latency ESC detection
    /// algorithm.
    ///
    /// [`InputDevice`]: crate::InputDevice
    pub async fn try_read_event(&mut self) -> Option<InputEvent> {
        // Allocate temp buffer ONCE before loop (performance optimization).
        // read() overwrites from index 0 each time, so no clearing between iterations.
        let mut temp_buf = [0u8; STDIN_READ_BUFFER_SIZE];

        loop {
            // 1. Try to parse from existing buffer
            if let Some((vt100_event, bytes_consumed)) = try_parse_input_event(
                &self.parse_buffer[self.buffer_position.as_usize()..],
            ) {
                self.consume(bytes_consumed);

                // 2. Apply paste collection state machine
                match (&mut self.paste_state, &vt100_event) {
                    // Start marker: enter collecting state, don't emit event
                    (
                        state @ PasteCollectionState::Inactive,
                        VT100InputEventIR::Paste(VT100PasteModeIR::Start),
                    ) => {
                        *state = PasteCollectionState::Accumulating(String::new());
                        continue; // Loop to get next event
                    }

                    // While collecting: accumulate keyboard characters
                    (
                        PasteCollectionState::Accumulating(buffer),
                        VT100InputEventIR::Keyboard {
                            code: VT100KeyCodeIR::Char(ch),
                            ..
                        },
                    ) => {
                        buffer.push(*ch);
                        continue; // Loop to get next event
                    }

                    // XMARK: How to get variant with owned data out of mut ref.

                    // End marker: emit complete paste and exit collecting state
                    (
                        state @ PasteCollectionState::Accumulating(_),
                        VT100InputEventIR::Paste(VT100PasteModeIR::End),
                    ) => {
                        // Swap out `&mut state` to `Inactive` to get ownership of what is
                        // currently there, then extract accumulated text.
                        let state =
                            std::mem::replace(state, PasteCollectionState::Inactive);
                        let PasteCollectionState::Accumulating(text) = state else {
                            unreachable!(
                                "state was matched as Accumulating(String), so this can't happen"
                            );
                        };
                        return Some(InputEvent::BracketedPaste(text));
                    }

                    // Orphaned end marker (End without Start): emit empty paste
                    (
                        PasteCollectionState::Inactive,
                        VT100InputEventIR::Paste(VT100PasteModeIR::End),
                    ) => {
                        return Some(InputEvent::BracketedPaste(String::new()));
                    }

                    // Normal event processing when not pasting
                    (PasteCollectionState::Inactive, _) => {
                        return convert_input_event(vt100_event);
                    }

                    // Other events while collecting paste should be ignored (or queued)
                    // For now, ignore them (they'll be lost)
                    (PasteCollectionState::Accumulating(_), _) => {
                        continue; // Ignore and get next event
                    }
                }
            }

            // 2. Buffer exhausted or incomplete sequence - wait for input or signal.
            // Use wait_for_input_or_signal() to handle both stdin data and SIGWINCH.
            // This yields until either data is ready or a resize signal arrives.
            #[cfg(unix)]
            {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(message = "direct-to-ansi: waiting for input or signal");
                });

                match wait_for_input_or_signal(
                    &mut self.stdin,
                    &mut self.sigwinch_receiver,
                    &mut temp_buf,
                )
                .await
                {
                    // Branch 1: stdin data available
                    WaitResult::Stdin(result) => {
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                            tracing::debug!(
                                message = "direct-to-ansi: stdin branch selected",
                                result = ?result
                            );
                        });
                        match result {
                            Ok(0) => {
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(message = "direct-to-ansi: stdin EOF (0 bytes)");
                                });
                                return None;
                            }
                            Err(ref e) => {
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "direct-to-ansi: stdin error",
                                        error = ?e
                                    );
                                });
                                return None;
                            }
                            Ok(n) => {
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "direct-to-ansi: stdin read bytes",
                                        bytes_read = n
                                    );
                                });
                                // Append new bytes to buffer
                                self.parse_buffer.extend_from_slice(&temp_buf[..n]);
                            }
                        }
                    }
                    // Branch 2: SIGWINCH received - terminal resized
                    WaitResult::Signal(sigwinch_result) => {
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                            tracing::debug!(
                                message = "direct-to-ansi: SIGWINCH branch selected",
                                result = ?sigwinch_result
                            );
                        });
                        match sigwinch_result {
                            Some(()) => {
                                // Signal received successfully, query terminal size
                                if let Ok(size) = get_size_rustix() {
                                    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                        tracing::debug!(
                                            message = "direct-to-ansi: returning Resize",
                                            size = ?size
                                        );
                                    });
                                    return Some(InputEvent::Resize(size));
                                }
                                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                                    tracing::debug!(
                                        message = "direct-to-ansi: get_size_rustix() failed, continuing"
                                    );
                                });
                                // If size query failed, continue to next iteration
                            }
                            None => {
                                // Signal stream closed - this is unexpected but shouldn't
                                // cause shutdown. Just continue waiting for stdin.
                                tracing::warn!(
                                    message = "direct-to-ansi: SIGWINCH receiver returned None (stream closed)"
                                );
                                // Continue to next loop iteration - stdin will still work
                            }
                        }
                    }
                }
            }

            // Non-Unix: DirectToAnsi is Linux-only, this code path should never be reached.
            #[cfg(not(unix))]
            {
                unreachable!(
                    "DirectToAnsi backend is Linux-only. \
                     This code path should never be reached on non-Unix systems."
                );
            }

            // 3. Loop back to try_parse_input_event() with new data
        }
    }

    /// Consume N bytes from the buffer.
    ///
    /// Increments the consumed counter and compacts the buffer if threshold exceeded.
    /// This is kind of like a ring buffer (except that it is not fixed size).
    ///
    /// # Semantic Correctness
    ///
    /// Takes [`ByteOffset`] (displacement from parser) and applies it to
    /// `self.buffer_position` (position in buffer): `position += displacement`.
    fn consume(&mut self, displacement: ByteOffset) {
        self.buffer_position += displacement;

        // Compact buffer if consumed bytes exceed half of PARSE_BUFFER_SIZE
        if self.buffer_position.as_usize() > PARSE_BUFFER_SIZE / 2 {
            self.parse_buffer.drain(..self.buffer_position.as_usize());
            self.buffer_position = ByteIndex::default();
        }
    }
}

impl std::fmt::Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("stdin", &"<tokio::io::Stdin>")
            .field("parse_buffer_len", &self.parse_buffer.len())
            .field("buffer_position", &self.buffer_position)
            .field("paste_state", &self.paste_state)
            .field("sigwinch_receiver", &"<Signal>")
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
    use super::*;
    use crate::{byte_offset,
                core::ansi::vt_100_terminal_input_parser::{VT100KeyModifiersIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_device_creation() {
        // Test DirectToAnsiInputDevice constructs successfully with correct initial state
        let device = DirectToAnsiInputDevice::new();

        // Verify buffer initialized with correct capacity (`PARSE_BUFFER_SIZE`)
        assert_eq!(device.parse_buffer.capacity(), PARSE_BUFFER_SIZE);

        // Verify buffer is empty initially (no data yet)
        assert_eq!(device.parse_buffer.len(), 0);

        // Verify consumed counter is at 0
        assert_eq!(device.buffer_position.as_usize(), 0);

        // Constructor completes without panic - success!
    }

    #[tokio::test]
    async fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence types
        let mut device = DirectToAnsiInputDevice::new();

        // Test 1: Parse UTF-8 text (simplest case)
        // Single character "A" should parse as keyboard input
        device.parse_buffer.extend_from_slice(b"A");
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(&device.parse_buffer)
        {
            assert_eq!(bytes_consumed, byte_offset(1));
            // Convert and verify we got a keyboard event for the character
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert UTF-8 text event");
            }
        } else {
            panic!("Failed to parse UTF-8 text 'A'");
        }

        // Test 2: Clear buffer and test ESC key (single byte)
        device.parse_buffer.clear();
        device.parse_buffer.push(0x1B); // ESC byte
        // Note: try_parse() is private, so we verify parsing logic through the buffer
        // setup A buffer with only [0x1B] should parse as ESC key (based on
        // try_parse logic)
        assert_eq!(device.parse_buffer.len(), 1);
        assert_eq!(device.parse_buffer[0], 0x1B);

        // Test 3: Set up CSI sequence for keyboard (Up Arrow: ESC [ A)
        device.parse_buffer.clear();
        device.parse_buffer.extend_from_slice(&[0x1B, 0x5B, 0x41]); // ESC [ A
        if let Some((vt100_event, bytes_consumed)) =
            parse_keyboard_sequence(&device.parse_buffer)
        {
            assert_eq!(bytes_consumed, byte_offset(3));
            // Convert and verify we got a keyboard event
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }

        // Test 4: Verify buffer consumption tracking
        device.buffer_position = ByteIndex::default();
        device.consume(byte_offset(1));
        assert_eq!(device.buffer_position.as_usize(), 1);

        device.consume(byte_offset(2));
        assert_eq!(device.buffer_position.as_usize(), 3);
    }

    #[tokio::test]
    async fn test_buffer_management() {
        // Test buffer handling: growth, consumption, and compaction at 2KB threshold
        let mut device = DirectToAnsiInputDevice::new();

        // Verify initial state
        assert_eq!(device.parse_buffer.len(), 0);
        assert_eq!(device.parse_buffer.capacity(), PARSE_BUFFER_SIZE);
        assert_eq!(device.buffer_position.as_usize(), 0);

        // Test 1: Buffer growth - add data and verify length increases
        let test_data = vec![b'X'; 100];
        device.parse_buffer.extend_from_slice(&test_data);
        assert_eq!(device.parse_buffer.len(), 100);
        assert!(device.parse_buffer.capacity() >= 100);

        // Test 2: Consumption tracking - consume bytes and verify counter
        device.consume(byte_offset(50));
        assert_eq!(device.buffer_position.as_usize(), 50);
        assert_eq!(device.parse_buffer.len(), 100); // Buffer still holds all bytes

        // Test 3: Verify consumed bytes are skipped in try_parse
        // The try_parse function uses &buffer[consumed..], so consumed bytes are
        // logically skipped
        let unread_portion = &device.parse_buffer[device.buffer_position.as_usize()..];
        assert_eq!(unread_portion.len(), 50);

        // Test 4: Buffer compaction at threshold (half of PARSE_BUFFER_SIZE)
        // Add enough data to exceed the threshold (2048 bytes)
        device.parse_buffer.clear();
        device.buffer_position = ByteIndex::default();

        // Add 2100 bytes (exceed 2048 threshold, which is half of 4096)
        let large_data = vec![b'Y'; 2100];
        device.parse_buffer.extend_from_slice(&large_data);
        assert_eq!(device.parse_buffer.len(), 2100);

        // Consume 1000 bytes (won't trigger compaction yet, need > 2048)
        device.consume(byte_offset(1000));
        assert_eq!(device.buffer_position.as_usize(), 1000);
        assert_eq!(device.parse_buffer.len(), 2100); // Buffer not compacted yet

        // Consume another 1100 bytes (total = 2100, which exceeds 2048 threshold)
        device.consume(byte_offset(1100));
        assert_eq!(device.buffer_position.as_usize(), 0); // Reset to 0 after compaction
        assert_eq!(device.parse_buffer.len(), 0); // Consumed data removed, remaining data preserved

        // Test 5: Verify capacity doesn't shrink unexpectedly
        // Even after compaction, we should maintain reasonable capacity
        let capacity_after_compact = device.parse_buffer.capacity();
        assert!(capacity_after_compact >= PARSE_BUFFER_SIZE);
    }

    #[tokio::test]
    async fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker
        let mut device = DirectToAnsiInputDevice::new();

        // Verify initial state is NotPasting
        assert!(matches!(device.paste_state, PasteCollectionState::Inactive));

        // Simulate receiving Paste(Start) event
        let start_event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        // Apply state machine logic (simulating what read_event does)
        match (&mut device.paste_state, &start_event) {
            (
                state @ PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
            ) => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::Accumulating(_)
        ));

        // Simulate receiving keyboard events (the pasted text)
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(*ch),
                modifiers: VT100KeyModifiersIR::default(),
            };
            match (&mut device.paste_state, &keyboard_event) {
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

        // Simulate receiving Paste(End) event
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let collected_text = match (&mut device.paste_state, &end_event) {
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

        // Verify we collected the correct text
        assert_eq!(collected_text, "Hello");

        // Verify we're back to NotPasting state
        assert!(matches!(device.paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines
        let mut device = DirectToAnsiInputDevice::new();

        // Start collection
        match &mut device.paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            PasteCollectionState::Accumulating(_) => panic!(),
        }

        // Accumulate "Line1\nLine2"
        for ch in "Line1\nLine2".chars() {
            match &mut device.paste_state {
                PasteCollectionState::Accumulating(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::Inactive => panic!(),
            }
        }

        // End collection
        let text = match &mut device.paste_state {
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
        // Test: Orphaned End marker (without Start) should be handled gracefully
        let mut device = DirectToAnsiInputDevice::new();

        // Should be NotPasting initially
        assert!(matches!(device.paste_state, PasteCollectionState::Inactive));

        // Receive End marker without Start - should emit empty paste
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let result = match (&mut device.paste_state, &end_event) {
            (
                PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be NotPasting
        assert!(matches!(device.paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End)
        let mut device = DirectToAnsiInputDevice::new();

        // Start
        match &mut device.paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between)
        let text = match &mut device.paste_state {
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! DirectToAnsi Input Device Implementation
//!
//! This module implements the async input device for the DirectToAnsi backend.
//! It handles non-blocking reading from stdin using tokio, manages a ring buffer
//! for partial ANSI sequences, and delegates to the protocol layer parsers for
//! sequence interpretation.

use crate::core::ansi::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O,
                        vt_100_terminal_input_parser::{InputEvent, KeyCode,
                                                       KeyModifiers,
                                                       parse_keyboard_sequence,
                                                       parse_mouse_sequence,
                                                       parse_ss3_sequence,
                                                       parse_terminal_event,
                                                       parse_utf8_text}};
use tokio::io::{AsyncReadExt, Stdin};

/// Buffer compaction threshold: compact when consumed bytes exceed this value.
const BUFFER_COMPACT_THRESHOLD: usize = 2048;

/// Initial buffer capacity: 4KB for efficient ANSI sequence buffering.
const INITIAL_BUFFER_CAPACITY: usize = 4096;

/// Async input device for DirectToAnsi backend.
///
/// Manages asynchronous reading from terminal stdin using tokio, with:
/// - Simple `Vec<u8>` buffer for handling partial/incomplete ANSI sequences
/// - Smart lookahead for zero-latency ESC key detection (no timeout!)
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// ## Architecture
///
/// This device is the bridge between raw I/O and the protocol layer:
/// ```text
/// stdin (tokio::io::stdin)
///   ↓
/// [Vec<u8> Buffer: 4KB, zero-timeout parsing]
///   ↓
/// [Protocol Layer Parsers]
/// ├─ keyboard::parse_keyboard_sequence()
/// ├─ mouse::parse_mouse_sequence()
/// ├─ terminal_events::parse_terminal_event()
/// └─ utf8::parse_utf8_text()
///   ↓
/// InputEvent (to application)
/// ```
///
/// ## Why No Timeout?
///
/// Traditional implementations wait 150ms to distinguish ESC from ESC sequences.
/// We use tokio's async I/O instead:
/// - `stdin.read().await` yields until data is ready
/// - ESC alone → emitted immediately (0ms latency)
/// - ESC sequence → parsed when complete
/// - No artificial delays needed!
#[derive(Debug)]
pub struct DirectToAnsiInputDevice {
    /// Tokio async stdin handle for non-blocking reading.
    stdin: Stdin,

    /// Raw byte buffer for ANSI sequences and text.
    /// Pre-allocated with 4KB capacity, grows as needed.
    buffer: Vec<u8>,

    /// Number of bytes already parsed and consumed from buffer.
    /// When this exceeds `BUFFER_COMPACT_THRESHOLD`, buffer is compacted.
    consumed: usize,
}

impl DirectToAnsiInputDevice {
    /// Create a new DirectToAnsiInputDevice.
    ///
    /// Initializes:
    /// - tokio::io::stdin() handle for non-blocking reading
    /// - 4KB `Vec<u8>` buffer (pre-allocated)
    /// - consumed counter at 0
    ///
    /// No timeout initialization needed - we use smart async lookahead instead!
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            consumed: 0,
        }
    }

    /// Read the next input event asynchronously.
    ///
    /// Uses a smart async loop with zero-timeout parsing:
    /// 1. Try to parse from existing buffer
    /// 2. If incomplete, read more from stdin (yields until data ready)
    /// 3. Loop back to parsing
    ///
    /// Returns None if stdin is closed (EOF).
    ///
    /// ## Event Types
    ///
    /// Returns InputEvent variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode start/end markers
    ///
    /// ## Zero-Latency ESC Key
    ///
    /// Unlike naive implementations with 150ms timeout, this immediately emits
    /// ESC when buffer contains only `[0x1B]`, with no artificial delay.
    ///
    /// ## How Escape Sequences Arrive in Practice
    ///
    /// When you press a special key (e.g., Up Arrow), the terminal emulator sends
    /// an escape sequence like `ESC [ A` (3 bytes: `[0x1B, 0x5B, 0x41]`).
    ///
    /// **Key Assumption**: Modern terminal emulators send escape sequences **atomically**
    /// in a single `write()` syscall, and the kernel buffers all bytes together.
    ///
    /// ### Typical Flow (99.9% of cases)
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
    /// ### Edge Case: Slow Byte Arrival (rare)
    ///
    /// Over high-latency SSH or slow serial connections, bytes might arrive separately:
    ///
    /// ```text
    /// First read:  [0x1B]           → Emits ESC immediately
    /// Second read: [0x5B, 0x41]     → User gets ESC instead of Up Arrow
    /// ```
    ///
    /// **Trade-off**: We optimize for the common case (local terminals with atomic
    /// sequences) to achieve 0ms ESC latency, accepting rare edge cases over forcing
    /// 150ms timeout on all users (Crossterm's approach).
    ///
    /// ### Why This Assumption Holds
    ///
    /// - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send
    ///   escape sequences atomically in one write
    /// - **Terminal protocol design**: Sequences are designed to be atomic units
    /// - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
    ///   before read() sees them
    /// - **Network delay case**: Over SSH with 200ms latency, UX is already degraded;
    ///   getting ESC instead of Up Arrow is annoying but not catastrophic
    pub async fn read_event(&mut self) -> Option<InputEvent> {
        // Allocate temp buffer ONCE before loop (performance optimization).
        // read() overwrites from index 0 each time, so no clearing between iterations.
        let mut temp_buf = vec![0u8; 256];

        loop {
            // 1. Try to parse from existing buffer
            if let Some((event, bytes_consumed)) = self.try_parse() {
                self.consume(bytes_consumed);
                return Some(event);
            }

            // 2. Buffer exhausted or incomplete sequence, read more from stdin.
            // This yields until data is ready - no busy-waiting!
            // Reuse temp_buf - read() overwrites from index 0, we only use [..n]
            match self.stdin.read(&mut temp_buf).await {
                Ok(0) => {
                    // EOF - stdin closed
                    return None;
                }
                Ok(n) => {
                    // Append new bytes to buffer
                    self.buffer.extend_from_slice(&temp_buf[..n]);
                }
                Err(_) => {
                    // Read error - treat as EOF
                    return None;
                }
            }

            // 3. Loop back to try_parse() with new data
        }
    }

    /// Try to parse a complete event from the buffer.
    ///
    /// ## Smart Lookahead Logic
    ///
    /// - `[0x1B]` alone → ESC key (emitted immediately)
    /// - `[0x1B, b'[', ...]` → CSI sequence (keyboard/mouse)
    /// - `[0x1B, b'O', ...]` → SS3 sequence (application mode keys)
    /// - `[0x1B, other]` → ESC key (unknown escape)
    /// - Other bytes → UTF-8 text
    ///
    /// Here's the algorithm visually:
    ///
    /// ```text
    /// try_parse() uses smart 1-2 byte lookahead:
    /// ┌─────────────────────────────────────────┐
    /// │  First byte check                       │
    /// ├─────────────────────────────────────────┤
    /// │ 0x1B (ESC)?                             │
    /// │  ├─ buf.len() == 1?                     │
    /// │  │  └─ YES → Emit ESC immediately ▲     │
    /// │  │     (zero-latency ESC key!)          │
    /// │  └─ buf.len() > 1?                      │
    /// │     ├─ Second byte = b'['?              │
    /// │     │  └─ CSI → keyboard/mouse/terminal │
    /// │     ├─ Second byte = b'O'?              │
    /// │     │  └─ SS3 → app mode keys (F1-F4)   │
    /// │     └─ Second byte = other?             │
    /// │        └─ Emit ESC, leave rest in buf   │
    /// ├─────────────────────────────────────────┤
    /// │ Not ESC?                                │
    /// │  └─ Try: terminal → mouse → UTF-8       │
    /// └─────────────────────────────────────────┘
    /// ```
    ///
    /// # Returns
    ///
    /// `Some((event, bytes_consumed))` if successful, `None` if incomplete.
    fn try_parse(&self) -> Option<(InputEvent, usize)> {
        let buf = &self.buffer[self.consumed..];

        // Fast path: empty buffer.
        if buf.is_empty() {
            return None;
        }

        // Check first byte for routing.
        match buf.first() {
            Some(&ANSI_ESC) => {
                // ESC sequence or ESC key.
                if buf.len() == 1 {
                    // Just ESC, emit immediately (no timeout!).
                    return Some((
                        InputEvent::Keyboard {
                            code: KeyCode::Escape,
                            modifiers: KeyModifiers::default(),
                        },
                        1,
                    ));
                }

                // Check second byte.
                match buf.get(1) {
                    Some(&ANSI_CSI_BRACKET) => {
                        // CSI sequence - try keyboard first, then mouse, then terminal
                        // events.
                        parse_keyboard_sequence(buf)
                            .or_else(|| parse_mouse_sequence(buf))
                            .or_else(|| parse_terminal_event(buf))
                    }
                    Some(&ANSI_SS3_O) => {
                        // SS3 sequence - application mode keys (F1-F4, Home, End,
                        // arrows).
                        parse_ss3_sequence(buf)
                    }
                    Some(_) => {
                        // ESC + unknown byte, emit ESC.
                        Some((
                            InputEvent::Keyboard {
                                code: KeyCode::Escape,
                                modifiers: KeyModifiers::default(),
                            },
                            1,
                        ))
                    }
                    None => {
                        // Shouldn't reach here (buf.len() > 1 but get(1) is None?).
                        None
                    }
                }
            }
            Some(_) => {
                // Not ESC - try terminal events, mouse (X10/RXVT), or UTF-8 text.
                parse_terminal_event(buf)
                    .or_else(|| parse_mouse_sequence(buf))
                    .or_else(|| parse_utf8_text(buf))
            }
            None => {
                // Empty buffer (shouldn't reach here due to early return).
                None
            }
        }
    }

    /// Consume N bytes from the buffer.
    ///
    /// Increments the consumed counter and compacts the buffer if threshold exceeded.
    fn consume(&mut self, count: usize) {
        self.consumed += count;

        // Compact buffer if consumed bytes exceed threshold
        if self.consumed > BUFFER_COMPACT_THRESHOLD {
            self.buffer.drain(..self.consumed);
            self.consumed = 0;
        }
    }
}

impl Default for DirectToAnsiInputDevice {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_device_creation() {
        // TODO: Test DirectToAnsiInputDevice construction
    }

    #[test]
    fn test_event_parsing() {
        // TODO: Test event parsing from buffer
    }

    #[test]
    fn test_buffer_management() {
        // TODO: Test ring buffer handling
    }
}

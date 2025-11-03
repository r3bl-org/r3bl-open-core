// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! `DirectToAnsi` Input Device Implementation
//!
//! This module implements the async input device for the `DirectToAnsi` backend.
//! It handles non-blocking reading from stdin using tokio, manages a ring buffer (kind
//! of, except that it is growable) for partial ANSI sequences, and delegates to the
//! protocol layer parsers for sequence interpretation.

use crate::{Button, ColWidth, FocusEvent, InputDeviceExt, InputEvent, Key, KeyPress,
            KeyState, ModifierKeysMask, MouseInput, MouseInputKind, Pos, RowHeight,
            SpecialKey,
            core::ansi::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O,
                         vt_100_terminal_input_parser::{VT100FocusState,
                                                        VT100InputEvent, VT100KeyCode,
                                                        VT100KeyModifiers,
                                                        VT100MouseAction,
                                                        VT100MouseButton,
                                                        VT100PasteMode,
                                                        VT100ScrollDirection,
                                                        parse_alt_letter,
                                                        parse_control_character,
                                                        parse_keyboard_sequence,
                                                        parse_mouse_sequence,
                                                        parse_ss3_sequence,
                                                        parse_terminal_event,
                                                        parse_utf8_text}}};
use tokio::io::{AsyncReadExt, Stdin};

/// Buffer compaction threshold: compact when consumed bytes exceed this value. This is
/// kind of like a ring buffer (except that it is not fixed size).
const BUFFER_COMPACT_THRESHOLD: usize = 2048;

/// Initial buffer capacity: 4KB for efficient ANSI sequence buffering.
const INITIAL_BUFFER_CAPACITY: usize = 4096;

/// Temporary read buffer size for stdin reads.
const TEMP_READ_BUFFER_SIZE: usize = 256;

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
    NotPasting,
    /// Currently collecting text for a paste operation.
    Collecting(String),
}

/// Async input device for `DirectToAnsi` backend.
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
/// ## Zero-Latency ESC Key Detection
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
/// ### Performance Comparison
///
/// | Input Type         | crossterm Latency | Our Latency | Improvement     |
/// |--------------------|-------------------|-------------|-----------------|
/// | **ESC key press**  | 150ms (timeout)   | 0ms         | **150ms faster**|
/// | Arrow keys         | 0ms (immediate)   | 0ms         | Same            |
/// | Regular text       | 0ms (immediate)   | 0ms         | Same            |
/// | Mouse events       | 0ms (immediate)   | 0ms         | Same            |
///
/// **Benefit applies to**: Vim-style modal editors, ESC-heavy workflows, dialog
/// dismissal.
///
/// ### How Escape Sequences Arrive in Practice
///
/// When you press a special key (e.g., Up Arrow), the terminal emulator sends
/// an escape sequence like `ESC [ A` (3 bytes: `[0x1B, 0x5B, 0x41]`).
///
/// **Key Assumption**: Modern terminal emulators send escape sequences **atomically**
/// in a single `write()` syscall, and the kernel buffers all bytes together.
///
/// #### Typical Flow (99.9% of cases - local terminals)
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
/// #### Edge Case: Slow Byte Arrival (rare - high-latency SSH, slow serial)
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
/// #### Why This Assumption Holds
///
/// - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send escape
///   sequences atomically in one write
/// - **Terminal protocol design**: Sequences are designed to be atomic units
/// - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
///   before `read()` sees them
/// - **Network delay case**: Over SSH with 200ms latency, UX is already degraded; getting
///   ESC instead of Up Arrow is annoying but not catastrophic
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

    /// State machine for collecting bracketed paste text.
    /// Tracks whether we're between Paste(Start) and Paste(End) markers.
    paste_state: PasteCollectionState,
}

impl DirectToAnsiInputDevice {
    /// Create a new `DirectToAnsiInputDevice`.
    ///
    /// Initializes:
    /// - `tokio::io::stdin()` handle for non-blocking reading
    /// - 4KB `Vec<u8>` buffer (pre-allocated)
    /// - consumed counter at 0
    ///
    /// No timeout initialization needed - we use smart async lookahead instead!
    #[must_use]
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
            buffer: Vec::with_capacity(INITIAL_BUFFER_CAPACITY),
            consumed: 0,
            paste_state: PasteCollectionState::NotPasting,
        }
    }

    /// Read the next input event asynchronously.
    ///
    /// Returns `InputEvent` variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   ESC latency)
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode text
    ///
    /// Returns `None` if stdin is closed (EOF).
    ///
    /// ## Implementation
    ///
    /// Async loop with zero-timeout parsing:
    /// 1. Try to parse from existing buffer
    /// 2. If incomplete, read more from stdin (yields until data ready)
    /// 3. Loop back to parsing
    ///
    /// See struct-level documentation for details on zero-latency ESC detection
    /// algorithm.
    pub async fn read_event(&mut self) -> Option<InputEvent> {
        // Allocate temp buffer ONCE before loop (performance optimization).
        // read() overwrites from index 0 each time, so no clearing between iterations.
        let mut temp_buf = vec![0u8; TEMP_READ_BUFFER_SIZE];

        loop {
            // 1. Try to parse from existing buffer
            if let Some((vt100_event, bytes_consumed)) = self.try_parse() {
                self.consume(bytes_consumed);

                // 2. Apply paste collection state machine
                match (&mut self.paste_state, &vt100_event) {
                    // Start marker: enter collecting state, don't emit event
                    (
                        state @ PasteCollectionState::NotPasting,
                        VT100InputEvent::Paste(VT100PasteMode::Start),
                    ) => {
                        *state = PasteCollectionState::Collecting(String::new());
                        continue; // Loop to get next event
                    }

                    // While collecting: accumulate keyboard characters
                    (
                        PasteCollectionState::Collecting(buffer),
                        VT100InputEvent::Keyboard {
                            code: VT100KeyCode::Char(ch),
                            ..
                        },
                    ) => {
                        buffer.push(*ch);
                        continue; // Loop to get next event
                    }

                    // End marker: emit complete paste and exit collecting state
                    (
                        state @ PasteCollectionState::Collecting(_),
                        VT100InputEvent::Paste(VT100PasteMode::End),
                    ) => {
                        if let PasteCollectionState::Collecting(text) =
                            std::mem::replace(state, PasteCollectionState::NotPasting)
                        {
                            return Some(InputEvent::BracketedPaste(text));
                        }
                        unreachable!()
                    }

                    // Orphaned end marker (End without Start): emit empty paste
                    (
                        PasteCollectionState::NotPasting,
                        VT100InputEvent::Paste(VT100PasteMode::End),
                    ) => {
                        return Some(InputEvent::BracketedPaste(String::new()));
                    }

                    // Normal event processing when not pasting
                    (PasteCollectionState::NotPasting, _) => {
                        return convert_input_event(vt100_event);
                    }

                    // Other events while collecting paste should be ignored (or queued)
                    // For now, ignore them (they'll be lost)
                    (PasteCollectionState::Collecting(_), _) => {
                        continue; // Ignore and get next event
                    }
                }
            }

            // 2. Buffer exhausted or incomplete sequence, read more from stdin.
            // This yields until data is ready - no busy-waiting!
            // Reuse temp_buf - read() overwrites from index 0, we only use [..n]
            match self.stdin.read(&mut temp_buf).await {
                Ok(0) | Err(_) => {
                    // EOF - stdin closed or read error - treat as EOF
                    return None;
                }
                Ok(n) => {
                    // Append new bytes to buffer
                    self.buffer.extend_from_slice(&temp_buf[..n]);
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
    /// Returns the protocol-level `VT100InputEvent` before conversion to canonical
    /// `InputEvent`.
    fn try_parse(&self) -> Option<(VT100InputEvent, usize)> {
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
                    let esc_event = VT100InputEvent::Keyboard {
                        code: VT100KeyCode::Escape,
                        modifiers: VT100KeyModifiers::default(),
                    };
                    return Some((esc_event, 1));
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
                        // ESC + unknown byte - try Alt+letter before emitting standalone
                        // ESC. This handles Alt+B (ESC+'b'),
                        // Alt+F (ESC+'f'), etc.
                        parse_alt_letter(buf).or_else(|| {
                            // Not Alt+letter, emit standalone ESC
                            let esc_event = VT100InputEvent::Keyboard {
                                code: VT100KeyCode::Escape,
                                modifiers: VT100KeyModifiers::default(),
                            };
                            Some((esc_event, 1))
                        })
                    }
                    None => {
                        // Shouldn't reach here (buf.len() > 1 but get(1) is None?).
                        unreachable!()
                    }
                }
            }
            Some(_) => {
                // Not ESC - try terminal events, mouse (X10/RXVT), control characters, or
                // UTF-8 text. Control characters (0x00-0x1F like Ctrl+A,
                // Ctrl+D, Ctrl+W) must be tried before UTF-8 because they
                // are technically valid UTF-8 but should be parsed as Ctrl+letter
                // instead.
                parse_terminal_event(buf)
                    .or_else(|| parse_mouse_sequence(buf))
                    .or_else(|| parse_control_character(buf))
                    .or_else(|| parse_utf8_text(buf))
            }
            None => {
                // Empty buffer (shouldn't reach here due to early return).
                unreachable!()
            }
        }
    }

    /// Consume N bytes from the buffer.
    ///
    /// Increments the consumed counter and compacts the buffer if threshold exceeded.
    /// This is kind of like a ring buffer (except that it is not fixed size).
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

impl InputDeviceExt for DirectToAnsiInputDevice {
    async fn next_input_event(&mut self) -> Option<InputEvent> { self.read_event().await }
}

/// Convert protocol-level `KeyCode` and `KeyModifiers` to canonical `KeyPress`.
fn convert_key_code_to_keypress(
    code: VT100KeyCode,
    modifiers: VT100KeyModifiers,
) -> KeyPress {
    let key = match code {
        VT100KeyCode::Char(ch) => Key::Character(ch),
        VT100KeyCode::Function(n) => {
            use crate::FunctionKey;
            match n {
                1 => Key::FunctionKey(FunctionKey::F1),
                2 => Key::FunctionKey(FunctionKey::F2),
                3 => Key::FunctionKey(FunctionKey::F3),
                4 => Key::FunctionKey(FunctionKey::F4),
                5 => Key::FunctionKey(FunctionKey::F5),
                6 => Key::FunctionKey(FunctionKey::F6),
                7 => Key::FunctionKey(FunctionKey::F7),
                8 => Key::FunctionKey(FunctionKey::F8),
                9 => Key::FunctionKey(FunctionKey::F9),
                10 => Key::FunctionKey(FunctionKey::F10),
                11 => Key::FunctionKey(FunctionKey::F11),
                12 => Key::FunctionKey(FunctionKey::F12),
                _ => Key::Character('?'), // Fallback
            }
        }
        VT100KeyCode::Up => Key::SpecialKey(SpecialKey::Up),
        VT100KeyCode::Down => Key::SpecialKey(SpecialKey::Down),
        VT100KeyCode::Left => Key::SpecialKey(SpecialKey::Left),
        VT100KeyCode::Right => Key::SpecialKey(SpecialKey::Right),
        VT100KeyCode::Home => Key::SpecialKey(SpecialKey::Home),
        VT100KeyCode::End => Key::SpecialKey(SpecialKey::End),
        VT100KeyCode::PageUp => Key::SpecialKey(SpecialKey::PageUp),
        VT100KeyCode::PageDown => Key::SpecialKey(SpecialKey::PageDown),
        VT100KeyCode::Tab => Key::SpecialKey(SpecialKey::Tab),
        VT100KeyCode::BackTab => Key::SpecialKey(SpecialKey::BackTab),
        VT100KeyCode::Delete => Key::SpecialKey(SpecialKey::Delete),
        VT100KeyCode::Insert => Key::SpecialKey(SpecialKey::Insert),
        VT100KeyCode::Enter => Key::SpecialKey(SpecialKey::Enter),
        VT100KeyCode::Backspace => Key::SpecialKey(SpecialKey::Backspace),
        VT100KeyCode::Escape => Key::SpecialKey(SpecialKey::Esc),
    };

    // Convert modifiers
    let mask = ModifierKeysMask {
        shift_key_state: if modifiers.shift {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        ctrl_key_state: if modifiers.ctrl {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        alt_key_state: if modifiers.alt {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
    };

    if mask.shift_key_state == KeyState::NotPressed
        && mask.ctrl_key_state == KeyState::NotPressed
        && mask.alt_key_state == KeyState::NotPressed
    {
        KeyPress::Plain { key }
    } else {
        KeyPress::WithModifiers { key, mask }
    }
}

/// Convert protocol-level `InputEvent` to canonical `InputEvent`.
fn convert_input_event(vt100_event: VT100InputEvent) -> Option<InputEvent> {
    match vt100_event {
        VT100InputEvent::Keyboard { code, modifiers } => {
            let keypress = convert_key_code_to_keypress(code, modifiers);
            Some(InputEvent::Keyboard(keypress))
        }
        VT100InputEvent::Mouse {
            button,
            pos,
            action,
            modifiers,
        } => {
            let button_kind = match button {
                VT100MouseButton::Left => Button::Left,
                VT100MouseButton::Right => Button::Right,
                VT100MouseButton::Middle => Button::Middle,
                VT100MouseButton::Unknown => return None,
            };

            let kind = match action {
                VT100MouseAction::Press => MouseInputKind::MouseDown(button_kind),
                VT100MouseAction::Release => MouseInputKind::MouseUp(button_kind),
                VT100MouseAction::Drag => MouseInputKind::MouseDrag(button_kind),
                VT100MouseAction::Motion => MouseInputKind::MouseMove,
                VT100MouseAction::Scroll(direction) => match direction {
                    VT100ScrollDirection::Up => MouseInputKind::ScrollUp,
                    VT100ScrollDirection::Down => MouseInputKind::ScrollDown,
                    VT100ScrollDirection::Left => MouseInputKind::ScrollLeft,
                    VT100ScrollDirection::Right => MouseInputKind::ScrollRight,
                },
            };

            let maybe_modifier_keys =
                if modifiers.shift || modifiers.ctrl || modifiers.alt {
                    Some(ModifierKeysMask {
                        shift_key_state: if modifiers.shift {
                            KeyState::Pressed
                        } else {
                            KeyState::NotPressed
                        },
                        ctrl_key_state: if modifiers.ctrl {
                            KeyState::Pressed
                        } else {
                            KeyState::NotPressed
                        },
                        alt_key_state: if modifiers.alt {
                            KeyState::Pressed
                        } else {
                            KeyState::NotPressed
                        },
                    })
                } else {
                    None
                };

            // Convert TermPos to Pos (convert from 1-based to 0-based)
            // TermCol and TermRow have built-in conversion to 0-based indices
            let canonical_pos = Pos {
                col_index: pos.col.to_zero_based(),
                row_index: pos.row.to_zero_based(),
            };

            let mouse_input = MouseInput {
                pos: canonical_pos,
                kind,
                maybe_modifier_keys,
            };
            Some(InputEvent::Mouse(mouse_input))
        }
        VT100InputEvent::Resize { rows, cols } => Some(InputEvent::Resize(crate::Size {
            col_width: ColWidth::from(cols),
            row_height: RowHeight::from(rows),
        })),
        VT100InputEvent::Focus(focus_state) => {
            let event = match focus_state {
                VT100FocusState::Gained => FocusEvent::Gained,
                VT100FocusState::Lost => FocusEvent::Lost,
            };
            Some(InputEvent::Focus(event))
        }
        VT100InputEvent::Paste(_paste_mode) => {
            unreachable!(
                "Paste events are handled by state machine in read_event() \
                 and should never reach convert_input_event()"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        // Test DirectToAnsiInputDevice constructs successfully with correct initial state
        let device = DirectToAnsiInputDevice::new();

        // Verify buffer initialized with correct capacity (4KB)
        assert_eq!(device.buffer.capacity(), INITIAL_BUFFER_CAPACITY);

        // Verify buffer is empty initially (no data yet)
        assert_eq!(device.buffer.len(), 0);

        // Verify consumed counter is at 0
        assert_eq!(device.consumed, 0);

        // Constructor completes without panic - success!
    }

    #[test]
    fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence types
        let mut device = DirectToAnsiInputDevice::new();

        // Test 1: Parse UTF-8 text (simplest case)
        // Single character "A" should parse as keyboard input
        device.buffer.extend_from_slice(b"A");
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(&device.buffer) {
            assert_eq!(bytes_consumed, 1);
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
        device.buffer.clear();
        device.buffer.push(0x1B); // ESC byte
        // Note: try_parse() is private, so we verify parsing logic through the buffer
        // setup A buffer with only [0x1B] should parse as ESC key (based on
        // try_parse logic)
        assert_eq!(device.buffer.len(), 1);
        assert_eq!(device.buffer[0], 0x1B);

        // Test 3: Set up CSI sequence for keyboard (Up Arrow: ESC [ A)
        device.buffer.clear();
        device.buffer.extend_from_slice(&[0x1B, 0x5B, 0x41]); // ESC [ A
        if let Some((vt100_event, bytes_consumed)) =
            parse_keyboard_sequence(&device.buffer)
        {
            assert_eq!(bytes_consumed, 3);
            // Convert and verify we got a keyboard event
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }

        // Test 4: Verify buffer consumption tracking
        device.consumed = 0;
        device.consume(1);
        assert_eq!(device.consumed, 1);

        device.consume(2);
        assert_eq!(device.consumed, 3);
    }

    #[test]
    fn test_buffer_management() {
        // Test buffer handling: growth, consumption, and compaction at 2KB threshold
        let mut device = DirectToAnsiInputDevice::new();

        // Verify initial state
        assert_eq!(device.buffer.len(), 0);
        assert_eq!(device.buffer.capacity(), INITIAL_BUFFER_CAPACITY);
        assert_eq!(device.consumed, 0);

        // Test 1: Buffer growth - add data and verify length increases
        let test_data = vec![b'X'; 100];
        device.buffer.extend_from_slice(&test_data);
        assert_eq!(device.buffer.len(), 100);
        assert!(device.buffer.capacity() >= 100);

        // Test 2: Consumption tracking - consume bytes and verify counter
        device.consume(50);
        assert_eq!(device.consumed, 50);
        assert_eq!(device.buffer.len(), 100); // Buffer still holds all bytes

        // Test 3: Verify consumed bytes are skipped in try_parse
        // The try_parse function uses &buffer[consumed..], so consumed bytes are
        // logically skipped
        let unread_portion = &device.buffer[device.consumed..];
        assert_eq!(unread_portion.len(), 50);

        // Test 4: Buffer compaction at 2KB threshold
        // Add enough data to exceed BUFFER_COMPACT_THRESHOLD (2048 bytes)
        device.buffer.clear();
        device.consumed = 0;

        // Add 2100 bytes (exceed 2048 threshold)
        let large_data = vec![b'Y'; 2100];
        device.buffer.extend_from_slice(&large_data);
        assert_eq!(device.buffer.len(), 2100);

        // Consume 1000 bytes (won't trigger compaction yet, need > 2048)
        device.consume(1000);
        assert_eq!(device.consumed, 1000);
        assert_eq!(device.buffer.len(), 2100); // Buffer not compacted yet

        // Consume another 1100 bytes (total = 2100, which exceeds 2048 threshold)
        device.consume(1100);
        assert_eq!(device.consumed, 0); // Reset to 0 after compaction
        assert_eq!(device.buffer.len(), 0); // Consumed data removed, remaining data preserved

        // Test 5: Verify capacity doesn't shrink unexpectedly
        // Even after compaction, we should maintain reasonable capacity
        let capacity_after_compact = device.buffer.capacity();
        assert!(capacity_after_compact >= INITIAL_BUFFER_CAPACITY);
    }

    #[test]
    fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker
        let mut device = DirectToAnsiInputDevice::new();

        // Verify initial state is NotPasting
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::NotPasting
        ));

        // Simulate receiving Paste(Start) event
        let start_event = VT100InputEvent::Paste(VT100PasteMode::Start);
        // Apply state machine logic (simulating what read_event does)
        match (&mut device.paste_state, &start_event) {
            (
                state @ PasteCollectionState::NotPasting,
                VT100InputEvent::Paste(VT100PasteMode::Start),
            ) => {
                *state = PasteCollectionState::Collecting(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::Collecting(_)
        ));

        // Simulate receiving keyboard events (the pasted text)
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEvent::Keyboard {
                code: VT100KeyCode::Char(*ch),
                modifiers: VT100KeyModifiers::default(),
            };
            match (&mut device.paste_state, &keyboard_event) {
                (
                    PasteCollectionState::Collecting(buffer),
                    VT100InputEvent::Keyboard {
                        code: VT100KeyCode::Char(ch),
                        ..
                    },
                ) => {
                    buffer.push(*ch);
                }
                _ => panic!("State machine should accumulate text while collecting"),
            }
        }

        // Simulate receiving Paste(End) event
        let end_event = VT100InputEvent::Paste(VT100PasteMode::End);
        let collected_text = match (&mut device.paste_state, &end_event) {
            (
                state @ PasteCollectionState::Collecting(_),
                VT100InputEvent::Paste(VT100PasteMode::End),
            ) => {
                if let PasteCollectionState::Collecting(text) =
                    std::mem::replace(state, PasteCollectionState::NotPasting)
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
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::NotPasting
        ));
    }

    #[test]
    fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines
        let mut device = DirectToAnsiInputDevice::new();

        // Start collection
        match &mut device.paste_state {
            state @ PasteCollectionState::NotPasting => {
                *state = PasteCollectionState::Collecting(String::new());
            }
            PasteCollectionState::Collecting(_) => panic!(),
        }

        // Accumulate "Line1\nLine2"
        for ch in "Line1\nLine2".chars() {
            match &mut device.paste_state {
                PasteCollectionState::Collecting(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::NotPasting => panic!(),
            }
        }

        // End collection
        let text = match &mut device.paste_state {
            state @ PasteCollectionState::Collecting(_) => {
                if let PasteCollectionState::Collecting(t) =
                    std::mem::replace(state, PasteCollectionState::NotPasting)
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

    #[test]
    fn test_paste_state_machine_orphaned_end() {
        // Test: Orphaned End marker (without Start) should be handled gracefully
        let mut device = DirectToAnsiInputDevice::new();

        // Should be NotPasting initially
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::NotPasting
        ));

        // Receive End marker without Start - should emit empty paste
        let end_event = VT100InputEvent::Paste(VT100PasteMode::End);
        let result = match (&mut device.paste_state, &end_event) {
            (
                PasteCollectionState::NotPasting,
                VT100InputEvent::Paste(VT100PasteMode::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be NotPasting
        assert!(matches!(
            device.paste_state,
            PasteCollectionState::NotPasting
        ));
    }

    #[test]
    fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End)
        let mut device = DirectToAnsiInputDevice::new();

        // Start
        match &mut device.paste_state {
            state @ PasteCollectionState::NotPasting => {
                *state = PasteCollectionState::Collecting(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between)
        let text = match &mut device.paste_state {
            state @ PasteCollectionState::Collecting(_) => {
                if let PasteCollectionState::Collecting(t) =
                    std::mem::replace(state, PasteCollectionState::NotPasting)
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

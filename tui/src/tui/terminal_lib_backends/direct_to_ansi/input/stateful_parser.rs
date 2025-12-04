// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes.
//!
//! This module provides the [`Parser`] struct that accumulates bytes and parses them
//! into [`VT100InputEventIR`] events using the `more` flag for ESC disambiguation.

use crate::core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR, try_parse_input_event};
use std::collections::VecDeque;

/// Stateful parser for terminal input bytes.
///
/// Accumulates bytes and parses them into [`VT100InputEventIR`] events using the
/// `more` flag for ESC disambiguation:
///
/// - `more = true`: More bytes might be coming, wait before deciding
/// - `more = false`: No more bytes available, a lone ESC is the ESC key
///
/// This works because if `read()` fills the entire buffer, more data is likely
/// waiting; if it returns fewer bytes, we've drained all available input.
#[derive(Debug)]
pub struct Parser {
    /// Accumulator for current ANSI escape sequence being parsed (capacity: 256
    /// bytes).
    buffer: Vec<u8>,

    /// Queue of parsed events ready to be consumed (capacity: 128).
    internal_events: VecDeque<VT100InputEventIR>,
}

impl Default for Parser {
    fn default() -> Self {
        Parser {
            buffer: Vec::with_capacity(256),
            internal_events: VecDeque::with_capacity(128),
        }
    }
}

impl Parser {
    /// Process incoming bytes and parse into events.
    ///
    /// - `buffer`: Raw bytes read from `stdin`.
    /// - `more`: Whether more data is likely available (`read_count ==
    ///   TTY_BUFFER_SIZE`).
    pub fn advance(&mut self, buffer: &[u8], more: bool) {
        for (idx, byte) in buffer.iter().enumerate() {
            // Recompute `more` for each byte:
            // - true if more bytes remain in current chunk, OR
            // - true if original read() filled the buffer (more data likely waiting)
            let more = idx + 1 < buffer.len() || more;

            self.buffer.push(*byte);

            match try_parse_input_event(&self.buffer, more) {
                Some((event, _bytes_consumed)) => {
                    // Successfully parsed - push event and clear buffer.
                    self.internal_events.push_back(event);
                    self.buffer.clear();
                }
                None => {
                    // Incomplete sequence or waiting for more bytes.
                    // Keep buffer and continue accumulating.
                }
            }
        }
    }
}

impl Iterator for Parser {
    type Item = VT100InputEventIR;

    fn next(&mut self) -> Option<Self::Item> { self.internal_events.pop_front() }
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                          VT100KeyCodeIR,
                                                          VT100KeyModifiersIR};

    /// Helper to create a keyboard event for assertions.
    fn keyboard_event(code: VT100KeyCodeIR) -> VT100InputEventIR {
        VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        }
    }

    mod basic_parsing {
        use super::*;

        #[test]
        fn single_ascii_char() {
            let mut parser = Parser::default();
            parser.advance(b"a", false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
        }

        #[test]
        fn multiple_ascii_chars_single_read() {
            let mut parser = Parser::default();
            parser.advance(b"abc", false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 3);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
            assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
            assert_eq!(events[2], keyboard_event(VT100KeyCodeIR::Char('c')));
        }

        #[test]
        fn enter_key() {
            let mut parser = Parser::default();
            parser.advance(b"\r", false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Enter));
        }

        #[test]
        fn tab_key() {
            let mut parser = Parser::default();
            parser.advance(b"\t", false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Tab));
        }

        #[test]
        fn backspace_key() {
            let mut parser = Parser::default();
            // Backspace is typically 0x7F (127)
            parser.advance(&[0x7F], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Backspace));
        }
    }

    mod esc_disambiguation {
        //! Tests for the core ESC disambiguation logic using the `more` flag.
        //!
        //! The `more` flag indicates whether additional bytes are likely waiting
        //! in the kernel buffer. When `more=true`, ESC (0x1B) is treated as the
        //! start of an escape sequence. When `more=false`, it's a standalone ESC
        //! key press.

        use super::*;

        #[test]
        fn lone_esc_with_more_false_emits_escape_key() {
            // User pressed ESC key alone - no more data coming.
            let mut parser = Parser::default();
            parser.advance(&[0x1B], false); // ESC byte, more=false

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Escape));
        }

        #[test]
        fn esc_with_more_true_waits_for_sequence() {
            // ESC arrived but more bytes are coming - wait for full sequence.
            let mut parser = Parser::default();
            parser.advance(&[0x1B], true); // ESC byte, more=true

            // No event emitted yet - waiting for rest of sequence.
            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 0);
        }

        #[test]
        fn arrow_up_complete_sequence() {
            // Arrow Up: ESC [ A (0x1B 0x5B 0x41)
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'A'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
        }

        #[test]
        fn arrow_down_complete_sequence() {
            // Arrow Down: ESC [ B
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'B'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Down));
        }

        #[test]
        fn arrow_right_complete_sequence() {
            // Arrow Right: ESC [ C
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'C'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Right));
        }

        #[test]
        fn arrow_left_complete_sequence() {
            // Arrow Left: ESC [ D
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'D'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Left));
        }
    }

    mod chunked_input {
        //! Tests for input arriving in multiple chunks (simulating slow network
        //! or read() returning partial data).

        use super::*;

        #[test]
        fn arrow_key_split_across_two_reads() {
            // Arrow Up arrives as: first read gets ESC, second read gets [ A
            let mut parser = Parser::default();

            // First chunk: ESC only, but more=true (buffer was full)
            parser.advance(&[0x1B], true);
            assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0); // No event yet

            // Second chunk: [ A completes the sequence
            parser.advance(&[b'[', b'A'], false);
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
        }

        #[test]
        fn arrow_key_split_into_three_reads() {
            // Extreme fragmentation: ESC, then [, then A
            let mut parser = Parser::default();

            parser.advance(&[0x1B], true);
            assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

            parser.advance(&[b'['], true);
            assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

            parser.advance(&[b'A'], false);
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
        }

        #[test]
        fn multiple_events_across_chunks() {
            let mut parser = Parser::default();

            // First chunk: 'a' and start of arrow sequence
            parser.advance(&[b'a', 0x1B], true);
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));

            // Second chunk: completes arrow, adds 'b'
            parser.advance(&[b'[', b'A', b'b'], false);
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
            assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
        }
    }

    mod iterator_impl {
        use super::*;

        #[test]
        fn iterator_drains_internal_queue() {
            let mut parser = Parser::default();
            parser.advance(b"xyz", false);

            // First iteration drains the queue.
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 3);

            // Second iteration returns empty - queue is drained.
            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 0);
        }

        #[test]
        fn iterator_returns_events_in_fifo_order() {
            let mut parser = Parser::default();
            parser.advance(b"abc", false);

            assert_eq!(
                parser.next(),
                Some(keyboard_event(VT100KeyCodeIR::Char('a')))
            );
            assert_eq!(
                parser.next(),
                Some(keyboard_event(VT100KeyCodeIR::Char('b')))
            );
            assert_eq!(
                parser.next(),
                Some(keyboard_event(VT100KeyCodeIR::Char('c')))
            );
            assert_eq!(parser.next(), None);
        }

        #[test]
        fn can_interleave_advance_and_iteration() {
            let mut parser = Parser::default();

            parser.advance(b"a", false);
            assert_eq!(
                parser.next(),
                Some(keyboard_event(VT100KeyCodeIR::Char('a')))
            );

            parser.advance(b"b", false);
            assert_eq!(
                parser.next(),
                Some(keyboard_event(VT100KeyCodeIR::Char('b')))
            );

            assert_eq!(parser.next(), None);
        }
    }

    mod special_keys {
        use super::*;

        #[test]
        fn home_key() {
            // Home: ESC [ H
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'H'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Home));
        }

        #[test]
        fn end_key() {
            // End: ESC [ F
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'F'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::End));
        }

        #[test]
        fn delete_key() {
            // Delete: ESC [ 3 ~
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'3', b'~'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Delete));
        }

        #[test]
        fn insert_key() {
            // Insert: ESC [ 2 ~
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'2', b'~'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Insert));
        }

        #[test]
        fn page_up_key() {
            // Page Up: ESC [ 5 ~
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'5', b'~'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageUp));
        }

        #[test]
        fn page_down_key() {
            // Page Down: ESC [ 6 ~
            let mut parser = Parser::default();
            parser.advance(&[0x1B, b'[', b'6', b'~'], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageDown));
        }
    }

    mod utf8_input {
        use super::*;

        #[test]
        fn two_byte_utf8_char() {
            // 'é' is U+00E9, encoded as 0xC3 0xA9
            let mut parser = Parser::default();
            parser.advance(&[0xC3, 0xA9], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
        }

        #[test]
        fn three_byte_utf8_char() {
            // '中' is U+4E2D, encoded as 0xE4 0xB8 0xAD
            let mut parser = Parser::default();
            parser.advance(&[0xE4, 0xB8, 0xAD], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('中')));
        }

        #[test]
        fn four_byte_utf8_emoji() {
            // '😀' is U+1F600, encoded as 0xF0 0x9F 0x98 0x80
            let mut parser = Parser::default();
            parser.advance(&[0xF0, 0x9F, 0x98, 0x80], false);

            let events: Vec<_> = parser.collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('😀')));
        }

        #[test]
        fn utf8_split_across_chunks() {
            // 'é' split across two reads
            let mut parser = Parser::default();

            parser.advance(&[0xC3], true);
            assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

            parser.advance(&[0xA9], false);
            let events: Vec<_> = (&mut parser).collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
        }
    }
}

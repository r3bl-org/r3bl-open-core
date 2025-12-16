// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes. See [`StatefulInputParser`] docs.

use crate::core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                      try_parse_input_event};
use std::collections::VecDeque;

/// Stateful parser for terminal input bytes.
///
/// Accumulates bytes and parses them into [`VT100InputEventIR`] events using the
/// `more` flag for `ESC` disambiguation (see [ESC Detection Limitations]):
///
/// - `more = true`: More bytes might be coming, wait before deciding
/// - `more = false`: No more bytes available, a lone `ESC` is the `ESC` key
///
/// This works because if [`read()`] fills the entire buffer, more data is likely
/// waiting; if it returns fewer bytes, we've drained all available input.
///
/// [ESC Detection Limitations]: mio_poller::MioPollerThread#esc-detection-limitations
/// [`read()`]: std::io::Read::read
#[derive(Debug)]
pub struct StatefulInputParser {
    /// Accumulator for current ANSI escape sequence being parsed (capacity: 256
    /// bytes).
    buffer: Vec<u8>,

    /// Queue of parsed events ready to be consumed (capacity: 128).
    internal_events: VecDeque<VT100InputEventIR>,
}

impl Default for StatefulInputParser {
    fn default() -> Self {
        StatefulInputParser {
            buffer: Vec::with_capacity(256),
            internal_events: VecDeque::with_capacity(128),
        }
    }
}

impl StatefulInputParser {
    /// Process incoming bytes and parse into events.
    /// - `buffer`: Raw bytes read from `stdin`.
    /// - `more`: Whether more data is likely available (`read_count == TTY_BUFFER_SIZE`).
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

impl Iterator for StatefulInputParser {
    type Item = VT100InputEventIR;

    fn next(&mut self) -> Option<Self::Item> { self.internal_events.pop_front() }
}

/// Shared test helpers and imports for [`StatefulInputParser`] tests.
#[cfg(test)]
mod test_fixtures {
    pub use super::StatefulInputParser;
    pub use crate::{core::ansi::{generator::{SEQ_ARROW_DOWN, SEQ_ARROW_LEFT,
                                             SEQ_ARROW_RIGHT, SEQ_ARROW_UP},
                                 vt_100_terminal_input_parser::{VT100InputEventIR,
                                                                VT100KeyCodeIR,
                                                                VT100KeyModifiersIR}},
                    input_sequences::{ANSI_ESC, ASCII_DEL}};

    /// Helper to create a keyboard event for assertions.
    pub fn keyboard_event(code: VT100KeyCodeIR) -> VT100InputEventIR {
        VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        }
    }
}

#[cfg(test)]
mod tests_basic_parsing {
    use super::test_fixtures::*;

    #[test]
    fn single_ascii_char() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"a", false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    }

    #[test]
    fn multiple_ascii_chars_single_read() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"abc", false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
        assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
        assert_eq!(events[2], keyboard_event(VT100KeyCodeIR::Char('c')));
    }

    #[test]
    fn enter_key() {
        let mut parser = StatefulInputParser::default();
        // In raw mode, Enter sends CR (0D), not LF (0A).
        // The kernel's line discipline translates CR→LF, but raw mode bypasses this.
        parser.advance(b"\r", false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Enter));
    }

    #[test]
    fn tab_key() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\t", false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Tab));
    }

    #[test]
    fn backspace_key() {
        let mut parser = StatefulInputParser::default();
        // Historical quirk: Backspace key sends DEL (7F), not BS (08).
        // DEC VT100 reserved BS for cursor-left; most terminals inherited this.
        parser.advance(&[ASCII_DEL], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Backspace));
    }
}

/// Tests for the core ESC disambiguation logic using the `more` flag.
///
/// The `more` flag indicates whether additional bytes are likely waiting
/// in the kernel buffer.
/// - When `more=true`, `ESC` (1B) is treated as the start of an escape sequence.
/// - When `more=false`, it's a standalone `ESC` key press.
#[cfg(test)]
mod tests_esc_disambiguation {
    use super::test_fixtures::*;

    #[test]
    fn lone_esc_with_more_false_emits_escape_key() {
        // User pressed ESC key alone - no more data coming.
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC], false); // ESC byte, more=false

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Escape));
    }

    #[test]
    fn esc_with_more_true_waits_for_sequence() {
        // ESC arrived but more bytes are coming - wait for full sequence.
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC], true); // ESC byte, more=true

        // No event emitted yet - waiting for rest of sequence.
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn arrow_up_complete_sequence() {
        // Arrow Up: ESC [ A
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_UP, false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_down_complete_sequence() {
        // Arrow Down: ESC [ B
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_DOWN, false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Down));
    }

    #[test]
    fn arrow_right_complete_sequence() {
        // Arrow Right: ESC [ C
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_RIGHT, false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Right));
    }

    #[test]
    fn arrow_left_complete_sequence() {
        // Arrow Left: ESC [ D
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_LEFT, false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Left));
    }
}

#[cfg(test)]
mod tests_chunked_input {
    //! Tests for input arriving in multiple chunks (simulating slow network
    //! or `read()` returning partial data).

    use super::test_fixtures::*;

    #[test]
    fn arrow_key_split_across_two_reads() {
        // Arrow Up arrives as: first read gets ESC, second read gets [ A.
        let mut parser = StatefulInputParser::default();

        // First chunk: ESC only, but more=true (buffer was full).
        parser.advance(&[ANSI_ESC], true);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0); // No event yet

        // Second chunk: [ A completes the sequence.
        parser.advance(b"[A", false);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_key_split_into_three_reads() {
        // Extreme fragmentation: ESC, then [, then A.
        let mut parser = StatefulInputParser::default();

        parser.advance(&[ANSI_ESC], true);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(b"[", true);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(b"A", false);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn multiple_events_across_chunks() {
        let mut parser = StatefulInputParser::default();

        // First chunk: 'a' and start of arrow sequence.
        parser.advance(&[b'a', ANSI_ESC], true);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));

        // Second chunk: completes arrow, adds 'b'.
        parser.advance(b"[Ab", false);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
        assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
    }
}

#[cfg(test)]
mod tests_iterator_impl {
    use super::test_fixtures::*;

    #[test]
    fn iterator_drains_internal_queue() {
        let mut parser = StatefulInputParser::default();
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
        let mut parser = StatefulInputParser::default();
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
        let mut parser = StatefulInputParser::default();

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

#[cfg(test)]
mod tests_special_keys {
    use super::test_fixtures::*;

    #[test]
    fn home_key() {
        // Home: ESC [ H
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'H'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Home));
    }

    #[test]
    fn end_key() {
        // End: ESC [ F
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'F'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::End));
    }

    #[test]
    fn delete_key() {
        // Delete: ESC [ 3 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'3', b'~'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Delete));
    }

    #[test]
    fn insert_key() {
        // Insert: ESC [ 2 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'2', b'~'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Insert));
    }

    #[test]
    fn page_up_key() {
        // Page Up: ESC [ 5 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'5', b'~'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageUp));
    }

    #[test]
    fn page_down_key() {
        // Page Down: ESC [ 6 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'6', b'~'], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageDown));
    }
}

#[cfg(test)]
mod tests_utf8_input {
    use super::test_fixtures::*;

    #[test]
    fn two_byte_utf8_char() {
        // 'é' is U+00E9, encoded as C3 A9
        let mut parser = StatefulInputParser::default();
        parser.advance(&[0xC3, 0xA9], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
    }

    #[test]
    fn three_byte_utf8_char() {
        // '中' is U+4E2D, encoded as E4 B8 AD
        let mut parser = StatefulInputParser::default();
        parser.advance(&[0xE4, 0xB8, 0xAD], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('中')));
    }

    #[test]
    fn four_byte_utf8_emoji() {
        // '😀' is U+1F600, encoded as F0 9F 98 80
        let mut parser = StatefulInputParser::default();
        parser.advance(&[0xF0, 0x9F, 0x98, 0x80], false);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('😀')));
    }

    #[test]
    fn utf8_split_across_chunks() {
        // 'é' split across two reads
        let mut parser = StatefulInputParser::default();

        parser.advance(&[0xC3], true);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(&[0xA9], false);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
    }
}

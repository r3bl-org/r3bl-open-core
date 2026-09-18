// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes. See [`StatefulInputParser`] docs.

use crate::{CSI_MIN_LEN, CSI_PREFIX, CSI_PREFIX_LEN, DEBUG_TUI_SHOW_DIRECT_TO_ANSI,
            SS3_PREFIX, SS3_SEQ_LEN,
            core::ansi::vt_100_terminal_input_parser::{MaybeMore, VT100InputEventIR,
                                                       try_parse_input_event}};
use std::collections::VecDeque;

/// Stateful parser for terminal input bytes.
///
/// Accumulates raw bytes and reassembles them across chunk boundaries into discrete
/// [`VT100InputEventIR`] events.
///
/// ## [`ESC`] Disambiguation ([`MaybeMore`])
///
/// Disambiguates a standalone [`ESC`] keypress from the initial byte of a multi-byte
/// escape sequence (such as `ESC [ A` for Up Arrow) using [`MaybeMore`].
///
/// While iterating through `read_buffer`, [`Self::advance()`] delegates to
/// [`MaybeMore::refine_for_byte_index()`] to determine stream availability per byte. See
/// [`MaybeMore`] for the full two-level heuristic model and pipeline architecture.
///
/// ## Unrecognized Sequence Discard Heuristics (Freeze Prevention)
///
/// When an escape sequence is not yet fully parsed, [`try_parse_input_event`] returns
/// `None`. If the sequence is unsupported or unrecognized (such as an obscure terminal
/// response, or a previously unhandled modified key like `Shift+Home`), returning `None`
/// must not leave the unparseable bytes in the accumulator indefinitely. Otherwise, every
/// subsequent keypress would be appended to the poisoned buffer, permanently locking up
/// the terminal input event loop.
///
/// To prevent this, `should_discard_unrecognized_sequence()` evaluates three heuristics
/// to detect when an unparsed buffer should be safely purged:
///
/// 1. **Completed [`CSI`] Sequence**: If the buffer begins with `ESC [` and contains a
///    terminating final byte in the range `0x40..=0x7E` (`CSI_FINAL_BYTE_MIN` through
///    `CSI_FINAL_BYTE_MAX`), the [`CSI`] sequence has reached its structural conclusion
///    according to the ECMA-48 standard. Because [`try_parse_input_event`] returned
///    `None`, the sequence is unrecognized and cannot be parsed. The buffer is cleared
///    immediately.
///
/// 2. **Completed [`SS3`] Sequence**: If the buffer begins with `ESC O` and has reached
///    its fixed 3-byte length (`ESC O` + command character), the [`SS3`] sequence is
///    structurally complete. If unrecognized, it is cleared immediately.
///
/// 3. **Safety Length Cap (`MAX_ESCAPE_SEQUENCE_LENGTH`)**: If malformed or
///    non-terminating input causes the accumulator to reach or exceed `64` bytes without
///    producing an event, the buffer is cleared to prevent unbounded memory growth and
///    restore input responsiveness.
///
/// When [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`] is enabled, discarded sequences log a
/// structured warning with both hex bytes and lossy [`UTF-8`] text to aid diagnosis.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`]: crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI
/// [`ESC`]: crate::EscSequence
/// [`MaybeMore::refine_for_byte_index()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::MaybeMore::refine_for_byte_index
/// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
/// [`try_parse_input_event`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[derive(Debug)]
pub struct StatefulInputParser {
    /// Accumulator for current [`ANSI`] escape sequence being parsed (capacity: 256
    /// bytes).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    accumulator: Vec<u8>,

    /// Queue of parsed events ready to be consumed (capacity: 128).
    internal_events: VecDeque<VT100InputEventIR>,
}

/// Minimum byte value for a [`CSI`] sequence terminating final byte (`b'@'`, `0x40`).
///
/// [`CSI`]: crate::CsiSequence
const CSI_FINAL_BYTE_MIN: u8 = b'@';

/// Maximum byte value for a [`CSI`] sequence terminating final byte (`b'~'`, `0x7E`).
///
/// [`CSI`]: crate::CsiSequence
const CSI_FINAL_BYTE_MAX: u8 = b'~';

/// Safety maximum byte length for an accumulated unparsed escape sequence.
///
/// Valid [`ANSI`]/[`CSI`]/[`SS3`] keyboard sequences rarely exceed 6-10 bytes, and mouse
/// tracking sequences rarely exceed 12-16 bytes. A length threshold of `64` provides an
/// abundant safety margin while preventing unbounded memory growth or permanent input
/// freeze if corrupted or malformed byte streams never terminate.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
const MAX_ESCAPE_SEQUENCE_LENGTH: usize = 64;

impl Default for StatefulInputParser {
    fn default() -> Self {
        StatefulInputParser {
            accumulator: Vec::with_capacity(256),
            internal_events: VecDeque::with_capacity(128),
        }
    }
}

impl StatefulInputParser {
    /// Processes incoming bytes and parses into events.
    /// - `read_buffer`: Raw bytes read from `stdin`.
    /// - `maybe_more`: Stream availability heuristic from the OS read syscall. See
    ///   [`MaybeMore`].
    pub fn advance(&mut self, read_buffer: &[u8], maybe_more: MaybeMore) {
        // Process one byte at a time & not the entire read buffer at once.
        for (idx, byte) in read_buffer.iter().enumerate() {
            let current_byte_maybe_more =
                maybe_more.refine_for_byte_index(idx, read_buffer.len());

            self.accumulator.push(*byte);

            match try_parse_input_event(&self.accumulator, current_byte_maybe_more) {
                Some((event, _bytes_consumed)) => {
                    // Successfully parsed - push event and clear accumulator.
                    self.internal_events.push_back(event);
                    self.accumulator.clear();
                }
                None => {
                    // Incomplete sequence or waiting for more bytes.
                    // Keep accumulator and continue accumulating, unless this is a
                    // completed but unrecognized sequence or exceeds
                    // the safety buffer length.
                    if self.should_discard_unrecognized_sequence() {
                        DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                            // % is Display, ? is Debug.
                            tracing::warn! {
                                message = "StatefulInputParser::advance - discarding unrecognized escape sequence",
                                discarded_hex = %format!("{:02X?}", self.accumulator),
                                discarded_str = %String::from_utf8_lossy(&self.accumulator),
                                buffer_len = self.accumulator.len(),
                            };
                        });
                        self.accumulator.clear();
                    }
                }
            }
        }
    }

    /// Checks if the accumulated buffer contains a completed or invalid escape sequence
    /// that failed to parse and should be discarded to avoid locking up future input.
    ///
    /// Evaluates three recovery criteria:
    /// 1. Completed [`CSI`] sequences: starts with `ESC [` and has reached a final byte
    ///    in `0x40..=0x7E` (`CSI_FINAL_BYTE_MIN` through `CSI_FINAL_BYTE_MAX`).
    /// 2. Completed [`SS3`] sequences: starts with `ESC O` and has reached `SS3_SEQ_LEN`
    ///    (3 bytes).
    /// 3. Safety overflow fallback: buffer length reaches or exceeds
    ///    `MAX_ESCAPE_SEQUENCE_LENGTH` (64 bytes).
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    fn should_discard_unrecognized_sequence(&self) -> bool {
        // CSI sequence (ESC [) that reached a terminating final byte (0x40..=0x7E)
        // after ESC [, but could not be parsed as a known event.
        if self.accumulator.starts_with(CSI_PREFIX)
            && self.accumulator.len() >= CSI_MIN_LEN
            && self.accumulator[CSI_PREFIX_LEN..]
                .iter()
                .any(|b| (CSI_FINAL_BYTE_MIN..=CSI_FINAL_BYTE_MAX).contains(b))
        {
            return true;
        }

        // SS3 sequence (ESC O) that reached its 3-byte length but could not be parsed.
        if self.accumulator.starts_with(SS3_PREFIX)
            && self.accumulator.len() >= SS3_SEQ_LEN
        {
            return true;
        }

        // Safety fallback: prevent unbounded accumulation for malformed input streams.
        if self.accumulator.len() >= MAX_ESCAPE_SEQUENCE_LENGTH {
            return true;
        }

        false
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
    pub use crate::{KeyState,
                    core::ansi::{generator::{SEQ_ARROW_DOWN, SEQ_ARROW_LEFT,
                                             SEQ_ARROW_RIGHT, SEQ_ARROW_UP},
                                 vt_100_terminal_input_parser::{MaybeMore,
                                                                VT100InputEventIR,
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

    /// Helper to create a keyboard event with modifiers for assertions.
    pub fn keyboard_event_with_modifiers(
        code: VT100KeyCodeIR,
        modifiers: VT100KeyModifiersIR,
    ) -> VT100InputEventIR {
        VT100InputEventIR::Keyboard { code, modifiers }
    }
}

#[cfg(test)]
mod tests_basic_parsing {
    use super::test_fixtures::*;

    #[test]
    fn single_ascii_char() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"a", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    }

    #[test]
    fn multiple_ascii_chars_single_read() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"abc", MaybeMore::Drained);

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
        parser.advance(b"\r", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Enter));
    }

    #[test]
    fn tab_key() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\t", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Tab));
    }

    #[test]
    fn backspace_key() {
        let mut parser = StatefulInputParser::default();
        // Historical quirk: Backspace key sends DEL (7F), not BS (08).
        // DEC VT100 reserved BS for cursor-left; most terminals inherited this.
        parser.advance(&[ASCII_DEL], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Backspace));
    }
}

/// Tests for the core [`ESC`] disambiguation logic using the [`MaybeMore`] heuristic.
///
/// The [`MaybeMore`] heuristic indicates whether additional bytes are likely waiting
/// in the kernel buffer or current in-memory chunk slice.
/// - When `is_more_anticipated()` is true, [`ESC`] (1B) is treated as the start of an
///   escape sequence.
/// - When [`MaybeMore::Drained`], it is emitted as a standalone [`ESC`] key press.
///
/// [`ESC`]: crate::EscSequence
/// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
#[cfg(test)]
mod tests_esc_disambiguation {
    use super::test_fixtures::*;

    #[test]
    fn lone_esc_with_more_false_emits_escape_key() {
        // User pressed ESC key alone - no more data coming.
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC], MaybeMore::Drained); // ESC byte, drained

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Escape));
    }

    #[test]
    fn esc_with_more_true_waits_for_sequence() {
        // ESC arrived but more bytes are coming - wait for full sequence.
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC], MaybeMore::KernelMayHaveMore); // ESC byte, kernel may have more

        // No event emitted yet - waiting for rest of sequence.
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn arrow_up_complete_sequence() {
        // Arrow Up: ESC [ A
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_UP, MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_down_complete_sequence() {
        // Arrow Down: ESC [ B
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_DOWN, MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Down));
    }

    #[test]
    fn arrow_right_complete_sequence() {
        // Arrow Right: ESC [ C
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_RIGHT, MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Right));
    }

    #[test]
    fn arrow_left_complete_sequence() {
        // Arrow Left: ESC [ D
        let mut parser = StatefulInputParser::default();
        parser.advance(SEQ_ARROW_LEFT, MaybeMore::Drained);

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

        // First chunk: ESC only, but more anticipated (kernel read buffer was full).
        parser.advance(&[ANSI_ESC], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0); // No event yet

        // Second chunk: [ A completes the sequence.
        parser.advance(b"[A", MaybeMore::Drained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_key_split_into_three_reads() {
        // Extreme fragmentation: ESC, then [, then A.
        let mut parser = StatefulInputParser::default();

        parser.advance(&[ANSI_ESC], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(b"[", MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(b"A", MaybeMore::Drained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn multiple_events_across_chunks() {
        let mut parser = StatefulInputParser::default();

        // First chunk: 'a' and start of arrow sequence.
        parser.advance(&[b'a', ANSI_ESC], MaybeMore::KernelMayHaveMore);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));

        // Second chunk: completes arrow, adds 'b'.
        parser.advance(b"[Ab", MaybeMore::Drained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
        assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
    }
}

#[cfg(test)]
mod impl_tests_iterator {
    use super::test_fixtures::*;

    #[test]
    fn iterator_drains_internal_queue() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"xyz", MaybeMore::Drained);

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
        parser.advance(b"abc", MaybeMore::Drained);

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

        parser.advance(b"a", MaybeMore::Drained);
        assert_eq!(
            parser.next(),
            Some(keyboard_event(VT100KeyCodeIR::Char('a')))
        );

        parser.advance(b"b", MaybeMore::Drained);
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
        parser.advance(&[ANSI_ESC, b'[', b'H'], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Home));
    }

    #[test]
    fn end_key() {
        // End: ESC [ F
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'F'], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::End));
    }

    #[test]
    fn delete_key() {
        // Delete: ESC [ 3 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'3', b'~'], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Delete));
    }

    #[test]
    fn insert_key() {
        // Insert: ESC [ 2 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'2', b'~'], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Insert));
    }

    #[test]
    fn page_up_key() {
        // Page Up: ESC [ 5 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'5', b'~'], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageUp));
    }

    #[test]
    fn page_down_key() {
        // Page Down: ESC [ 6 ~
        let mut parser = StatefulInputParser::default();
        parser.advance(&[ANSI_ESC, b'[', b'6', b'~'], MaybeMore::Drained);

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
        parser.advance(&[0xC3, 0xA9], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
    }

    #[test]
    fn three_byte_utf8_char() {
        // '中' is U+4E2D, encoded as E4 B8 AD
        let mut parser = StatefulInputParser::default();
        parser.advance(&[0xE4, 0xB8, 0xAD], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('中')));
    }

    #[test]
    fn four_byte_utf8_emoji() {
        // '😀' is U+1F600, encoded as F0 9F 98 80
        let mut parser = StatefulInputParser::default();
        parser.advance(&[0xF0, 0x9F, 0x98, 0x80], MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('😀')));
    }

    #[test]
    fn utf8_split_across_chunks() {
        // 'é' split across two reads
        let mut parser = StatefulInputParser::default();

        parser.advance(&[0xC3], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(&[0xA9], MaybeMore::Drained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
    }
}

#[cfg(test)]
mod tests_modified_and_unrecognized_sequences {
    use super::test_fixtures::*;

    #[test]
    fn shift_home_parsing() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\x1b[1;2H", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            keyboard_event_with_modifiers(
                VT100KeyCodeIR::Home,
                VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            )
        );
    }

    #[test]
    fn ctrl_home_parsing() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\x1b[1;5H", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            keyboard_event_with_modifiers(
                VT100KeyCodeIR::Home,
                VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                }
            )
        );
    }

    #[test]
    fn shift_end_parsing() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\x1b[1;2F", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            keyboard_event_with_modifiers(
                VT100KeyCodeIR::End,
                VT100KeyModifiersIR {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            )
        );
    }

    #[test]
    fn ctrl_end_parsing() {
        let mut parser = StatefulInputParser::default();
        parser.advance(b"\x1b[1;5F", MaybeMore::Drained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            keyboard_event_with_modifiers(
                VT100KeyCodeIR::End,
                VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::Pressed,
                }
            )
        );
    }

    #[test]
    fn unrecognized_csi_does_not_block_subsequent_input() {
        let mut parser = StatefulInputParser::default();

        // Send unrecognized CSI sequence (e.g., CSI 99 ; 99 z).
        parser.advance(b"\x1b[99;99z", MaybeMore::Drained);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly without freeze.
        parser.advance(b"a", MaybeMore::Drained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    }

    #[test]
    fn unrecognized_ss3_does_not_block_subsequent_input() {
        let mut parser = StatefulInputParser::default();

        // Send unrecognized SS3 sequence (ESC O X).
        parser.advance(b"\x1bOX", MaybeMore::Drained);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly without freeze.
        parser.advance(b"b", MaybeMore::Drained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('b')));
    }

    #[test]
    fn safety_buffer_overflow_clears_buffer() {
        let mut parser = StatefulInputParser::default();

        // Send a malformed unterminated escape sequence (ESC [ followed by 62 parameter
        // digits = 64 bytes).
        let mut long_unterminated = vec![ANSI_ESC, b'['];
        long_unterminated.extend_from_slice(&[b'1'; 62]);
        parser.advance(&long_unterminated, MaybeMore::KernelMayHaveMore);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly.
        parser.advance(b"c", MaybeMore::Drained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('c')));
    }
}

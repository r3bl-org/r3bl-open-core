// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes. See [`InputByteStreamToIrParser`] docs.

use super::osc_circuit_breaker::{OscCircuitBreaker, OscDrainResult};
use crate::{CSI_FINAL_BYTE_MAX, CSI_FINAL_BYTE_MIN, CSI_MIN_LEN, CSI_PREFIX,
            CSI_PREFIX_LEN, DEBUG_TUI_SHOW_DIRECT_TO_ANSI, OSC_PREFIX, SS3_PREFIX,
            SS3_SEQ_LEN,
            core::ansi::vt_100_terminal_input_parser::{MaybeMore, OscScanResult,
                                                       VT100InputEventIR,
                                                       scan_osc_sequence,
                                                       try_parse_input_event}};
use std::collections::VecDeque;

/// Stateful parser that converts raw terminal input bytes from [`stdin`] into
/// strongly-typed intermediate representation events ([`VT100InputEventIR`]).
///
/// In [raw mode], user inputs (keyboard, mouse) and terminal emulator responses (e.g.,
/// [`OSC`] 11 background color reports like `ESC ] 11 ; rgb:rrrr/gggg/bbbb BEL`, or
/// Cursor Position Reports like `ESC [ 24 ; 80 R`) arrive as a continuous, unframed byte
/// stream over standard input. These include:
///
/// - Single-byte [`ASCII`] control codes (e.g., `Ctrl+C`, `Enter`).
/// - Multi-byte [`UTF-8`] sequences (e.g., Unicode text and emojis).
/// - Variable-length [`ANSI`]/[`VT-100`] escape sequences, like [`CSI`] sequences for
///   arrow keys, function keys, mouse clicks, and modified keystrokes like `Shift+Home`.
/// - [`OSC`] query responses.
///
/// Because the operating system delivers these bytes in arbitrary chunk sizes via
/// [`read()`] syscalls, a single logical sequence may be split across multiple read
/// buffers. This parser maintains an internal accumulator to reassemble fragmented
/// sequences across read buffer boundaries into discrete [`VT100InputEventIR`] events.
///
/// # Stream Availability & Incomplete Sequences ([`MaybeMore`])
///
/// Because [`ANSI`] escape sequences can be split across multiple [`read()`] syscalls,
/// the parser must distinguish between an incomplete sequence waiting for subsequent
/// bytes and a standalone keypress (such as a physical [`ESC`]).
///
/// ## The I/O Pipeline
///
/// 1. A dedicated I/O thread ([`MioPollWorker`]) reads incoming [`stdin`] bytes into a
///    fixed-size userspace buffer ([`STDIN_READ_BUFFER_SIZE`], 1,024 bytes) via
///    [`consume_stdin_input_with_sender()`].
/// 2. For each read chunk, [`consume_stdin_input_with_sender()`] calls
///    [`parse_stdin_bytes_with_sender()`], which uses [`MaybeMore::from_read_count()`] to
///    check whether the userspace buffer was filled to capacity (`bytes_read ==
///    STDIN_READ_BUFFER_SIZE`), inferring whether more bytes may remain in the kernel's
///    [`PTY`] queue.
/// 3. [`parse_stdin_bytes_with_sender()`] forwards both the read bytes and the
///    [`MaybeMore`] hint to [`InputByteStreamToIrParser::advance()`], which delegates
///    sequence dispatching to [`try_parse_input_event()`].
///
/// ## Accumulator Lifecycle by Stream Availability
///
/// - [`MaybeMore::KernelMayHaveMore`]: The userspace read buffer is full, so trailing
///   bytes may still be queued in the kernel. Incomplete prefixes (like a lone [`ESC`])
///   return `None`, leaving unparsed bytes in the accumulator across calls to
///   [`InputByteStreamToIrParser::advance()`] to be completed by subsequent [`read()`]
///   chunks.
/// - [`MaybeMore::KernelDrained`]: The userspace read buffer is not full, confirming the
///   kernel's [`PTY`] buffer was completely drained. Standalone keys (like [`ESC`]) are
///   parsed with 0ms latency, drained from the accumulator, and enqueued.
///
/// For full details on the stream availability heuristic, kernel queue detection, and
/// network latency trade-offs, see [`MaybeMore`]. For escape sequence parsing and event
/// dispatch rules, see [`try_parse_input_event()`].
///
/// # Unrecognized Sequence Discard Heuristics (Freeze Prevention)
///
/// When an [`ANSI`] escape sequence is not yet fully parsed, [`try_parse_input_event`]
/// returns `None`. If the sequence is unsupported or unrecognized (such as an obscure
/// terminal response, or a previously unhandled modified key like `Shift+Home`),
/// returning `None` must not leave the unparseable bytes in the accumulator indefinitely.
/// Otherwise, every subsequent keypress would be appended to the poisoned buffer,
/// permanently locking up the terminal input event loop.
///
/// To prevent this, [`classify_unparsed_buffer()`] evaluates the accumulator with a
/// single classification pass returning [`UnparsedBufferClassification`]:
///
/// 1. **[`OSC`] Runaway Sequence ([`UnparsedBufferClassification::RunawayOsc`])**: If the
///    buffer begins with `ESC ]` ([`OSC_PREFIX`]) and exceeds [`MAX_OSC_SEQUENCE_LENGTH`]
///    (1 MiB) without encountering a terminator, it is purged to prevent unbounded memory
///    growth, and the parser transitions to [`OscCircuitBreaker::Open`] to discard
///    remaining in-flight payload bytes on-the-fly, preventing framing desynchronization
///    and text leakage.
///
/// 2. **Completed [`CSI`] Sequence
///    ([`UnparsedBufferClassification::MalformedSequence`])**: If the buffer begins with
///    `ESC [` and contains a terminating final byte in the range `0x40..=0x7E`
///    (`CSI_FINAL_BYTE_MIN` through `CSI_FINAL_BYTE_MAX`), the [`CSI`] sequence has
///    reached its structural conclusion according to the ECMA-48 standard. Because
///    [`try_parse_input_event`] returned `None`, this [`CSI`] sequence is unsupported or
///    malformed. Purging it immediately prevents the terminal from locking up on
///    unhandled keys like `Shift+Home`.
///
/// 3. **Completed [`SS3`] Sequence
///    ([`UnparsedBufferClassification::MalformedSequence`])**: If the buffer begins with
///    `ESC O` ([`SS3`]) and reaches 3 bytes, it is a complete single-character function
///    key sequence (e.g. `ESC O P` for F1). If unparsed, it is purged.
///
/// 4. **Safety Buffer Limit ([`UnparsedBufferClassification::MalformedSequence`])**: If
///    an unrecognized sequence does not match [`CSI`], [`OSC`], or [`SS3`], and reaches
///    [`MAX_ESCAPE_SEQUENCE_LENGTH`] (64 bytes), it is purged as corrupted or malformed
///    input to prevent unbounded memory growth.
///
/// 5. **Incomplete Sequence ([`UnparsedBufferClassification::Incomplete`])**: The buffer
///    contains an unfinished sequence and requires more bytes from subsequent reads to
///    complete. The accumulator is left intact.
///
/// When [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`] is enabled, discarded sequences log a
/// structured warning with both hex bytes and lossy [`UTF-8`] text to aid diagnosis.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`classify_unparsed_buffer()`]: InputByteStreamToIrParser::classify_unparsed_buffer
/// [`consume_stdin_input_with_sender()`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::consume_stdin_input_with_sender
/// [`CSI`]: crate::CsiSequence
/// [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`]: crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI
/// [`ESC`]: crate::EscSequence
/// [`InputByteStreamToIrParser::advance()`]: InputByteStreamToIrParser::advance
/// [`InputByteStreamToIrParser`]: InputByteStreamToIrParser
/// [`MAX_ESCAPE_SEQUENCE_LENGTH`]: MAX_ESCAPE_SEQUENCE_LENGTH
/// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
/// [`MaybeMore::from_read_count()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::MaybeMore::from_read_count
/// [`MaybeMore::KernelDrained`]:
///     crate::core::ansi::vt_100_terminal_input_parser::MaybeMore::KernelDrained
/// [`MaybeMore::KernelMayHaveMore`]:
///     crate::core::ansi::vt_100_terminal_input_parser::MaybeMore::KernelMayHaveMore
/// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
/// [`mio_poller`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller
/// [`MioPollWorker`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
/// [`OSC_PREFIX`]: crate::OSC_PREFIX
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`OscCircuitBreaker::Open`]:
///     super::osc_circuit_breaker::OscCircuitBreaker::Open
/// [`parse_stdin_bytes_with_sender()`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::parse_stdin_bytes_with_sender
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
/// [`STDIN_READ_BUFFER_SIZE`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::STDIN_READ_BUFFER_SIZE
/// [`stdin`]: std::io::stdin
/// [`try_parse_input_event()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`try_parse_input_event`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`UnparsedBufferClassification::Incomplete`]:
///     UnparsedBufferClassification::Incomplete
/// [`UnparsedBufferClassification::MalformedSequence`]:
///     UnparsedBufferClassification::MalformedSequence
/// [`UnparsedBufferClassification::RunawayOsc`]:
///     UnparsedBufferClassification::RunawayOsc
/// [`UnparsedBufferClassification`]: UnparsedBufferClassification
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
#[derive(Debug)]
pub struct InputByteStreamToIrParser {
    /// Accumulator for current [`ANSI`] escape sequence being parsed (capacity: 256
    /// bytes).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    accumulator: Vec<u8>,

    /// Queue of parsed events ready to be consumed (capacity: 128).
    internal_events: VecDeque<VT100InputEventIR>,

    /// Circuit breaker for swallowing runaway [`OSC`] sequences without allocating.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    osc_circuit_breaker: OscCircuitBreaker,
}

impl Default for InputByteStreamToIrParser {
    fn default() -> Self {
        InputByteStreamToIrParser {
            accumulator: Vec::with_capacity(256),
            internal_events: VecDeque::with_capacity(128),
            osc_circuit_breaker: OscCircuitBreaker::default(),
        }
    }
}

impl InputByteStreamToIrParser {
    /// Returns the current state of the circuit breaker.
    #[must_use]
    pub fn osc_circuit_breaker(&self) -> OscCircuitBreaker { self.osc_circuit_breaker }

    /// Drains bytes from an incoming chunk while in [`OscCircuitBreaker::Open`].
    ///
    /// Delegates to [`OscCircuitBreaker::drain_chunk`].
    ///
    /// [`OscCircuitBreaker::Open`]: OscCircuitBreaker::Open
    pub fn drain_osc_payload(&mut self, chunk: &[u8]) -> OscDrainResult {
        self.osc_circuit_breaker.drain_chunk(chunk)
    }

    /// Processes incoming bytes and parses into events.
    /// - `read_buffer`: Raw bytes read from [`stdin`].
    /// - `maybe_more`: Stream availability heuristic from the OS [`read()`] syscall. See
    ///   [`MaybeMore`].
    ///
    /// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
    /// [`stdin`]: std::io::stdin
    pub fn advance(&mut self, read_buffer: &[u8], maybe_more: MaybeMore) {
        let mut slice = read_buffer;

        // If the circuit breaker is open (draining a runaway OSC sequence), consume bytes
        // directly without appending to self.accumulator.
        match self.osc_circuit_breaker {
            OscCircuitBreaker::Open { .. } => {
                match self.osc_circuit_breaker.drain_chunk(slice) {
                    OscDrainResult::Full { .. } => return,
                    OscDrainResult::Partial { bytes_consumed, .. } => {
                        slice = &slice[bytes_consumed.as_usize()..];
                    }
                }
            }
            OscCircuitBreaker::Closed => {}
        }

        self.accumulator.extend_from_slice(slice);
        while !self.accumulator.is_empty() {
            match try_parse_input_event(&self.accumulator, maybe_more) {
                Some((event, bytes_consumed)) => {
                    let consumed = bytes_consumed.as_usize();
                    debug_assert!(
                        consumed > 0,
                        "Parser must consume at least 1 byte to prevent infinite loops"
                    );
                    if consumed == 0 {
                        break;
                    }
                    if event != VT100InputEventIR::Ignored {
                        self.internal_events.push_back(event);
                    }
                    self.accumulator.drain(..consumed);
                }
                None => {
                    match self.classify_unparsed_buffer() {
                        UnparsedBufferClassification::Incomplete => {
                            // Incomplete sequence: await more bytes from subsequent
                            // reads.
                            break;
                        }
                        UnparsedBufferClassification::MalformedSequence => {
                            DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                                // % is Display, ? is Debug.
                                tracing::warn! {
                                    message = "InputByteStreamToIrParser::advance",
                                    status = "discarding unrecognized/malformed escape sequence",
                                    discarded_hex = %format!("{:02X?}", self.accumulator),
                                    discarded_str = %String::from_utf8_lossy(&self.accumulator),
                                    buffer_len = self.accumulator.len(),
                                };
                            });
                            self.accumulator.clear();
                            break;
                        }
                        UnparsedBufferClassification::RunawayOsc => {
                            DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                                tracing::warn! {
                                    message = "InputByteStreamToIrParser::advance",
                                    status = "tripping circuit breaker for runaway OSC sequence",
                                    buffer_len = self.accumulator.len(),
                                };
                            });
                            self.osc_circuit_breaker.trip(self.accumulator.len());
                            self.accumulator.clear();
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Inspects and classifies the unparsed bytes currently in `self.accumulator`.
    ///
    /// Performs a single-pass classification across 4 criteria:
    /// 1. **[`OSC`] sequence**: Scans for runaway (> 1 MiB) or incomplete state via
    ///    [`scan_osc_sequence()`].
    /// 2. **Completed [`CSI`]**: Reached a final byte in `0x40..=0x7E`
    ///    ([`CSI_FINAL_BYTE_MIN`] through [`CSI_FINAL_BYTE_MAX`]) but could not be
    ///    parsed.
    /// 3. **Completed [`SS3`]**: Reached [`SS3_SEQ_LEN`] (3 bytes) but could not be
    ///    parsed.
    /// 4. **Safety overflow**: Non-[`OSC`] sequence reached or exceeded
    ///    [`MAX_ESCAPE_SEQUENCE_LENGTH`] (64 bytes).
    ///
    /// [`CSI_FINAL_BYTE_MAX`]: crate::CSI_FINAL_BYTE_MAX
    /// [`CSI_FINAL_BYTE_MIN`]: crate::CSI_FINAL_BYTE_MIN
    /// [`CSI`]: crate::CsiSequence
    /// [`MAX_ESCAPE_SEQUENCE_LENGTH`]: MAX_ESCAPE_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`scan_osc_sequence()`]: crate::core::ansi::vt_100_terminal_input_parser::scan_osc_sequence
    /// [`SS3_SEQ_LEN`]: crate::SS3_SEQ_LEN
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    #[must_use]
    pub fn classify_unparsed_buffer(&self) -> UnparsedBufferClassification {
        // 1. OSC sequence check (1 MiB threshold).
        if self.accumulator.starts_with(OSC_PREFIX) {
            return match scan_osc_sequence(&self.accumulator) {
                OscScanResult::Runaway => UnparsedBufferClassification::RunawayOsc,
                OscScanResult::IncompleteDigits | OscScanResult::IncompletePayload => {
                    UnparsedBufferClassification::Incomplete
                }
                OscScanResult::Complete(_) | OscScanResult::InvalidSyntax => {
                    UnparsedBufferClassification::MalformedSequence
                }
            };
        }

        // 2. Completed CSI sequence that could not be parsed.
        if self.accumulator.starts_with(CSI_PREFIX)
            && self.accumulator.len() >= CSI_MIN_LEN
            && self.accumulator[CSI_PREFIX_LEN..]
                .iter()
                .any(|b| (CSI_FINAL_BYTE_MIN..=CSI_FINAL_BYTE_MAX).contains(b))
        {
            return UnparsedBufferClassification::MalformedSequence;
        }

        // 3. Completed SS3 sequence that could not be parsed.
        if self.accumulator.starts_with(SS3_PREFIX)
            && self.accumulator.len() >= SS3_SEQ_LEN
        {
            return UnparsedBufferClassification::MalformedSequence;
        }

        // 4. Safety fallback for non-OSC sequences exceeding 64 bytes.
        if self.accumulator.len() >= MAX_ESCAPE_SEQUENCE_LENGTH {
            return UnparsedBufferClassification::MalformedSequence;
        }

        UnparsedBufferClassification::Incomplete
    }
}

impl Iterator for InputByteStreamToIrParser {
    type Item = VT100InputEventIR;

    fn next(&mut self) -> Option<Self::Item> { self.internal_events.pop_front() }
}

/// Classification of unparsed bytes accumulated in [`InputByteStreamToIrParser`].
///
/// When [`try_parse_input_event()`] returns `None`, this enum categorizes why the
/// bytes could not be parsed and guides the parser's disposition.
///
/// [`try_parse_input_event()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnparsedBufferClassification {
    /// Incomplete sequence awaiting additional bytes from subsequent reads.
    ///
    /// The parser leaves the accumulator intact and waits for the next read from
    /// [`stdin`].
    ///
    /// [`stdin`]: std::io::stdin
    Incomplete,

    /// Completed but malformed, unsupported, or overflowing non-[`OSC`] sequence (e.g.,
    /// invalid [`CSI`] final byte, unrecognized [`SS3`] sequence, or >64 bytes of
    /// escape syntax).
    ///
    /// The parser purges the sequence to prevent stream lockup.
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    MalformedSequence,

    /// Oversized [`OSC`] sequence exceeding [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB) without a
    /// terminator.
    ///
    /// The parser trips the circuit breaker: purges the 1 MiB accumulator and enters
    /// streaming drain mode to swallow the remainder without allocations.
    ///
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    RunawayOsc,
}

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
pub const MAX_ESCAPE_SEQUENCE_LENGTH: usize = 64;

/// Shared test helpers and imports for [`InputByteStreamToIrParser`] tests.
#[cfg(test)]
mod test_fixtures {
    pub use super::{super::osc_circuit_breaker::OscCircuitBreaker,
                    InputByteStreamToIrParser, UnparsedBufferClassification};
    pub use crate::{ANSI_BEL, ANSI_ESC, ANSI_ST_7BIT, ANSI_ST_FINAL, ASCII_DEL,
                    CONTROL_ENTER, CONTROL_TAB, CSI_PREFIX, KeyState, LINE_FEED,
                    MODIFIER_CTRL, MODIFIER_SHIFT, OSC_PREFIX, SPECIAL_DELETE_CODE,
                    SPECIAL_END_FINAL, SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                    SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE,
                    core::ansi::{generator::{SEQ_ARROW_DOWN, SEQ_ARROW_LEFT,
                                             SEQ_ARROW_RIGHT, SEQ_ARROW_UP, SEQ_END,
                                             SEQ_HOME, csi_modified, csi_tilde, ss3},
                                 vt_100_terminal_input_parser::{MaybeMore,
                                                                VT100InputEventIR,
                                                                VT100KeyCodeIR,
                                                                VT100KeyModifiersIR}}};

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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(b"a", MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    }

    #[test]
    fn multiple_ascii_chars_single_read() {
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(b"abc", MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
        assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
        assert_eq!(events[2], keyboard_event(VT100KeyCodeIR::Char('c')));
    }

    #[test]
    fn enter_key() {
        let mut parser = InputByteStreamToIrParser::default();
        // In raw mode, Enter sends CR (0D), not LF (0A).
        // The kernel's line discipline translates CR→LF, but raw mode bypasses this.
        parser.advance(&[CONTROL_ENTER], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Enter));
    }

    #[test]
    fn tab_key() {
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[CONTROL_TAB], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Tab));
    }

    #[test]
    fn backspace_key() {
        let mut parser = InputByteStreamToIrParser::default();
        // Historical quirk: Backspace key sends DEL (7F), not BS (08).
        // DEC VT100 reserved BS for cursor-left; most terminals inherited this.
        parser.advance(&[ASCII_DEL], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Backspace));
    }
}

/// Tests for the core [`ESC`] disambiguation logic using the [`MaybeMore`] heuristic.
///
/// The [`MaybeMore`] heuristic indicates whether additional bytes are likely waiting
/// in the kernel buffer or current in-memory chunk slice.
/// - When [`MaybeMore::KernelMayHaveMore`], [`ESC`] (1B) is treated as the start of an
///   escape sequence.
/// - When [`MaybeMore::KernelDrained`], it is emitted as a standalone [`ESC`] key press.
///
/// [`ESC`]: crate::EscSequence
/// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
#[cfg(test)]
mod tests_esc_disambiguation {
    use super::test_fixtures::*;

    #[test]
    fn lone_esc_with_more_false_emits_escape_key() {
        // User pressed ESC key alone - no more data coming.
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[ANSI_ESC], MaybeMore::KernelDrained); // ESC byte, drained

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Escape));
    }

    #[test]
    fn esc_with_more_true_waits_for_sequence() {
        // ESC arrived but more bytes are coming - wait for full sequence.
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[ANSI_ESC], MaybeMore::KernelMayHaveMore); // ESC byte, kernel may have more

        // No event emitted yet - waiting for rest of sequence.
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn arrow_up_complete_sequence() {
        // Arrow Up: ESC [ A
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_ARROW_UP, MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_down_complete_sequence() {
        // Arrow Down: ESC [ B
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_ARROW_DOWN, MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Down));
    }

    #[test]
    fn arrow_right_complete_sequence() {
        // Arrow Right: ESC [ C
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_ARROW_RIGHT, MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Right));
    }

    #[test]
    fn arrow_left_complete_sequence() {
        // Arrow Left: ESC [ D
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_ARROW_LEFT, MaybeMore::KernelDrained);

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
        let mut parser = InputByteStreamToIrParser::default();

        // First chunk: ESC only, but more anticipated (kernel read buffer was full).
        parser.advance(&[SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0); // No event yet

        // Second chunk: [ A completes the sequence.
        parser.advance(&SEQ_ARROW_UP[1..], MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn arrow_key_split_into_three_reads() {
        // Extreme fragmentation: ESC, then [, then A.
        let mut parser = InputByteStreamToIrParser::default();

        parser.advance(&[SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(&[SEQ_ARROW_UP[1]], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(&[SEQ_ARROW_UP[2]], MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    }

    #[test]
    fn multiple_events_across_chunks() {
        let mut parser = InputByteStreamToIrParser::default();

        // First chunk: 'a' and start of arrow sequence.
        parser.advance(&[b'a', SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));

        // Second chunk: completes arrow, adds 'b'.
        let second_chunk = [&SEQ_ARROW_UP[1..], b"b"].concat();
        parser.advance(&second_chunk, MaybeMore::KernelDrained);
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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(b"xyz", MaybeMore::KernelDrained);

        // First iteration drains the queue.
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 3);

        // Second iteration returns empty - queue is drained.
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn iterator_returns_events_in_fifo_order() {
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(b"abc", MaybeMore::KernelDrained);

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
        let mut parser = InputByteStreamToIrParser::default();

        parser.advance(b"a", MaybeMore::KernelDrained);
        assert_eq!(
            parser.next(),
            Some(keyboard_event(VT100KeyCodeIR::Char('a')))
        );

        parser.advance(b"b", MaybeMore::KernelDrained);
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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_HOME, MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Home));
    }

    #[test]
    fn end_key() {
        // End: ESC [ F
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(SEQ_END, MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::End));
    }

    #[test]
    fn delete_key() {
        // Delete: ESC [ 3 ~
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&csi_tilde(SPECIAL_DELETE_CODE), MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Delete));
    }

    #[test]
    fn insert_key() {
        // Insert: ESC [ 2 ~
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&csi_tilde(SPECIAL_INSERT_CODE), MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Insert));
    }

    #[test]
    fn page_up_key() {
        // Page Up: ESC [ 5 ~
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&csi_tilde(SPECIAL_PAGE_UP_CODE), MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageUp));
    }

    #[test]
    fn page_down_key() {
        // Page Down: ESC [ 6 ~
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&csi_tilde(SPECIAL_PAGE_DOWN_CODE), MaybeMore::KernelDrained);

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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[0xC3, 0xA9], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
    }

    #[test]
    fn three_byte_utf8_char() {
        // '中' is U+4E2D, encoded as E4 B8 AD
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[0xE4, 0xB8, 0xAD], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('中')));
    }

    #[test]
    fn four_byte_utf8_emoji() {
        // '😀' is U+1F600, encoded as F0 9F 98 80
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[0xF0, 0x9F, 0x98, 0x80], MaybeMore::KernelDrained);

        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('😀')));
    }

    #[test]
    fn utf8_split_across_chunks() {
        // 'é' split across two reads
        let mut parser = InputByteStreamToIrParser::default();

        parser.advance(&[0xC3], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(&[0xA9], MaybeMore::KernelDrained);
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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(
            &csi_modified(MODIFIER_SHIFT, SPECIAL_HOME_FINAL),
            MaybeMore::KernelDrained,
        );

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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(
            &csi_modified(MODIFIER_CTRL, SPECIAL_HOME_FINAL),
            MaybeMore::KernelDrained,
        );

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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(
            &csi_modified(MODIFIER_SHIFT, SPECIAL_END_FINAL),
            MaybeMore::KernelDrained,
        );

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
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(
            &csi_modified(MODIFIER_CTRL, SPECIAL_END_FINAL),
            MaybeMore::KernelDrained,
        );

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
        let mut parser = InputByteStreamToIrParser::default();

        // Send unrecognized CSI sequence (e.g., CSI 99 ; 99 z).
        let unrecognized_csi = [CSI_PREFIX, b"99;99z"].concat();
        parser.advance(&unrecognized_csi, MaybeMore::KernelDrained);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly without freeze.
        parser.advance(b"a", MaybeMore::KernelDrained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    }

    #[test]
    fn unrecognized_ss3_does_not_block_subsequent_input() {
        let mut parser = InputByteStreamToIrParser::default();

        // Send unrecognized SS3 sequence (ESC O X).
        parser.advance(&ss3(b'X'), MaybeMore::KernelDrained);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly without freeze.
        parser.advance(b"b", MaybeMore::KernelDrained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('b')));
    }

    #[test]
    fn safety_buffer_overflow_clears_buffer() {
        let mut parser = InputByteStreamToIrParser::default();

        // Send a malformed unterminated escape sequence (ESC [ followed by 62 parameter
        // digits = 64 bytes).
        let mut long_unterminated = CSI_PREFIX.to_vec();
        long_unterminated.extend_from_slice(&[b'1'; 62]);
        parser.advance(&long_unterminated, MaybeMore::KernelMayHaveMore);

        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events.len(), 0);

        // Next character typed must be parsed cleanly.
        parser.advance(b"c", MaybeMore::KernelDrained);
        let events: Vec<_> = parser.collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('c')));
    }
}

/// Tests for [`OSC`] sequence absorption, Alt+] disambiguation, and multi-event draining.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[cfg(test)]
mod tests_osc_and_alt_bracket {
    use super::test_fixtures::*;
    use crate::{MAX_OSC_DRAIN_BYTES, MAX_OSC_SEQUENCE_LENGTH};

    fn alt_bracket() -> VT100InputEventIR {
        keyboard_event_with_modifiers(
            VT100KeyCodeIR::Char(']'),
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            },
        )
    }

    #[test]
    fn lone_alt_bracket_single_and_split_reads() {
        // Single chunk:
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(OSC_PREFIX, MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![alt_bracket()]);

        // Split reads: ESC in chunk 1 (KernelMayHaveMore), ] in chunk 2 (Drained)
        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&[ANSI_ESC], MaybeMore::KernelMayHaveMore);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        parser.advance(b"]", MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![alt_bracket()]);
    }

    #[test]
    fn alt_bracket_followed_by_multiple_characters_same_chunk() {
        let mut parser = InputByteStreamToIrParser::default();
        let input = [OSC_PREFIX, b"abc"].concat();
        parser.advance(&input, MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(
            events,
            vec![
                alt_bracket(),
                keyboard_event(VT100KeyCodeIR::Char('a')),
                keyboard_event(VT100KeyCodeIR::Char('b')),
                keyboard_event(VT100KeyCodeIR::Char('c')),
            ]
        );
    }

    #[test]
    fn alt_bracket_followed_by_digit_same_chunk_drained() {
        let mut parser = InputByteStreamToIrParser::default();
        let input = [OSC_PREFIX, b"5"].concat();
        parser.advance(&input, MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(
            events,
            vec![alt_bracket(), keyboard_event(VT100KeyCodeIR::Char('5')),]
        );
    }

    #[test]
    fn osc_terminated_with_bel_and_st_absorbed() {
        let mut parser = InputByteStreamToIrParser::default();
        let bel_seq = [OSC_PREFIX, b"11;rgb:0000/0000/0000", &[ANSI_BEL]].concat();
        parser.advance(&bel_seq, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        let st_seq = [OSC_PREFIX, b"11;rgb:ffff/ffff/ffff", ANSI_ST_7BIT].concat();
        parser.advance(&st_seq, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    }

    #[test]
    fn long_osc_sequence_not_purged_by_64_byte_limit() {
        // OSC sequence longer than 64 bytes (119 bytes).
        let mut long_osc = Vec::new();
        long_osc.extend_from_slice(OSC_PREFIX);
        long_osc.extend_from_slice(b"52;c;");
        long_osc.resize(118, b'A');
        long_osc.push(ANSI_BEL); // BEL terminator
        assert_eq!(long_osc.len(), 119);

        let mut parser = InputByteStreamToIrParser::default();
        parser.advance(&long_osc, MaybeMore::KernelDrained);
        // Completely absorbed, zero events leaked, buffer drained.
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
    }

    /// An [`OSC`] `52` sequence follows this format:
    /// `ESC ] 52 ; <target> ; <payload> <terminator>`
    /// - Prefix: `ESC ]` (0x1B 0x5D) introduces an Operating System Command
    /// - Code `52`: Specifies the clipboard operation.
    /// - Target: The clipboard selection to interact with:
    ///     - c: System clipboard.
    ///     - p: Primary selection (common on Wayland / Linux).
    /// - Payload: Base64-encoded text (or ? when performing a query).
    /// - Terminator: Either BEL (0x07) or 7-bit ST ([`ESC`] \, 0x1B 0x5C).
    ///
    /// [`ESC`]: crate::EscSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[test]
    fn osc_with_utf8_continuation_byte_0x9c_absorbed() {
        let mut parser = InputByteStreamToIrParser::default();
        // UTF-8 checkmark ✓ contains 0x9C. Must be absorbed with 0 leakage.
        let seq = [OSC_PREFIX, b"52;c;\xe2\x9c\x93", &[ANSI_BEL]].concat();
        parser.advance(&seq, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
    }

    #[test]
    fn osc_followed_by_typing_same_chunk() {
        let mut parser = InputByteStreamToIrParser::default();
        let seq = [OSC_PREFIX, b"0;title", &[ANSI_BEL, b'a']].concat();
        parser.advance(&seq, MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('a'))]);
    }

    #[test]
    fn chunked_osc_sequence_split_across_reads() {
        let mut parser = InputByteStreamToIrParser::default();
        // Chunk 1: prefix + partial payload, read drained (more == false)
        let chunk1 = [OSC_PREFIX, b"11;rgb:00"].concat();
        parser.advance(&chunk1, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        // Chunk 2: rest of payload + terminator, read drained
        let chunk2 = [b"00/0000/0000".as_slice(), &[ANSI_BEL]].concat();
        parser.advance(&chunk2, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
    }

    #[test]
    fn chunked_osc_followed_by_typing_across_reads() {
        let mut parser = InputByteStreamToIrParser::default();
        // Chunk 1: incomplete OSC
        let chunk1 = [OSC_PREFIX, b"0;ti"].concat();
        parser.advance(&chunk1, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

        // Chunk 2: end of OSC + user typed 'a'
        let chunk2 = [b"tle".as_slice(), &[ANSI_BEL, b'a']].concat();
        parser.advance(&chunk2, MaybeMore::KernelDrained);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('a'))]);
    }

    #[test]
    fn runaway_unterminated_osc_purged_and_recovers() {
        let mut parser = InputByteStreamToIrParser::default();

        // Send unterminated OSC exceeding MAX_OSC_SEQUENCE_LENGTH
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'x');

        parser.advance(&runaway, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
        assert_eq!(
            parser.osc_circuit_breaker(),
            OscCircuitBreaker::Open {
                saw_partial_esc: false,
                drained_bytes: runaway.len(),
            }
        );

        // Terminating the runaway sequence with BEL cleanly ends the drain and emits
        // typed 'z'
        parser.advance(&[ANSI_BEL, b'z'], MaybeMore::KernelDrained);
        assert_eq!(parser.osc_circuit_breaker(), OscCircuitBreaker::Closed);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('z'))]);
    }

    #[test]
    fn runaway_osc_draining_swallows_subsequent_chunks_until_bel() {
        let mut parser = InputByteStreamToIrParser::default();

        // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        parser.advance(&runaway, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
        assert!(matches!(
            parser.osc_circuit_breaker(),
            OscCircuitBreaker::Open { .. }
        ));

        // Chunk 2: trailing payload chunk in the pipe without terminator.
        // MUST be swallowed and discarded silently (0 events, accumulator remains empty).
        let trailing_chunk = vec![b'b'; 4096];
        parser.advance(&trailing_chunk, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(parser.accumulator.is_empty());
        assert!(matches!(
            parser.osc_circuit_breaker(),
            OscCircuitBreaker::Open { .. }
        ));

        // Chunk 3: trailing payload ending in BEL terminator, followed by human typing
        // "ok". BEL terminates the drain; "ok" is emitted as keystrokes.
        let term_chunk = [b"bbbb".as_slice(), &[ANSI_BEL], b"ok"].concat();
        parser.advance(&term_chunk, MaybeMore::KernelDrained);
        assert_eq!(parser.osc_circuit_breaker(), OscCircuitBreaker::Closed);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(
            events,
            vec![
                keyboard_event(VT100KeyCodeIR::Char('o')),
                keyboard_event(VT100KeyCodeIR::Char('k')),
            ]
        );
    }

    #[test]
    fn runaway_osc_draining_terminated_by_st_across_chunk_boundary() {
        let mut parser = InputByteStreamToIrParser::default();

        // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        parser.advance(&runaway, MaybeMore::KernelDrained);

        // Chunk 2: ends in lone ESC
        let chunk2 = [b"payload_data".as_slice(), &[ANSI_ESC]].concat();
        parser.advance(&chunk2, MaybeMore::KernelDrained);
        assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
        assert!(matches!(
            parser.osc_circuit_breaker(),
            OscCircuitBreaker::Open {
                saw_partial_esc: true,
                ..
            }
        ));

        // Chunk 3: begins with '\' completing 7-bit ST (\x1b\), followed by typed 'w'
        parser.advance(&[ANSI_ST_FINAL, b'w'], MaybeMore::KernelDrained);
        assert_eq!(parser.osc_circuit_breaker(), OscCircuitBreaker::Closed);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('w'))]);
    }

    #[test]
    fn runaway_osc_draining_aborted_by_newline() {
        let mut parser = InputByteStreamToIrParser::default();

        // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        parser.advance(&runaway, MaybeMore::KernelDrained);

        // Chunk 2: raw newline aborts OSC control string.
        // Newline is emitted as Enter, and subsequent characters as keystrokes.
        let chunk2 = [b"payload".as_slice(), &[LINE_FEED], b"hi"].concat();
        parser.advance(&chunk2, MaybeMore::KernelDrained);
        assert_eq!(parser.osc_circuit_breaker(), OscCircuitBreaker::Closed);
        let events: Vec<_> = (&mut parser).collect();
        assert_eq!(
            events,
            vec![
                keyboard_event(VT100KeyCodeIR::Enter),
                keyboard_event(VT100KeyCodeIR::Char('h')),
                keyboard_event(VT100KeyCodeIR::Char('i')),
            ]
        );
    }

    #[test]
    fn runaway_osc_draining_safety_ceiling() {
        let mut parser = InputByteStreamToIrParser::default();

        // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH (starts drain with ~1 MiB drained)
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        parser.advance(&runaway, MaybeMore::KernelDrained);

        // Chunk 2: massive unterminated chunk exceeding MAX_OSC_DRAIN_BYTES
        let massive_chunk = vec![b'b'; MAX_OSC_DRAIN_BYTES];
        parser.advance(&massive_chunk, MaybeMore::KernelDrained);

        // Safety ceiling triggered: breaker resets to Closed
        assert_eq!(parser.osc_circuit_breaker(), OscCircuitBreaker::Closed);
    }
}

#[cfg(test)]
mod tests_classify_unparsed_buffer {
    use super::test_fixtures::*;
    use crate::MAX_OSC_SEQUENCE_LENGTH;

    #[test]
    fn incomplete_sequences() {
        let mut parser = InputByteStreamToIrParser::default();

        // Empty accumulator.
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::Incomplete
        );

        // Incomplete CSI sequence.
        parser.accumulator.extend_from_slice(CSI_PREFIX);
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::Incomplete
        );

        // Incomplete OSC sequence (< 1 MiB).
        parser.accumulator.clear();
        parser.accumulator.extend_from_slice(b"\x1b]11;rgb");
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::Incomplete
        );
    }

    #[test]
    fn malformed_csi_sequence() {
        let mut parser = InputByteStreamToIrParser::default();

        // Completed CSI sequence with an unknown final byte (e.g. 'z').
        parser.accumulator.extend_from_slice(b"\x1b[999z");
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::MalformedSequence
        );
    }

    #[test]
    fn malformed_ss3_sequence() {
        let mut parser = InputByteStreamToIrParser::default();

        // Completed SS3 sequence with an unrecognized key byte.
        parser.accumulator.extend_from_slice(b"\x1bO?");
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::MalformedSequence
        );
    }

    #[test]
    fn safety_buffer_overflow() {
        let mut parser = InputByteStreamToIrParser::default();

        // Non-OSC sequence exceeding 64 bytes without completing.
        let mut overflow = Vec::with_capacity(65);
        overflow.extend_from_slice(b"\x1b?");
        overflow.resize(65, b'x');
        parser.accumulator.extend_from_slice(&overflow);
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::MalformedSequence
        );
    }

    #[test]
    fn runaway_osc_sequence() {
        let mut parser = InputByteStreamToIrParser::default();

        // Runaway unterminated OSC exceeding MAX_OSC_SEQUENCE_LENGTH.
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'x');
        parser.accumulator.extend_from_slice(&runaway);
        assert_eq!(
            parser.classify_unparsed_buffer(),
            UnparsedBufferClassification::RunawayOsc
        );
    }
}

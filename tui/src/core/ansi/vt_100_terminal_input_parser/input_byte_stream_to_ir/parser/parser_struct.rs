// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful parser for terminal input bytes. See [`InputByteStreamToIrParser`] docs.

use super::{classification::UnparsedBufferClassification,
            constants::{ACCUMULATOR_INITIAL_CAPACITY, INTERNAL_EVENTS_INITIAL_CAPACITY}};
use crate::{DEBUG_TUI_SHOW_DIRECT_TO_ANSI, NumericValue,
            core::ansi::vt_100_terminal_input_parser::{MaybeMore, ParsedInputEventIR,
                                                       VT100InputEventIR,
                                                       input_byte_stream_to_ir::OscCircuitBreaker,
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
///    [`MaybeMore`] hint to [`InputByteStreamToIrParser::process_incoming_bytes()`],
///    which delegates sequence dispatching to [`try_parse_input_event()`].
///
/// ## Accumulator Lifecycle by Stream Availability
///
/// - [`MaybeMore::KernelMayHaveMore`]: The userspace read buffer is full, so trailing
///   bytes may still be queued in the kernel. Incomplete prefixes (like a lone [`ESC`])
///   return `None`, leaving unparsed bytes in the accumulator across calls to
///   [`InputByteStreamToIrParser::process_incoming_bytes()`] to be completed by
///   subsequent [`read()`] chunks.
/// - [`MaybeMore::KernelDrained`]: The userspace read buffer is not full, confirming the
///   kernel's [`PTY`] buffer was completely drained. Standalone keys (like [`ESC`]) are
///   parsed with 0ms latency, drained from the accumulator, and enqueued.
///
/// For full details on the stream availability heuristic, kernel queue detection, and
/// network latency trade-offs, see [`MaybeMore`]. For escape sequence parsing and event
/// dispatch rules, see [`try_parse_input_event()`].
///
/// # Unrecognized Sequence Discard Heuristics (Accumulator Poisoning Prevention)
///
/// When an [`ANSI`] escape sequence is not yet fully parsed, [`try_parse_input_event`]
/// returns `None`. If the sequence is unsupported or unrecognized (such as an obscure
/// terminal response or unmapped control sequence), returning `None` must not leave the
/// unparseable bytes in the accumulator indefinitely. Otherwise, every subsequent
/// keypress would be appended to the poisoned accumulator, causing accumulator poisoning
/// and permanently locking up the terminal input event loop.
///
/// To prevent this, [`UnparsedBufferClassification::classify()`] evaluates the
/// accumulator with a single classification pass returning
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
///    malformed. Purging it immediately prevents accumulator poisoning from unhandled
///    sequences.
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
/// [`consume_stdin_input_with_sender()`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::consume_stdin_input_with_sender
/// [`CSI`]: crate::CsiSequence
/// [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`]: crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI
/// [`ESC`]: crate::EscSequence
/// [`InputByteStreamToIrParser::process_incoming_bytes()`]:
///     InputByteStreamToIrParser::process_incoming_bytes
/// [`InputByteStreamToIrParser`]: InputByteStreamToIrParser
/// [`MAX_ESCAPE_SEQUENCE_LENGTH`]: super::constants::MAX_ESCAPE_SEQUENCE_LENGTH
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
///     crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::OscCircuitBreaker::Open
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
///     super::classification::UnparsedBufferClassification::Incomplete
/// [`UnparsedBufferClassification::MalformedSequence`]:
///     super::classification::UnparsedBufferClassification::MalformedSequence
/// [`UnparsedBufferClassification::RunawayOsc`]:
///     super::classification::UnparsedBufferClassification::RunawayOsc
/// [`UnparsedBufferClassification`]:
///     super::classification::UnparsedBufferClassification
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
#[derive(Debug)]
pub struct InputByteStreamToIrParser {
    /// Accumulator for current [`ANSI`] escape sequence being parsed.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub(crate) accumulator: Vec<u8>,

    /// Queue of parsed events ready to be consumed.
    pub(crate) internal_events: VecDeque<VT100InputEventIR>,

    /// Circuit breaker for swallowing runaway [`OSC`] sequences without allocating.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub(crate) osc_circuit_breaker: OscCircuitBreaker,
}

impl Default for InputByteStreamToIrParser {
    fn default() -> Self {
        InputByteStreamToIrParser {
            accumulator: Vec::with_capacity(ACCUMULATOR_INITIAL_CAPACITY),
            internal_events: VecDeque::with_capacity(INTERNAL_EVENTS_INITIAL_CAPACITY),
            osc_circuit_breaker: OscCircuitBreaker::default(),
        }
    }
}

impl InputByteStreamToIrParser {
    /// Processes incoming byte chunks from [`stdin`].
    ///
    /// This method is called repeatedly as streaming chunks arrive from [`stdin`] (such
    /// as in the edge-triggered draining loop of [`MioPollWorker`] on the dedicated I/O
    /// thread).
    ///
    /// This is a stateful parser that processes these incoming bytes and uses the three
    /// fields in this struct to do the following:
    /// 1. Uses the state machine in the [`OscCircuitBreaker`] field of this struct (via
    ///    its [`try_drain()`] method) to actually detect and handle runaway [`OSC`]
    ///    sequences across chunk boundaries (before they enter the accumulator).
    /// 2. Parses incoming bytes into input event IR (intermediate representation)
    ///    [`VT100InputEventIR`] and stores them in the internal events queue field.
    /// 3. Accumulates bytes (incomplete escape sequence, multi-byte [`UTF-8`] sequences)
    ///    in the internal accumulator field to be processed in subsequent calls.
    ///
    /// # Arguments
    ///
    /// - `read_buffer`: Raw bytes read from [`stdin`].
    /// - `maybe_more`: Stream availability heuristic from the OS [`read()`] syscall. See
    ///   [`MaybeMore`] for details.
    ///
    /// # Accumulator Poisoning Prevention
    ///
    /// **Accumulator poisoning** occurs when an unsupported or malformed sequence remains
    /// stuck at index 0 of `self.accumulator`. Because [`try_parse_input_event()`] always
    /// evaluates from the start of the accumulator, it repeatedly fails on this unparsed
    /// prefix and returns `None`. Every subsequent keystroke typed by the user (such as
    /// `'a'`, `Enter`, or `Ctrl+C`) is appended behind this stuck sequence, starving the
    /// application event loop of all input. The user experiences this as a completely
    /// frozen terminal interface even though the I/O thread is fully operational and idle
    /// at 0% CPU (neither an infinite loop nor a blocking read deadlock).
    ///
    /// Without the active inspection in [`UnparsedBufferClassification::classify()`], the
    /// parser would be susceptible to accumulator poisoning on any unhandled
    /// sequence. By detecting structurally completed but unhandled sequences
    /// ([`UnparsedBufferClassification::MalformedSequence`]) and immediately purging the
    /// accumulator via [`Vec::clear()`], subsequent keystrokes start fresh and
    /// accumulator poisoning is prevented.
    ///
    /// > This [article] has more details on mutable reborrowing. `&mut *self` breaks down
    /// > into:
    /// > - `*self`: Dereference the reference to access the struct in place (in memory).
    /// > - `&mut`: Fresh and temporary reborrow of the struct.
    ///
    /// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
    /// [`MioPollWorker`]:
    ///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::OscCircuitBreaker
    /// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
    /// [`stdin`]: std::io::stdin
    /// [`try_drain()`]: crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::OscCircuitBreaker::try_drain
    /// [`try_parse_input_event()`]:
    ///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
    /// [`UnparsedBufferClassification::classify()`]:
    ///     super::classification::UnparsedBufferClassification::classify
    /// [`UnparsedBufferClassification::MalformedSequence`]:
    ///     super::classification::UnparsedBufferClassification::MalformedSequence
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    /// [article]: https://developerlife.com/2026/09/25/rust-reborrowing/
    pub fn process_incoming_bytes(&mut self, read_buffer: &[u8], maybe_more: MaybeMore) {
        let parser = &mut *self; // Mutable reborrow.

        // Filter incoming bytes through the OSC circuit breaker before buffering:
        // - Closed (normal): all bytes pass through directly to the accumulator.
        // - Open (runaway OSC): payload bytes are swallowed without allocating; only
        //   valid trailing bytes following the terminator (BEL or ST) are buffered.
        parser.accumulator.extend_from_slice(
            parser
                .osc_circuit_breaker
                .try_drain(read_buffer)
                .undrained_bytes(),
        );

        loop {
            let acc = &parser.accumulator;
            if acc.is_empty() {
                break;
            }
            let maybe_input_event_ir = try_parse_input_event(acc, maybe_more);

            match maybe_input_event_ir {
                // Happy path - parser found an event.
                Some(ParsedInputEventIR {
                    event,
                    bytes_consumed,
                }) => {
                    let acc_mut = &mut parser.accumulator;
                    let events_mut = &mut parser.internal_events;

                    // bytes_consumed must be > 0 to advance the stream.
                    if bytes_consumed.is_zero() {
                        debug_assert!(
                            false,
                            "Parser must consume at least 1 byte to prevent infinite loops"
                        );
                        break;
                    }

                    // Don't push Ignored events into the internal events queue.
                    if event != VT100InputEventIR::Ignored {
                        events_mut.push_back(event);
                    }

                    // Consume the parsed bytes from the accumulator.
                    acc_mut.drain(..bytes_consumed.as_usize());
                }

                // Happy, Unhappy path - Make sure that accumulator poisoning (see above)
                // is prevented by distinguishing stream fragmentation (happy path), from
                // malformed sequences that would cause such poisoning (unhappy path).
                None => {
                    let class = UnparsedBufferClassification::classify(acc.as_slice());

                    let acc_mut = &mut parser.accumulator;
                    let breaker_mut = &mut parser.osc_circuit_breaker;

                    match class {
                        // Incomplete sequence: normal stream fragmentation, don't touch
                        // accumulator, and await more bytes on next read.
                        UnparsedBufferClassification::Incomplete => {
                            break;
                        }

                        // Malformed / unsupported sequence: structurally complete (e.g.
                        // CSI terminating in 0x40..=0x7E or
                        // length >= 64) but unhandled. Clear the
                        // accumulator to prevent accumulator poisoning from unhandled
                        // sequences, and await more bytes on next read.
                        UnparsedBufferClassification::MalformedSequence => {
                            DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                                // % is Display, ? is Debug.
                                tracing::warn! {
                                    message = "InputByteStreamToIrParser::process_incoming_bytes",
                                    status = "discarding unrecognized/malformed escape sequence",
                                    discarded_hex = %format!("{:02X?}", acc_mut),
                                    discarded_str = %String::from_utf8_lossy(acc_mut),
                                    buffer_len = acc_mut.len(),
                                };
                            });
                            acc_mut.clear();
                            break;
                        }

                        // Runaway OSC sequence (> 1 MiB): trip the circuit breaker into
                        // Open state and clear the accumulator to prevent unbounded
                        // memory growth. And await more bytes on next read.
                        UnparsedBufferClassification::RunawayOsc => {
                            let already_drained_byte_count = {
                                let acc_len = acc_mut.len().into();
                                acc_mut.clear();
                                acc_len
                            };
                            breaker_mut.trip(already_drained_byte_count);
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Iterator for InputByteStreamToIrParser {
    type Item = VT100InputEventIR;

    fn next(&mut self) -> Option<Self::Item> { self.internal_events.pop_front() }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for modified sequences (Shift+Home, Ctrl+Home, etc.), unrecognized sequence
//! purging, and [`OSC`] absorption / circuit-breaker streaming drain.
//!
//! [`OSC`]: crate::osc_codes::OscSequence

use super::test_fixtures::*;

// ======================================================================================
// Modified and unrecognized sequences
// ======================================================================================

#[test]
fn shift_home_parsing() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_modified(MODIFIER_SHIFT, SPECIAL_HOME_FINAL),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        keyboard_event_with_modifiers(VT100KeyCodeIR::Home, VT100KeyModifiersIR::SHIFT,)
    );
}

#[test]
fn ctrl_home_parsing() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_modified(MODIFIER_CTRL, SPECIAL_HOME_FINAL),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        keyboard_event_with_modifiers(VT100KeyCodeIR::Home, VT100KeyModifiersIR::CTRL,)
    );
}

#[test]
fn shift_end_parsing() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_modified(MODIFIER_SHIFT, SPECIAL_END_FINAL),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        keyboard_event_with_modifiers(VT100KeyCodeIR::End, VT100KeyModifiersIR::SHIFT,)
    );
}

#[test]
fn ctrl_end_parsing() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_modified(MODIFIER_CTRL, SPECIAL_END_FINAL),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        keyboard_event_with_modifiers(VT100KeyCodeIR::End, VT100KeyModifiersIR::CTRL,)
    );
}

#[test]
fn unrecognized_csi_does_not_block_subsequent_input() {
    let mut parser = ChunkFramer::default();

    // Send unrecognized CSI sequence (e.g., CSI 99 ; 99 z).
    let unrecognized_csi = [CSI_PREFIX, b"99;99z"].concat();
    parser.process_incoming_bytes(&unrecognized_csi, MaybeMore::KernelDrained);

    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 0);

    // Next character typed must be parsed cleanly without freeze.
    parser.process_incoming_bytes(b"a", MaybeMore::KernelDrained);
    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
}

#[test]
fn unrecognized_ss3_does_not_block_subsequent_input() {
    let mut parser = ChunkFramer::default();

    // Send unrecognized SS3 sequence (ESC O X).
    parser.process_incoming_bytes(&ss3(b'X'), MaybeMore::KernelDrained);

    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 0);

    // Next character typed must be parsed cleanly without freeze.
    parser.process_incoming_bytes(b"b", MaybeMore::KernelDrained);
    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('b')));
}

#[test]
fn safety_buffer_overflow_clears_buffer() {
    let mut parser = ChunkFramer::default();

    // Send a malformed unterminated escape sequence (ESC [ followed by 62 parameter
    // digits = 64 bytes).
    let mut long_unterminated = CSI_PREFIX.to_vec();
    long_unterminated.extend_from_slice(&[b'1'; 62]);
    parser.process_incoming_bytes(&long_unterminated, MaybeMore::KernelMayHaveMore);

    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 0);

    // Next character typed must be parsed cleanly.
    parser.process_incoming_bytes(b"c", MaybeMore::KernelDrained);
    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('c')));
}

// ======================================================================================
// OSC absorption and Alt+] handling
// ======================================================================================

fn alt_bracket() -> VT100InputEventIR {
    keyboard_event_with_modifiers(VT100KeyCodeIR::Char(']'), VT100KeyModifiersIR::ALT)
}

#[test]
fn lone_alt_bracket_single_and_split_reads() {
    // Single chunk:
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(OSC_PREFIX, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![alt_bracket()]);

    // Split reads: ESC in chunk 1 (KernelMayHaveMore), ] in chunk 2 (Drained)
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(&[ANSI_ESC], MaybeMore::KernelMayHaveMore);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    parser.process_incoming_bytes(b"]", MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![alt_bracket()]);
}

#[test]
fn alt_bracket_followed_by_multiple_characters_same_chunk() {
    let mut parser = ChunkFramer::default();
    let input = [OSC_PREFIX, b"abc"].concat();
    parser.process_incoming_bytes(&input, MaybeMore::KernelDrained);
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
    let mut parser = ChunkFramer::default();
    let input = [OSC_PREFIX, b"5"].concat();
    parser.process_incoming_bytes(&input, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(
        events,
        vec![alt_bracket(), keyboard_event(VT100KeyCodeIR::Char('5')),]
    );
}

#[test]
fn osc_color_reports_emitted_and_osc_52_absorbed() {
    let mut parser = ChunkFramer::default();
    let bel_seq = [OSC_PREFIX, b"11;rgb:0000/0000/0000", &[ANSI_BEL]].concat();
    parser.process_incoming_bytes(&bel_seq, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(
        events,
        vec![VT100InputEventIR::ColorReport(TerminalColorReport {
            role: TerminalColorRole::Background,
            color: RgbValue::from_u8(0, 0, 0),
        })]
    );

    let st_seq = [
        OSC_PREFIX,
        b"10;rgb:ffff/ffff/ffff",
        ANSI_ST_7BIT_TRANSPORT_ENCODING,
    ]
    .concat();
    parser.process_incoming_bytes(&st_seq, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(
        events,
        vec![VT100InputEventIR::ColorReport(TerminalColorReport {
            role: TerminalColorRole::Foreground,
            color: RgbValue::from_u8(255, 255, 255),
        })]
    );

    // Unhandled / OSC 52 sequences are absorbed with 0 events leaked.
    let osc52_seq = [OSC_PREFIX, b"52;c;SGVsbG8=", &[ANSI_BEL]].concat();
    parser.process_incoming_bytes(&osc52_seq, MaybeMore::KernelDrained);
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

    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(&long_osc, MaybeMore::KernelDrained);
    // Completely absorbed, zero events leaked, buffer drained.
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
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
    let mut parser = ChunkFramer::default();
    // UTF-8 checkmark ✓ contains 0x9C. Must be absorbed with 0 leakage.
    let seq = [OSC_PREFIX, b"52;c;\xe2\x9c\x93", &[ANSI_BEL]].concat();
    parser.process_incoming_bytes(&seq, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
}

#[test]
fn osc_followed_by_typing_same_chunk() {
    let mut parser = ChunkFramer::default();
    let seq = [OSC_PREFIX, b"0;title", &[ANSI_BEL, b'a']].concat();
    parser.process_incoming_bytes(&seq, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('a'))]);
}

#[test]
fn chunked_osc_sequence_split_across_reads() {
    let mut parser = ChunkFramer::default();
    // Chunk 1: prefix + partial payload, read drained (more == false)
    let chunk1 = [OSC_PREFIX, b"11;rgb:00"].concat();
    parser.process_incoming_bytes(&chunk1, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    // Chunk 2: rest of payload + terminator, read drained
    let chunk2 = [b"00/0000/0000".as_slice(), &[ANSI_BEL]].concat();
    parser.process_incoming_bytes(&chunk2, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(
        events,
        vec![VT100InputEventIR::ColorReport(TerminalColorReport {
            role: TerminalColorRole::Background,
            color: RgbValue::from_u8(0, 0, 0),
        })]
    );
    assert!(parser.accumulator_for_testing().is_empty());

    // Also verify unhandled OSC (e.g. OSC 52) split across reads is absorbed with 0
    // events leaked.
    let unhandled_chunk1 = [OSC_PREFIX, b"52;c;SGVs"].concat();
    parser.process_incoming_bytes(&unhandled_chunk1, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    let unhandled_chunk2 = [b"bG8=".as_slice(), &[ANSI_BEL]].concat();
    parser.process_incoming_bytes(&unhandled_chunk2, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
}

#[test]
fn chunked_osc_followed_by_typing_across_reads() {
    let mut parser = ChunkFramer::default();
    // Chunk 1: incomplete OSC
    let chunk1 = [OSC_PREFIX, b"0;ti"].concat();
    parser.process_incoming_bytes(&chunk1, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    // Chunk 2: end of OSC + user typed 'a'
    let chunk2 = [b"tle".as_slice(), &[ANSI_BEL, b'a']].concat();
    parser.process_incoming_bytes(&chunk2, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('a'))]);
}

#[test]
fn runaway_unterminated_osc_purged_and_recovers() {
    let mut parser = ChunkFramer::default();

    // Send unterminated OSC exceeding MAX_OSC_SEQUENCE_LENGTH
    let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
    runaway.extend_from_slice(OSC_PREFIX);
    runaway.extend_from_slice(b"52;");
    runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'x');

    parser.process_incoming_bytes(&runaway, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Open {
            already_drained_byte_count: byte_offset(runaway.len()),
        }
    );

    // Terminating the runaway sequence with BEL cleanly ends the drain and emits
    // typed 'z'
    parser.process_incoming_bytes(&[ANSI_BEL, b'z'], MaybeMore::KernelDrained);
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Closed
    );
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('z'))]);
}

#[test]
fn runaway_osc_draining_swallows_subsequent_chunks_until_bel() {
    let mut parser = ChunkFramer::default();

    // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
    let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
    runaway.extend_from_slice(OSC_PREFIX);
    runaway.extend_from_slice(b"52;");
    runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
    parser.process_incoming_bytes(&runaway, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
    assert!(matches!(
        parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Open { .. }
    ));

    // Chunk 2: trailing payload chunk in the pipe without terminator.
    // MUST be swallowed and discarded silently (0 events, accumulator remains empty).
    let trailing_chunk = vec![b'b'; 4096];
    parser.process_incoming_bytes(&trailing_chunk, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert!(parser.accumulator_for_testing().is_empty());
    assert!(matches!(
        parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Open { .. }
    ));

    // Chunk 3: trailing payload ending in BEL terminator, followed by human typing
    // "ok". BEL terminates the drain; "ok" is emitted as keystrokes.
    let term_chunk = [b"bbbb".as_slice(), &[ANSI_BEL], b"ok"].concat();
    parser.process_incoming_bytes(&term_chunk, MaybeMore::KernelDrained);
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Closed
    );
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
    let mut parser = ChunkFramer::default();

    // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
    let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
    runaway.extend_from_slice(OSC_PREFIX);
    runaway.extend_from_slice(b"52;");
    runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
    parser.process_incoming_bytes(&runaway, MaybeMore::KernelDrained);

    // Chunk 2: ends in lone ESC
    let chunk2 = [b"payload_data".as_slice(), &[ANSI_ESC]].concat();
    parser.process_incoming_bytes(&chunk2, MaybeMore::KernelDrained);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::OpenAwaitingSt {
            already_drained_byte_count: byte_offset(runaway.len() + chunk2.len()),
        }
    );

    // Chunk 3: begins with '\' completing 7-bit ST (ESC \), followed by typed 'w'
    parser.process_incoming_bytes(&[ANSI_ST_FINAL, b'w'], MaybeMore::KernelDrained);
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Closed
    );
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events, vec![keyboard_event(VT100KeyCodeIR::Char('w'))]);
}

#[test]
fn runaway_osc_draining_aborted_by_newline() {
    let mut parser = ChunkFramer::default();

    // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH
    let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
    runaway.extend_from_slice(OSC_PREFIX);
    runaway.extend_from_slice(b"52;");
    runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
    parser.process_incoming_bytes(&runaway, MaybeMore::KernelDrained);

    // Chunk 2: raw newline aborts OSC control string.
    // Newline is emitted as Enter, and subsequent characters as keystrokes.
    let chunk2 = [b"payload".as_slice(), &[LINE_FEED], b"hi"].concat();
    parser.process_incoming_bytes(&chunk2, MaybeMore::KernelDrained);
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Closed
    );
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
    let mut parser = ChunkFramer::default();

    // Chunk 1: exceeds MAX_OSC_SEQUENCE_LENGTH (starts drain with ~1 MiB drained)
    let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
    runaway.extend_from_slice(OSC_PREFIX);
    runaway.extend_from_slice(b"52;");
    runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
    parser.process_incoming_bytes(&runaway, MaybeMore::KernelDrained);

    // Chunk 2: massive unterminated chunk exceeding MAX_OSC_DRAIN_BYTES
    let massive_chunk = vec![b'b'; MAX_OSC_DRAIN_BYTES];
    parser.process_incoming_bytes(&massive_chunk, MaybeMore::KernelDrained);

    // Safety ceiling triggered: breaker resets to Closed
    assert_eq!(
        *parser.osc_circuit_breaker_for_testing(),
        OscCircuitBreaker::Closed
    );
}

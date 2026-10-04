// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for [`ESC`] key disambiguation ([`MaybeMore`]), chunked input
//! fragmentation, and iterator draining.
//!
//! [`ESC`]: crate::EscSequence
//! [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore

use super::test_fixtures::*;

// ======================================================================================
// ESC disambiguation
// ======================================================================================

#[test]
fn lone_esc_with_more_false_emits_escape_key() {
    // User pressed ESC key alone - no more data coming.
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(&[ANSI_ESC], MaybeMore::KernelDrained); // ESC byte, drained

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Escape));
}

#[test]
fn esc_with_more_true_waits_for_sequence() {
    // ESC arrived but more bytes are coming - wait for full sequence.
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(&[ANSI_ESC], MaybeMore::KernelMayHaveMore); // ESC byte, kernel may have more

    // No event emitted yet - waiting for rest of sequence.
    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 0);
}

#[test]
fn arrow_up_complete_sequence() {
    // Arrow Up: ESC [ A
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(SEQ_ARROW_UP, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
}

#[test]
fn arrow_down_complete_sequence() {
    // Arrow Down: ESC [ B
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(SEQ_ARROW_DOWN, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Down));
}

#[test]
fn arrow_right_complete_sequence() {
    // Arrow Right: ESC [ C
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(SEQ_ARROW_RIGHT, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Right));
}

#[test]
fn arrow_left_complete_sequence() {
    // Arrow Left: ESC [ D
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(SEQ_ARROW_LEFT, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Left));
}

// ======================================================================================
// Chunked input fragmentation
// ======================================================================================

#[test]
fn arrow_key_split_across_two_reads() {
    // Arrow Up arrives as: first read gets ESC, second read gets [ A.
    let mut parser = InputByteStreamToIrParser::default();

    // First chunk: ESC only, but more anticipated (kernel read buffer was full).
    parser.process_incoming_bytes(&[SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0); // No event yet

    // Second chunk: [ A completes the sequence.
    parser.process_incoming_bytes(&SEQ_ARROW_UP[1..], MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
}

#[test]
fn arrow_key_split_into_three_reads() {
    // Extreme fragmentation: ESC, then [, then A.
    let mut parser = InputByteStreamToIrParser::default();

    parser.process_incoming_bytes(&[SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    parser.process_incoming_bytes(&[SEQ_ARROW_UP[1]], MaybeMore::KernelMayHaveMore);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    parser.process_incoming_bytes(&[SEQ_ARROW_UP[2]], MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
}

#[test]
fn multiple_events_across_chunks() {
    let mut parser = InputByteStreamToIrParser::default();

    // First chunk: 'a' and start of arrow sequence.
    parser.process_incoming_bytes(&[b'a', SEQ_ARROW_UP[0]], MaybeMore::KernelMayHaveMore);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));

    // Second chunk: completes arrow, adds 'b'.
    let second_chunk = [&SEQ_ARROW_UP[1..], b"b"].concat();
    parser.process_incoming_bytes(&second_chunk, MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Up));
    assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
}

// ======================================================================================
// Iterator implementation
// ======================================================================================

#[test]
fn iterator_drains_internal_queue() {
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(b"xyz", MaybeMore::KernelDrained);

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
    parser.process_incoming_bytes(b"abc", MaybeMore::KernelDrained);

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

    parser.process_incoming_bytes(b"a", MaybeMore::KernelDrained);
    assert_eq!(
        parser.next(),
        Some(keyboard_event(VT100KeyCodeIR::Char('a')))
    );

    parser.process_incoming_bytes(b"b", MaybeMore::KernelDrained);
    assert_eq!(
        parser.next(),
        Some(keyboard_event(VT100KeyCodeIR::Char('b')))
    );

    assert_eq!(parser.next(), None);
}

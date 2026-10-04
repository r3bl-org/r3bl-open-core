// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for multi-byte [`UTF-8`] sequences and emojis.
//!
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8

use super::test_fixtures::*;

#[test]
fn two_byte_utf8_char() {
    // 'é' is U+00E9, encoded as C3 A9
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(&[0xC3, 0xA9], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
}

#[test]
fn three_byte_utf8_char() {
    // '中' is U+4E2D, encoded as E4 B8 AD
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(&[0xE4, 0xB8, 0xAD], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('中')));
}

#[test]
fn four_byte_utf8_emoji() {
    // '😀' is U+1F600, encoded as F0 9F 98 80
    let mut parser = InputByteStreamToIrParser::default();
    parser.process_incoming_bytes(&[0xF0, 0x9F, 0x98, 0x80], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('😀')));
}

#[test]
fn utf8_split_across_chunks() {
    // 'é' split across two reads
    let mut parser = InputByteStreamToIrParser::default();

    parser.process_incoming_bytes(&[0xC3], MaybeMore::KernelMayHaveMore);
    assert_eq!((&mut parser).collect::<Vec<_>>().len(), 0);

    parser.process_incoming_bytes(&[0xA9], MaybeMore::KernelDrained);
    let events: Vec<_> = (&mut parser).collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('é')));
}

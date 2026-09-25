// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unit tests for basic [`ASCII`] keystrokes and special keys (arrows, home/end, page
//! up/down, insert/delete).
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII

use super::test_fixtures::*;

#[test]
fn single_ascii_char() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(b"a", MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
}

#[test]
fn multiple_ascii_chars_single_read() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(b"abc", MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Char('a')));
    assert_eq!(events[1], keyboard_event(VT100KeyCodeIR::Char('b')));
    assert_eq!(events[2], keyboard_event(VT100KeyCodeIR::Char('c')));
}

#[test]
fn enter_key() {
    let mut parser = ChunkFramer::default();
    // In raw mode, Enter sends CR (0D), not LF (0A).
    // The kernel's line discipline translates CR→LF, but raw mode bypasses this.
    parser.process_incoming_bytes(&[CONTROL_ENTER], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Enter));
}

#[test]
fn tab_key() {
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(&[CONTROL_TAB], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Tab));
}

#[test]
fn backspace_key() {
    let mut parser = ChunkFramer::default();
    // Historical quirk: Backspace key sends DEL (7F), not BS (08).
    // DEC VT100 reserved BS for cursor-left; most terminals inherited this.
    parser.process_incoming_bytes(&[ASCII_DEL], MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Backspace));
}

#[test]
fn home_key() {
    // Home: ESC [ H
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(SEQ_HOME, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Home));
}

#[test]
fn end_key() {
    // End: ESC [ F
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(SEQ_END, MaybeMore::KernelDrained);

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::End));
}

#[test]
fn delete_key() {
    // Delete: ESC [ 3 ~
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_tilde(SPECIAL_DELETE_CODE),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Delete));
}

#[test]
fn insert_key() {
    // Insert: ESC [ 2 ~
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_tilde(SPECIAL_INSERT_CODE),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::Insert));
}

#[test]
fn page_up_key() {
    // Page Up: ESC [ 5 ~
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_tilde(SPECIAL_PAGE_UP_CODE),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageUp));
}

#[test]
fn page_down_key() {
    // Page Down: ESC [ 6 ~
    let mut parser = ChunkFramer::default();
    parser.process_incoming_bytes(
        &csi_tilde(SPECIAL_PAGE_DOWN_CODE),
        MaybeMore::KernelDrained,
    );

    let events: Vec<_> = parser.collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], keyboard_event(VT100KeyCodeIR::PageDown));
}

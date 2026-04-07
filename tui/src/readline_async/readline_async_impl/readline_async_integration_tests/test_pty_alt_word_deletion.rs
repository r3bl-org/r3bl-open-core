// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Alt+D and Alt+Backspace word killing.
//!
//! Validates that Alt+D (kill word forward) and Alt+Backspace (kill word backward)
//! correctly delete words at word boundaries.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_alt_word_deletion -- --nocapture
//! ```
//!
//! Tests:
//! 1. Alt+D: Delete word forward from cursor
//! 2. Alt+Backspace: Delete word backward from cursor
//! 3. Word deletion with punctuation boundaries
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! [`LineState`]: crate::readline_async::readline_async_impl::LineState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async_controlled_loop()`]: super::readline_async_pty_test_fixtures::readline_async_controlled_loop

use crate::{MSG_CONTROLLED_READY, GLYPH_CONTROLLER, GLYPH_WAITING, MSG_LINE_PREFIX,
            MSG_CONTROLLED_STARTING, PtyTestContext, PtyTestMode,
            generate_pty_test, seg_index,
            readline_async::readline_async_impl::readline_async_integration_tests::readline_async_pty_test_fixtures::{
                readline_async_controlled_loop, readline_async_controller_exit
            }};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_alt_word_deletion,
    controller: controller,
    controlled: readline_async_controlled_loop("hello world test", seg_index(16)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Alt+D/Backspace sequences and verify word deletion.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Alt+D/Backspace test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context
        .child
        .wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING)
        .unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context
        .child
        .wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY)
        .unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // Test 1: Alt+D to delete word forward
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 1 - Alt+D to delete word forward...");

    // Send "hello world test"
    context
        .writer
        .write_all(b"hello world test")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 16");

    // Move to start with Ctrl+A
    context
        .writer
        .write_all(&[0x01])
        .expect("Failed to write Ctrl+A");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← After Ctrl+A: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 0");

    // Alt+D to delete "hello" Alt+D is ESC d
    context
        .writer
        .write_all(b"\x1bd")
        .expect("Failed to write Alt+D");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← After Alt+D: {result}");
    assert_eq!(result, "Line:  world test, Cursor: 0");

    // Test 2: Alt+Backspace to delete word backward
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test 2 - Alt+Backspace to delete word backward..."
    );

    // Move cursor to end with Ctrl+E, then clear with Ctrl+U
    context
        .writer
        .write_all(&[0x05])
        .expect("Failed to write Ctrl+E");
    context.writer.flush().expect("Failed to flush");

    let _result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });

    context
        .writer
        .write_all(&[0x15])
        .expect("Failed to write Ctrl+U");
    context.writer.flush().expect("Failed to flush");

    let _result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });

    context
        .writer
        .write_all(b"one two three")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← New line: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 13");

    // Alt+Backspace to delete "three" Alt+Backspace is ESC DEL (0x7f)
    context
        .writer
        .write_all(b"\x1b\x7f")
        .expect("Failed to write Alt+Backspace");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← After Alt+Backspace: {result}");
    assert_eq!(result, "Line: one two , Cursor: 8");

    // Test 3: Another Alt+Backspace to delete "two"
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 3 - Another Alt+Backspace...");
    context
        .writer
        .write_all(b"\x1b\x7f")
        .expect("Failed to write Alt+Backspace");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
        });
    eprintln!("  ← After Alt+Backspace: {result}");
    assert_eq!(result, "Line: one , Cursor: 4");

    readline_async_controller_exit(context);
}

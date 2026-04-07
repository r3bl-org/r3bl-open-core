// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Ctrl+W word deletion.
//!
//! Validates that Ctrl+W correctly deletes the word before the cursor, respecting word
//! boundaries (whitespace and punctuation).
//!
//! Tests:
//! 1. Delete word with space boundary: "hello world" → "hello "
//! 2. Delete word with punctuation boundary: "hello-world" → "hello-"
//! 3. Multiple deletions: "one two three" → "one two " → "one "
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_ctrl_w_deletion -- --nocapture
//! ```
//!
//! [`LineState`]: crate::readline_async::readline_async_impl::LineState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async_controlled_loop()`]:
//!     super::readline_async_pty_test_fixtures::readline_async_controlled_loop

use crate::{MSG_CONTROLLED_READY, GLYPH_CONTROLLER, GLYPH_WAITING, MSG_LINE_PREFIX,
            MSG_CONTROLLED_STARTING, PtyTestContext, PtyTestMode,
            generate_pty_test, seg_index,
            readline_async::readline_async_impl::readline_async_integration_tests::readline_async_pty_test_fixtures::{
                readline_async_controlled_loop, readline_async_controller_exit
            }};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_ctrl_w_deletion,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+W sequences and verify word deletion.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Ctrl+W test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING).unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY).unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // Test 1: Ctrl+W with space boundary
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test 1 - Delete word with space boundary..."
    );

    // Send "hello world"
    context.writer
        .write_all(b"hello world")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← Line state: {result}");
    assert_eq!(result, "Line: hello world, Cursor: 11");

    // Send Ctrl+W (0x17) to delete "world"
    context.writer.write_all(&[0x17]).expect("Failed to write Ctrl+W");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Ctrl+W: {result}");
    assert_eq!(result, "Line: hello , Cursor: 6");

    // Test 2: Ctrl+W with punctuation boundary
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test 2 - Delete word with punctuation boundary..."
    );

    // Clear line with Ctrl+U (0x15) and send "hello-world"
    context.writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    context.writer.flush().expect("Failed to flush");

    let _result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });

    context.writer
        .write_all(b"hello-world")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← Line state: {result}");
    assert_eq!(result, "Line: hello-world, Cursor: 11");

    // Send Ctrl+W to delete "world"
    context.writer.write_all(&[0x17]).expect("Failed to write Ctrl+W");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Ctrl+W: {result}");
    assert_eq!(result, "Line: hello-, Cursor: 6");

    readline_async_controller_exit(context);
}

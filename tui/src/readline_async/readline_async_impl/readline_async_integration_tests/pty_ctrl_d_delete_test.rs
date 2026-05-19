// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words ello

//! [`PTY`]-based integration test for Ctrl+D delete character behavior.
//!
//! Validates that Ctrl+D on a non-empty line deletes the character at cursor position.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_ctrl_d_delete -- --nocapture
//! ```
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! [`LineState`]: crate::readline_async::readline_async_impl::LineState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async_controlled_loop()`]:
//!     super::readline_async_pty_test_fixtures::readline_async_controlled_loop

use crate::{MSG_CONTROLLED_READY, GLYPH_CONTROLLER, GLYPH_SUCCESS, GLYPH_WAITING,
            MSG_LINE_PREFIX, MSG_CONTROLLED_STARTING, PtyTestContext, PtyTestMode,
            generate_pty_test, seg_index,
            readline_async::readline_async_impl::readline_async_integration_tests::readline_async_pty_test_fixtures::{
                readline_async_controlled_loop, readline_async_controller_exit
            }};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_ctrl_d_delete,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+D on non-empty line and verify delete behavior.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Ctrl+D delete test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING).unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY).unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // Test: Ctrl+D on non-empty line → delete character at cursor
    eprintln!("{GLYPH_WAITING} PTY Controller: Sending 'hello'...");
    context.writer.write_all(b"hello").expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← Line state: {result}");
    assert_eq!(result, "Line: hello, Cursor: 5");

    // Move cursor to beginning with Ctrl+A
    eprintln!("{GLYPH_WAITING} PTY Controller: Sending Ctrl+A (move to beginning)...");
    context.writer.write_all(&[0x01]).expect("Failed to write Ctrl+A");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← After Ctrl+A: {result}");
    assert_eq!(result, "Line: hello, Cursor: 0");

    // Send Ctrl+D to delete 'h'
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Sending Ctrl+D (delete character at cursor)..."
    );
    context.writer.write_all(&[0x04]).expect("Failed to write Ctrl+D");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← After Ctrl+D: {result}");
    assert_eq!(result, "Line: ello, Cursor: 0");

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Ctrl+D delete test passed!");

    readline_async_controller_exit(context);
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Ctrl+U line clearing behavior.
//!
//! Validates that Ctrl+U correctly clears from the start of the line to the cursor
//! position.
//!
//! # Test Cases
//!
//! 1. **Cursor at position 0**: Ctrl+U deletes nothing (0 to 0)
//! 2. **Cursor at the end**: Ctrl+U deletes entire line (start to cursor at end)
//!
//! Note: We don't test "cursor in middle" as that would require navigation commands
//! (Alt+B, Ctrl+Left, arrow keys, etc.) which violates Separation of Concerns. The two
//! cases above cover the boundary conditions for Ctrl+U behavior.
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_ctrl_u -- --nocapture
//! ```
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
    test_fn: test_pty_ctrl_u,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+U sequences and verify line clearing behavior.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Ctrl+U test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING).unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY).unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // Test Case 1: Ctrl+U with cursor at the end (deletes entire line)
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test Case 1 - Ctrl+U with cursor at end..."
    );

    // Type "hello world" which naturally leaves cursor at end
    context.writer
        .write_all(b"hello world")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← Line with cursor at end: {result}");
    assert_eq!(result, "Line: hello world, Cursor: 11");

    // Ctrl+U at end should delete entire line
    context.writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← After Ctrl+U (cursor at end): {result}");
    assert_eq!(
        result, "Line: , Cursor: 0",
        "Ctrl+U at end should delete entire line"
    );

    // Test Case 2: Ctrl+U with cursor at position 0 (deletes nothing)
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test Case 2 - Ctrl+U with cursor at position 0..."
    );

    // Now line is empty and cursor is at position 0 Ctrl+U at position 0 should still
    // delete nothing
    context.writer.write_all(&[0x15]).expect("Failed to write Ctrl+U");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX)
    });
    eprintln!("  ← After Ctrl+U on empty line: {result}");
    assert_eq!(
        result, "Line: , Cursor: 0",
        "Ctrl+U on empty line should delete nothing"
    );

    eprintln!("{GLYPH_SUCCESS} PTY Controller: All Ctrl+U test cases passed!");

    readline_async_controller_exit(context);
}


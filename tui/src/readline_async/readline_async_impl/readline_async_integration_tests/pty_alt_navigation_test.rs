// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Alt+B/F word navigation.
//!
//! Validates that Alt+B (backward) and Alt+F (forward) correctly move the cursor to word
//! boundaries, providing bash-compatible word navigation.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_alt_navigation -- --nocapture
//! ```
//!
//! Tests:
//! 1. Alt+B: Move backward one word
//! 2. Alt+F: Move forward one word
//! 3. Multiple navigations across word boundaries
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! [`ESC`]: crate::EscSequence
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
    test_fn: test_pty_alt_navigation,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Alt+B/F sequences and verify navigation.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Alt+B/F test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING).unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY).unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // Setup: Send "one two three"
    eprintln!("{GLYPH_WAITING} PTY Controller: Setting up line...");
    context.writer
        .write_all(b"one two three")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 13");

    // Test 1: Alt+B to move backward to "two"
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 1 - Alt+B to start of 'three'...");

    // Alt+B is ESC b
    context.writer.write_all(b"\x1bb").expect("Failed to write Alt+B");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Alt+B: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 8");

    // Test 2: Another Alt+B to move to "one"
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 2 - Alt+B to start of 'two'...");
    context.writer.write_all(b"\x1bb").expect("Failed to write Alt+B");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Alt+B: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 4");

    // Test 3: Alt+F to move forward to "two"
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 3 - Alt+F to start of 'three'...");

    // Alt+F is ESC f
    context.writer.write_all(b"\x1bf").expect("Failed to write Alt+F");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Alt+F: {result}");
    assert_eq!(result, "Line: one two three, Cursor: 8");

    readline_async_controller_exit(context);
}

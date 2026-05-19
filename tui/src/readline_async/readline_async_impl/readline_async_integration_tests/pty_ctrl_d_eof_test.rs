// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Ctrl+D [`EOF`] behavior on empty line.
//!
//! Validates that Ctrl+D on an empty line returns [`EOF`] ([`ReadlineEvent::Eof`]).
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_ctrl_d_eof -- --nocapture
//! ```
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`LineState`]: crate::readline_async::readline_async_impl::LineState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async_controlled_loop()`]:
//!     super::readline_async_pty_test_fixtures::readline_async_controlled_loop
//! [`ReadlineEvent::Eof`]: crate::ReadlineEvent::Eof

use crate::{MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, GLYPH_CONTROLLER,
            GLYPH_SUCCESS, GLYPH_WAITING, MSG_LINE_PREFIX, PtyTestContext, PtyTestMode,
            generate_pty_test, seg_index,
            readline_async::readline_async_impl::readline_async_integration_tests::readline_async_pty_test_fixtures::{
                readline_async_controlled_loop, readline_async_controller_exit
            }};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_ctrl_d_eof,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+D on empty line and verify [`EOF`].
///
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Ctrl+D EOF test...");

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

    // Test: Ctrl+D on empty line → EOF
    eprintln!("{GLYPH_WAITING} PTY Controller: Sending Ctrl+D on empty line...");
    context
        .writer
        .write_all(&[0x04])
        .expect("Failed to write Ctrl+D");
    context.writer.flush().expect("Failed to flush");

    let result = context
        .child
        .read_line_state(&mut context.buf_reader, |line| {
            line.starts_with(MSG_LINE_PREFIX) || line.contains(EOF_SIGNAL)
        });
    eprintln!("  ← Controlled response: {result}");
    assert!(
        result.contains(EOF_SIGNAL),
        "Expected {EOF_SIGNAL}, got: {result}"
    );

    eprintln!("{GLYPH_SUCCESS} PTY Controller: Ctrl+D EOF test passed!");

    readline_async_controller_exit(context);
}

/// [`EOF`] signal.
///
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
const EOF_SIGNAL: &str = "EOF";

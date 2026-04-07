// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for Ctrl+Left/Right word navigation.
//!
//! Validates that Ctrl+Left and Ctrl+Right correctly move the cursor to word boundaries,
//! respecting whitespace and punctuation.
//!
//! Tests:
//! 1. Ctrl+Left: Move to start of previous word
//! 2. Ctrl+Right: Move to start of next word
//! 3. Multiple navigations across word boundaries
//!
//! # Test Protocol (Request-Response Pattern)
//!
//! This test follows the **request-response protocol** defined in
//! [`readline_async_controlled_loop()`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_ctrl_navigation -- --nocapture
//! ```
//!
//! [`ESC`]: crate::EscSequence
//! [`LineState`]: crate::readline_async::readline_async_impl::LineState
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async_controlled_loop()`]:
//!     super::readline_async_pty_test_fixtures::readline_async_controlled_loop

use crate::{GLYPH_CONTROLLER, GLYPH_WAITING, KeyState, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, MSG_LINE_PREFIX, PtyTestContext, PtyTestMode,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::{VT100InputEventIR,
                                                        VT100KeyCodeIR,
                                                        VT100KeyModifiersIR}},
            generate_pty_test, seg_index,
            readline_async::readline_async_impl::readline_async_integration_tests::readline_async_pty_test_fixtures::{
                readline_async_controlled_loop, readline_async_controller_exit
            }};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_ctrl_navigation,
    controller: controller,
    controlled: readline_async_controlled_loop("", seg_index(0)),
    mode: PtyTestMode::Raw,
}

/// [`PTY`] Controller: Send Ctrl+Left/Right sequences and verify navigation.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[allow(clippy::too_many_lines)]
fn controller(mut context: PtyTestContext) {
    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting Ctrl+Left/Right test...");

    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Waiting for controlled process to start..."
    );

    // Wait for controlled process to start.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_STARTING).unwrap();
    eprintln!("  ✅ Controlled process confirmed running!");

    // Wait for controlled process to be ready.
    context.child.wait_for_ready(&mut context.buf_reader, MSG_CONTROLLED_READY).unwrap();
    eprintln!("  ✅ Controlled is ready (input device created)");

    // ==================== Setup: Send "hello world test" ====================
    eprintln!("{GLYPH_WAITING} PTY Controller: Setting up line...");
    context.writer
        .write_all(b"hello world test")
        .expect("Failed to write text");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← Initial line: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 16");

    // ==================== Test 1: Ctrl+Left to move to start of "test"
    // ====================
    eprintln!("{GLYPH_WAITING} PTY Controller: Test 1 - Ctrl+Left to start of 'test'...");
    context.writer
        .write_all(&ctrl_left())
        .expect("Failed to write Ctrl+Left");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Ctrl+Left: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 12");

    // ==================== Test 2: Another Ctrl+Left to move to start of "world"
    // ====================
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test 2 - Ctrl+Left to start of 'world'..."
    );
    context.writer
        .write_all(&ctrl_left())
        .expect("Failed to write Ctrl+Left");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Ctrl+Left: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 6");

    // ==================== Test 3: Ctrl+Right to move to start of "test"
    // ====================
    eprintln!(
        "{GLYPH_WAITING} PTY Controller: Test 3 - Ctrl+Right to start of 'test'..."
    );
    context.writer
        .write_all(&ctrl_right())
        .expect("Failed to write Ctrl+Right");
    context.writer.flush().expect("Failed to flush");

    let result = context.child.read_line_state(&mut context.buf_reader, |line| {
        line.starts_with(MSG_LINE_PREFIX) || line.contains("EOF")
    });
    eprintln!("  ← After Ctrl+Right: {result}");
    assert_eq!(result, "Line: hello world test, Cursor: 12");

    readline_async_controller_exit(context);
}

// ==================== Test Input Sequences ====================
//
// These helper functions generate ANSI escape sequences using the VT100 input generator.
// This ensures the test sends the exact same sequences that the parser expects.

/// Ctrl+Left: Move cursor one word backward Generates: [`ESC`] [ 1 ; 5 D
///
/// [`ESC`]: crate::EscSequence
fn ctrl_left() -> Vec<u8> {
    generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Left,
        modifiers: VT100KeyModifiersIR {
            ctrl: KeyState::Pressed,
            shift: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        },
    })
    .expect("Ctrl+Left should generate valid sequence")
}

/// Ctrl+Right: Move cursor one word forward Generates: [`ESC`] [ 1 ; 5 C
///
/// [`ESC`]: crate::EscSequence
fn ctrl_right() -> Vec<u8> {
    generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Right,
        modifiers: VT100KeyModifiersIR {
            ctrl: KeyState::Pressed,
            shift: KeyState::NotPressed,
            alt: KeyState::NotPressed,
        },
    })
    .expect("Ctrl+Right should generate valid sequence")
}

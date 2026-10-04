// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Verifies that all three interactivity checks return [`IsInteractive`] when running
//! inside a real [`PTY`].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_is_interactive -- --nocapture
//! ```
//!
//! [`IsInteractive`]: crate::TTYResult::IsInteractive
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{PtyTestContext, PtyTestMode, TTYResult, generate_pty_test,
            is_fully_interactive, is_input_interactive, is_output_interactive};
use std::io::{BufRead, Write};

generate_pty_test! {
    test_fn: test_pty_is_interactive,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    let mut tests_passed = 0;
    let expected_tests = 3;

    for _ in 0..100 {
        let mut line = String::new();
        match buf_reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  <- Controlled output: {trimmed}");
                if trimmed.contains("SUCCESS:") {
                    tests_passed += 1;
                }
                assert!(
                    !trimmed.contains("FAILED:"),
                    "Test failed in controlled process: {trimmed}"
                );
            }
        }
        if tests_passed == expected_tests {
            break;
        }
    }

    assert_eq!(
        tests_passed, expected_tests,
        "Not all interactivity tests passed"
    );
    child.drain_and_wait(buf_reader, pty_pair);
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let is_input_interactive = is_input_interactive() == TTYResult::IsInteractive;
    let is_output_interactive = is_output_interactive() == TTYResult::IsInteractive;
    let is_fully_interactive = is_fully_interactive() == TTYResult::IsInteractive;

    if is_input_interactive {
        println!("SUCCESS: is_input_interactive");
    } else {
        println!("FAILED: is_input_interactive returned IsNotInteractive");
    }

    if is_output_interactive {
        println!("SUCCESS: is_output_interactive");
    } else {
        println!("FAILED: is_output_interactive returned IsNotInteractive");
    }

    if is_fully_interactive {
        println!("SUCCESS: is_fully_interactive");
    } else {
        println!("FAILED: is_fully_interactive returned IsNotInteractive");
    }

    std::io::stdout().flush().ok();
    let all_ok = is_input_interactive && is_output_interactive && is_fully_interactive;
    std::process::exit(i32::from(!all_ok));
}

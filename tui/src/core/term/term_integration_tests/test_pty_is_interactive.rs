// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{PtyTestContext, PtyTestMode, TTYResult, is_fully_interactive,
            is_input_interactive, is_output_interactive};
use std::io::{BufRead, Write};

generate_pty_test! {
    /// Verifies that all three interactivity checks return [`IsInteractive`] when
    /// running inside a real [`PTY`].
    ///
    /// [`IsInteractive`]: TTYResult::IsInteractive
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    test_fn: test_pty_is_interactive,
    controller: pty_controller_entry_point,
    controlled: pty_controlled_entry_point,
    mode: PtyTestMode::Cooked,
}

fn pty_controller_entry_point(context: PtyTestContext) {
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

fn pty_controlled_entry_point() -> ! {
    let input_ok = is_input_interactive() == TTYResult::IsInteractive;
    let output_ok = is_output_interactive() == TTYResult::IsInteractive;
    let full_ok = is_fully_interactive() == TTYResult::IsInteractive;

    if input_ok {
        println!("SUCCESS: is_input_interactive");
    } else {
        println!("FAILED: is_input_interactive returned IsNotInteractive");
    }

    if output_ok {
        println!("SUCCESS: is_output_interactive");
    } else {
        println!("FAILED: is_output_interactive returned IsNotInteractive");
    }

    if full_ok {
        println!("SUCCESS: is_fully_interactive");
    } else {
        println!("FAILED: is_fully_interactive returned IsNotInteractive");
    }

    std::io::stdout().flush().ok();
    let all_ok = input_ok && output_ok && full_ok;
    std::process::exit(i32::from(!all_ok));
}

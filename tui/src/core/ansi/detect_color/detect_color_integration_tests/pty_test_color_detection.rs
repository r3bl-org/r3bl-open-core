// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Verifies that [`examine_env_vars_to_determine_color_support()`] correctly detects
//! color support when running inside a real terminal environment.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_color_detection_in_pty -- --nocapture
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{ColorSupport, PtyTestContext, PtyTestMode, Stream,
            examine_env_vars_to_determine_color_support, generate_pty_test};
use std::io::{BufRead, Write};

generate_pty_test! {
    test_fn: test_color_detection_in_pty,
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

    let mut success = false;
    for _ in 0..10 {
        let mut line = String::new();
        if buf_reader.read_line(&mut line).is_err() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.contains("SUCCESS: Detected color in PTY") {
            success = true;
            break;
        }
        assert!(
            !trimmed.contains("FAILED:"),
            "Test failed in controlled process: {trimmed}"
        );
    }
    assert!(
        success,
        "Did not receive success message from controlled process"
    );
    child.drain_and_wait(buf_reader, pty_pair);
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    // Ensure no env vars override TTY detection.
    unsafe {
        std::env::remove_var("NO_COLOR");
        std::env::remove_var("FORCE_COLOR");
        std::env::set_var("COLORTERM", "truecolor");
    }

    let result = examine_env_vars_to_determine_color_support(Stream::Stdout);
    if result == ColorSupport::Truecolor {
        println!("SUCCESS: Detected color in PTY");
    } else {
        println!("FAILED: Detected {result:?} in PTY");
    }
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

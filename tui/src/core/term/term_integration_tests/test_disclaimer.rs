// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{emit_stderr_redirection_disclaimer, generate_isolated_process_test};
use std::io::Write;

// XMARK: Process isolated test.

generate_isolated_process_test!(
    /// Verifies that [`emit_stderr_redirection_disclaimer()`] works as expected:
    /// 1. It prints a note to [`stderr`] when [`stderr`] is redirected.
    /// 2. It is idempotent (only prints once per process lifetime).
    ///
    /// Run this test with:
    /// ```bash
    /// cargo test -- --nocapture test_disclaimer_is_emitted_exactly_once
    /// ```
    ///
    /// [`emit_stderr_redirection_disclaimer()`]: crate::emit_stderr_redirection_disclaimer
    /// [`stderr`]: std::io::stderr
    test_disclaimer_is_emitted_exactly_once,
    controller,
    controlled,
    std::process::Stdio::null(), // stdin
    std::process::Stdio::null(), // stdout
    std::process::Stdio::piped() // stderr
);

fn controller(spawned_self_process_output: std::process::Output) {
    let stderr = String::from_utf8_lossy(&spawned_self_process_output.stderr);
    eprintln!("  child stderr: {stderr}");

    let status = spawned_self_process_output.status;
    assert!(
        status.success(),
        "Child process failed with status: {status:?}"
    );

    let disclaimer_note = "Note: stderr is redirected.";
    let matches: Vec<_> = stderr.matches(disclaimer_note).collect();

    assert_eq!(
        matches.len(),
        1,
        "Expected exactly 1 disclaimer note in stderr, but found {}. Stderr content: \n{}",
        matches.len(),
        stderr
    );
}

/// Controlled path: stderr is redirected, call disclaimer twice and exit. The
/// harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    // Call it twice to test idempotency.
    emit_stderr_redirection_disclaimer();
    emit_stderr_redirection_disclaimer();

    std::io::stderr().flush().ok();
}

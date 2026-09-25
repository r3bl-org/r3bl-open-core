// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{generate_isolated_process_test, is_output_interactive};
use std::io::Write;

generate_isolated_process_test!(
    /// Verifies that [`is_output_interactive()`] returns [`IsNotInteractive`] when
    /// [`stdout`] is a pipe (not a [`TTY`]).
    ///
    /// Run this test with:
    /// ```bash
    /// cargo test -- --nocapture test_piped_stdout_is_interactive
    /// ```
    ///
    /// [`is_output_interactive()`]: crate::is_output_interactive
    /// [`IsNotInteractive`]: TTYResult::IsNotInteractive
    /// [`stdout`]: std::io::stdout
    /// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
    test_piped_stdout_is_interactive,
    controller,
    controlled,
    std::process::Stdio::null(),
    std::process::Stdio::piped(),
    std::process::Stdio::piped()
);

fn controller(spawned_self_process_output: std::process::Output) {
    let stdout = String::from_utf8_lossy(&spawned_self_process_output.stdout);
    let stderr = String::from_utf8_lossy(&spawned_self_process_output.stderr);
    eprintln!("  child stdout: {stdout}");
    eprintln!("  child stderr: {stderr}");

    let status = spawned_self_process_output.status;
    assert!(
        status.success(),
        "Child process failed with status: {status:?}"
    );
    assert!(
        stdout.contains("IsNotInteractive"),
        "Expected IsNotInteractive in stdout, got: {stdout}"
    );
}

/// Controlled path: stdout is piped, report the result and exit. The harness
/// performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let result = is_output_interactive();
    println!("{result:?}");
    std::io::stdout().flush().ok();
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{generate_isolated_process_test, is_input_interactive};
use std::io::Write;

generate_isolated_process_test!(
    /// Verifies that [`is_input_interactive()`] returns [`IsNotInteractive`] when
    /// [`stdin`] is `/dev/null` (not a [`TTY`]).
    ///
    /// Run this test with:
    /// ```bash
    /// cargo test -- --nocapture test_piped_stdin_is_not_interactive
    /// ```
    ///
    /// [`is_input_interactive()`]: crate::is_input_interactive
    /// [`IsNotInteractive`]: TTYResult::IsNotInteractive
    /// [`stdin`]: std::io::stdin
    /// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
    test_piped_stdin_is_not_interactive,
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

    assert!(
        spawned_self_process_output.status.success(),
        "Child process failed with status: {:?}",
        spawned_self_process_output.status
    );
    assert!(
        stdout.contains("IsNotInteractive"),
        "Expected IsNotInteractive in stdout, got: {stdout}"
    );
}

/// Controlled path: [`stdin`] is `/dev/null`, report the result and exit. The
/// harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let result = is_input_interactive();
    println!("{result:?}");
    std::io::stdout().flush().ok();
}

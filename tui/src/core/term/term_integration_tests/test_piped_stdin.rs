// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::is_input_interactive;
use std::io::Write;

const ENV_VAR: &str = "R3BL_TEST_PIPED_STDIN";

/// Verifies that [`is_input_interactive()`] returns [`IsNotInteractive`] when
/// stdin is `/dev/null` (not a [`TTY`]).
///
/// [`is_input_interactive()`]: crate::is_input_interactive
/// [`IsNotInteractive`]: TTYResult::IsNotInteractive
/// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
#[test]
fn test_piped_stdin_is_not_interactive() {
    if std::env::var(ENV_VAR).is_ok() {
        // Controlled path: stdin is /dev/null, report the result and exit.
        let result = is_input_interactive();
        println!("{result:?}");
        std::io::stdout().flush().ok();
        std::process::exit(0);
    }

    // Controller path: spawn self with piped streams.
    let output = super::test_fixtures::spawn_self_with_piped_streams(
        "test_piped_stdin_is_not_interactive",
        ENV_VAR,
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("  child stdout: {stdout}");
    eprintln!("  child stderr: {stderr}");

    assert!(
        output.status.success(),
        "Child process failed with status: {:?}",
        output.status
    );
    assert!(
        stdout.contains("IsNotInteractive"),
        "Expected IsNotInteractive in stdout, got: {stdout}"
    );
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Spawns the current test binary as a child process with piped streams (no [`PTY`]).
///
/// - [`stdin`] is connected to `/dev/null` (or `NUL` on Windows), so
///   [`is_input_interactive()`] sees a non-TTY.
/// - [`stdout`] is piped back to the parent, so [`is_output_interactive()`] sees a
///   non-TTY, and the parent can read the child's output.
/// - [`stderr`] is piped for diagnostics.
///
/// The child re-invokes itself with `env_var=1` set, and the test function uses
/// that env var to route into the controlled code path (same pattern as
/// [`generate_pty_test!`]).
///
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`is_input_interactive()`]: crate::is_input_interactive
/// [`is_output_interactive()`]: crate::is_output_interactive
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stderr`]: std::process::Command::stderr
/// [`stdin`]: std::process::Command::stdin
/// [`stdout`]: std::process::Command::stdout
pub fn spawn_self_with_piped_streams(
    test_name: &str,
    env_var: &str,
) -> std::process::Output {
    let test_binary = std::env::current_exe().expect("Failed to get current executable");
    std::process::Command::new(test_binary)
        .args(["--test-threads", "1", "--nocapture", test_name])
        .env(env_var, "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to spawn child process")
}

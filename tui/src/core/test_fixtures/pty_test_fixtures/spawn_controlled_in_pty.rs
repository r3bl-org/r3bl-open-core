// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::PtyPair;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

/// Creates a PTY pair and spawns the current test binary as a controlled process.
///
/// Use this function for **multi-backend comparison tests** that need to run the same
/// controlled code with different backends and compare results. For **single-feature
/// PTY tests** that test one specific behavior, use [`generate_pty_test!`] instead.
///
/// # When to Use This Function vs [`generate_pty_test!`]
///
/// | Scenario                                         | Use                         |
/// | ------------------------------------------------ | --------------------------- |
/// | Testing a single feature in a PTY environment    | [`generate_pty_test!`]      |
/// | Comparing two backends produce identical results | [`spawn_controlled_in_pty`] |
/// | One test function, one controlled process        | [`generate_pty_test!`]      |
/// | One test function, multiple controlled processes | [`spawn_controlled_in_pty`] |
///
/// # Use Case
///
/// Backend compatibility tests need to:
/// 1. Run a test with backend A, capture results
/// 2. Run the same test with backend B, capture results
/// 3. Compare the results
///
/// This helper handles steps 1 and 2's PTY setup, returning the `PtyPair` so you can
/// call it multiple times (once per backend) within a single test function.
///
/// # Relationship to [`generate_pty_test!`]
///
/// | Aspect   | [`generate_pty_test!`]      | [`spawn_controlled_in_pty`]    |
/// | -------- | --------------------------- | ------------------------------ |
/// | Purpose  | Single-feature PTY tests    | Multi-backend comparison tests |
/// | Creates  | Full test function          | Just PTY + spawns process      |
/// | Returns  | Nothing (test passes/fails) | `PtyPair` for controller use   |
///
/// # Arguments
///
/// * `backend` - Backend identifier (e.g., `"direct_to_ansi"`, `"crossterm"`), set as the
///   env var value
/// * `env_var` - Environment variable name used to signal controlled mode and backend
///   selection
/// * `test_name` - Name of the test function to invoke in the subprocess
/// * `rows` - PTY height in rows
/// * `cols` - PTY width in columns
///
/// # Returns
///
/// A tuple containing `backend` name and the owned [`PtyPair`] so that the caller can
/// chain this in a pipeline (and reuse both).
///
/// # Panics
///
/// Panics if PTY creation or process spawning fails.
///
/// # Note on Child Process
///
/// The spawned child process handle is intentionally not returned. The child runs
/// independently and communicates via PTY I/O. Controllers should use timeouts and
/// completion signals rather than waiting on the child handle, as PTY `EOF` behavior
/// varies by platform.
///
/// For complete implementations using this helper, see:
/// - [`backend_compat_output_test`] - Output backend comparison using **snapshot
///   testing** (captures rendered terminal state via [`OffscreenBuffer`]).
/// - [`backend_compat_input_test`] - Input backend comparison (compares parsed
///   [`InputEvent`]s).
///
/// [`backend_compat_input_test`]: mod@crate::core::terminal_io::backend_compat_tests::backend_compat_input_test
/// [`backend_compat_output_test`]: mod@crate::core::terminal_io::backend_compat_tests::backend_compat_output_test
/// [`InputEvent`]: crate::InputEvent
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
/// [`generate_pty_test!`]: crate::generate_pty_test
#[must_use]
pub fn spawn_controlled_in_pty<'a>(
    backend: &'a str,
    env_var: &'a str,
    test_name: &'a str,
    rows: u16,
    cols: u16,
) -> (
    /* backend */ &'a str,
    /* return ownership of */ PtyPair,
) {
    // Create PTY pair.
    let pty_system = NativePtySystem::default();
    let raw_pty_pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY pair");

    let pty_pair = PtyPair::from(raw_pty_pair);

    // Spawn controlled process.
    let test_binary = std::env::current_exe().expect("Failed to get current executable");
    let mut cmd = CommandBuilder::new(&test_binary);
    cmd.env(env_var, backend);
    cmd.args(["--test-threads", "1", "--nocapture", test_name]);

    let _child = pty_pair
        .controlled()
        .spawn_command(cmd)
        .expect("Failed to spawn controlled process");

    (backend, pty_pair)
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Environment variable used by [`generate_pty_test!`] to route execution between
/// controller and controlled processes. When set, the test binary runs as the
/// controlled (child) process.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_TEST_CONTROLLED";

/// Signal confirming the test binary routed to the controlled process.
///
/// Printed by the [`generate_pty_test!`] macro before any user code runs. The
/// controller asserts on this to confirm the cargo test runner actually executed
/// the controlled code path.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const TEST_RUNNING: &str = "TEST_RUNNING";

/// Signal indicating the controlled process has started and is initializing.
///
/// Printed by the [`generate_pty_test!`] macro after detecting the environment
/// variable, before enabling raw mode or calling the controlled function.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const CONTROLLED_STARTING: &str = "CONTROLLED_STARTING";

/// Signal indicating the controlled process is ready to receive input.
///
/// Unlike [`TEST_RUNNING`] and [`CONTROLLED_STARTING`] (which the macro prints
/// automatically), this marker is printed by each controlled function after its
/// input device or runtime is fully initialized.
pub const CONTROLLED_READY: &str = "CONTROLLED_READY";

/// Default prefix for line state output in [`readline_async`] integration tests.
///
/// [`readline_async`]: crate::readline_async
pub const LINE_PREFIX: &str = "Line:";

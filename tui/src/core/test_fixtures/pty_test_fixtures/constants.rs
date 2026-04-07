// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Environment variable used by [`generate_pty_test!`] to route execution between
/// controller and controlled processes. When set, the test binary runs as the
/// controlled (child) process.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const ENV_VAR_PTY_CONTROLLED: &str = "R3BL_PTY_TEST_CONTROLLED";

/// Signal confirming the test binary routed to the controlled process.
///
/// Printed by the [`generate_pty_test!`] macro before any user code runs. The
/// controller asserts on this to confirm the cargo test runner actually executed
/// the controlled code path.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const MSG_TEST_RUNNING: &str = "TEST_RUNNING";

/// Signal indicating the controlled process has started and is initializing.
///
/// Printed by the [`generate_pty_test!`] macro after detecting the environment
/// variable, before enabling raw mode or calling the controlled function.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
pub const MSG_CONTROLLED_STARTING: &str = "CONTROLLED_STARTING";

/// Signal indicating the controlled process is ready to receive input.
///
/// Unlike [`MSG_TEST_RUNNING`] and [`MSG_CONTROLLED_STARTING`] (which the macro prints
/// automatically), this marker is printed by each controlled function after its
/// input device or runtime is fully initialized.
pub const MSG_CONTROLLED_READY: &str = "CONTROLLED_READY";

/// Signal used to report received input back to the controller.
pub const MSG_RECEIVED: &str = "RECEIVED:";

/// Signal used to report failure back to the controller.
pub const MSG_FAILED: &str = "FAILED:";

/// Signal used to report success back to the controller.
pub const MSG_SUCCESS: &str = "SUCCESS:";

/// Default prefix for line state output in [`readline_async`] integration tests.
///
/// [`readline_async`]: crate::readline_async
pub const MSG_LINE_PREFIX: &str = "Line:";

/// Success glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_SUCCESS: &str = "✅";

/// Failure glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_FAILURE: &str = "❌";

/// Waiting or progress glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_WAITING: &str = "📝";

/// Warning glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_WARNING: &str = "⚠️";

/// Controller process glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_CONTROLLER: &str = "🚀";

/// Controlled process glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_CONTROLLED: &str = "🔍";

/// Step marker for numbered sequences in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_STEP: &str = "📍";

/// Final "all assertions passed" marker in [`PTY`] tests.
///
/// This is distinct from [`GLYPH_SUCCESS`] (which is used for sub-step success).
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_COMPLETION: &str = "🎉";

/// Cleanup-phase glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_CONTROLLER_CLEANUP: &str = "🧹";

/// Skipping-phase glyph used in [`PTY`] tests.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const GLYPH_SKIPPING: &str = "⏭️";

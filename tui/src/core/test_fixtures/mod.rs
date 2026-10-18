// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Reusable test fixtures, mocks, and process/[`PTY`] isolation harnesses for `r3bl_tui`
//! and downstream crates.
//!
//! # Why This Module Is Not Gated Behind `#[cfg(test)]`
//!
//! Even though these utilities exist to support testing, this module is compiled
//! unconditionally as part of the normal `r3bl_tui` library target for three reasons:
//!
//! 1. **Static-Dispatch Mock Terminal Backend:** [`InputDevice::Mock`] embeds
//!    [`MockInputDevice`] directly as an enum variant on the production [`InputDevice`]
//!    type so terminal I/O uses zero-cost enum dispatch rather than `dyn` trait objects.
//! 2. **Rust Doctests:** Cargo compiles doctests as separate binaries linked against the
//!    normal (`not(test)`) library build. Doctests across `r3bl_tui` rely on
//!    [`StdoutMock`], [`MockInputDevice`], [`CaughtPanicResult`], and
//!    [`retry_until_success_test!`].
//! 3. **Downstream Crate & Consumer Testing:** When downstream workspace crates (such as
//!    `r3bl-cmdr`) or external `r3bl_tui` applications run `cargo test`, Cargo compiles
//!    `r3bl_tui` with `not(test)`. Exporting these fixtures publicly allows downstream
//!    crates to test their TUI and CLI flows with [`StdoutMock`], [`PtyTestContext`], and
//!    [`generate_isolated_process_test!`].
//!
//! [`CaughtPanicResult`]: crate::CaughtPanicResult
//! [`generate_isolated_process_test!`]: crate::generate_isolated_process_test
//! [`InputDevice::Mock`]: crate::InputDevice::Mock
//! [`InputDevice`]: crate::InputDevice
//! [`MockInputDevice`]: crate::MockInputDevice
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtyTestContext`]: crate::PtyTestContext
//! [`retry_until_success_test!`]: crate::retry_until_success_test
//! [`StdoutMock`]: crate::StdoutMock

// Attach sources.
pub mod assertion_fixtures;
pub mod input_device_fixtures;
pub mod isolated_process_fixtures;
pub mod output_device_fixtures;
pub mod pty_test_fixtures;
pub mod retry_fixtures;
pub mod tcp_stream_fixtures;

// Re-export.
pub use assertion_fixtures::*;
pub use input_device_fixtures::*;
pub use isolated_process_fixtures::*;
pub use output_device_fixtures::*;
pub use pty_test_fixtures::*;
pub use retry_fixtures::*;
pub use tcp_stream_fixtures::*;

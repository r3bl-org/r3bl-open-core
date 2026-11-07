// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Subsystem end-to-end integration tests for [`UpgradeJourney`].
//!
//! # Test Approach & Rationale
//!
//! Unit tests in sibling modules verify individual state transitions in isolation (in the
//! [Control Plane]).
//!
//! This module exercises the full [`UpgradeJourney::start`] ->
//! [`StartOutcome::handle_upgrade_on_exit`][2] lifecycle end-to-end, spanning both the
//! [Control Plane] and [Execution Plane] across three test files:
//!
//! 1. **[`primary_tests`]**: In-process tests (`#[serial]`) covering the single-start guard
//!    ([`StartOutcome::AlreadyStarted`]), immediate and deferred state transitions, `1h`
//!    cooldown timestamp file creation/[`mtime`] refresh/preservation on failure, and
//!    [`UpgradePolicy::Disabled`].
//! 2. **[`self_replace_test`]**: Isolated subprocess test verifying
//!    [`UpgradeSource::ReleaseArchive`][4] end-to-end: streaming a `.tar.gz` archive and
//!    `.sha256` checksum from a local HTTP server, verifying the SHA-256 digest,
//!    replacing the copied executable in-place via [`self_replace`], and installing
//!    sibling binaries (`edi`, `env-source`).
//! 3. **[`cargo_install_rustup_test`]**: Isolated subprocess test verifying
//!    [`UpgradeSource::BuildFromSourceRustup`][5] end-to-end: falling back when the
//!    release archive server is unreachable and driving stub `rustup` and `cargo`
//!    binaries (emitting [`OscSequence`] `9;4` progress events) through
//!    [`PtySessionBuilder`].
//!
//! Shared canned JSONC manifest responses live in `http_responses/*.jsonc` and are loaded
//! via [`include_str!`] in [`test_fixtures`].
//!
//! [2]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
//! [4]: crate::self_upgrade::UpgradeSource::ReleaseArchive
//! [5]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
//! [`cargo_install_rustup_test`]: mod@cargo_install_rustup_test
//! [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
//! [`OscSequence`]: r3bl_tui::osc_codes::OscSequence
//! [`primary_tests`]: mod@primary_tests
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`self_replace_test`]: mod@self_replace_test
//! [`self_replace`]: self_replace::self_replace
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`test_fixtures`]: mod@test_fixtures
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane

#![rustfmt::skip]
#![allow(clippy::unwrap_used)]

use super::*;

pub mod test_fixtures;
pub mod primary_tests;
pub mod self_replace_test;
pub mod cargo_install_rustup_test;

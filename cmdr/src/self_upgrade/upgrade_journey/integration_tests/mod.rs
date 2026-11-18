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
//! [Control Plane] and [Execution Plane] across six test files:
//!
//! 1. **`start_guard_and_disabled_tests`**: In-process tests (`#[serial]`) covering the
//!    single-start guard ([`StartOutcome::AlreadyStarted`]) and
//!    [`UpgradePolicy::Disabled`].
//! 2. **`immediate_upgrade_tests`**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Immediate`] sync/async [`UpToDate`], `1h` cooldown bypass and
//!    [`mtime`] refresh, [`CheckFailed`], and [`UpgradeResolution::UpgradeFailed`][6].
//! 3. **`deferred_upgrade_tests`**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Deferred`] `1h` cooldown skip/expiry and in-flight dedicated
//!    thread join on exit.
//! 4. **`deferred_unhappy_path_tests`**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Deferred`] HTTP client timeout, in-flight delayed HTTP 500
//!    failure, manifest network errors ([`ExitOutcome::CheckFailed`]), dedicated thread
//!    panic recovery ([`ExitOutcome::NoDeferredCheckRunning`][1]), and
//!    [`UpgradeActionOnExit::Auto`] binary download failure.
//! 5. **`self_replace_test`**: Isolated subprocess test verifying
//!    [`UpgradeSource::ReleaseArchive`][4] end-to-end: streaming a `.tar.gz` archive and
//!    `.sha256` checksum from a local HTTP server, verifying the SHA-256 digest,
//!    replacing the copied executable in-place via [`self_replace`], and installing
//!    sibling binaries (`edi`, `env-source`).
//! 6. **`cargo_install_rustup_test`**: Isolated subprocess test verifying
//!    [`UpgradeSource::BuildFromSourceRustup`][5] end-to-end: falling back when the
//!    release archive server is unreachable and driving stub `rustup` and `cargo`
//!    binaries (emitting [`OscSequence`] `9;4` progress events) through
//!    [`PtySessionBuilder`].
//!
//! Shared canned JSONC manifest responses are generated via
//! [`test_fixtures::jsonc_golden`].
//!
//! [1]: crate::self_upgrade::ExitOutcome::NoDeferredCheckRunning
//! [2]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
//! [4]: crate::self_upgrade::UpgradeSource::ReleaseArchive
//! [5]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
//! [6]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`ExitOutcome::CheckFailed`]: crate::self_upgrade::ExitOutcome::CheckFailed
//! [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
//! [`OscSequence`]: r3bl_tui::OscSequence
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`self_replace`]: self_replace::self_replace
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`test_fixtures`]: mod@test_fixtures
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [`UpToDate`]: crate::self_upgrade::UpToDate
//! [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane

#![rustfmt::skip]

pub mod test_fixtures;

#[cfg(test)]
pub mod start_guard_and_disabled_tests;
#[cfg(test)]
pub mod immediate_upgrade_tests;
#[cfg(test)]
pub mod deferred_upgrade_tests;
#[cfg(test)]
pub mod deferred_unhappy_path_tests;
#[cfg(test)]
pub mod self_replace_test;
#[cfg(test)]
pub mod cargo_install_rustup_test;

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
//! 1. **[`start_guard_and_disabled_tests`]**: In-process tests (`#[serial]`) covering the
//!    single-start guard ([`StartOutcome::AlreadyStarted`]) and
//!    [`UpgradePolicy::Disabled`].
//! 2. **[`immediate_upgrade_tests`]**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Immediate`] sync/async [`UpToDate`], `1h` cooldown bypass and
//!    [`mtime`] refresh, [`CheckFailed`], and [`UpgradeResolution::UpgradeFailed`].
//! 3. **[`deferred_upgrade_tests`]**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Deferred`] `1h` cooldown skip/expiry and in-flight dedicated
//!    thread join on exit.
//! 4. **[`deferred_unhappy_path_tests`]**: In-process tests (`#[serial]`) covering
//!    [`UpgradeStrategy::Deferred`] HTTP client timeout, in-flight delayed HTTP 500
//!    failure, manifest network errors ([`ExitOutcome::CheckFailed`]), dedicated thread
//!    panic recovery ([`ExitOutcome::NoDeferredCheckRunning`]), and
//!    [`UpgradeActionOnExit::Auto`] binary download failure.
//! 5. **[`self_replace_test`]**: Isolated subprocess test verifying
//!    [`UpgradeSource::ReleaseArchive`][4] end-to-end: streaming a `.tar.gz` archive and
//!    `.sha256` checksum from a local HTTP server, verifying the SHA-256 digest,
//!    replacing the copied executable in-place via [`self_replace`], and installing
//!    sibling binaries (`edi`, `env-source`).
//! 6. **[`cargo_install_rustup_test`]**: Isolated subprocess test verifying
//!    [`UpgradeSource::BuildFromSourceRustup`][5] end-to-end: falling back when the
//!    release archive server is unreachable and driving stub `rustup` and `cargo`
//!    binaries (emitting [`OscSequence`] `9;4` progress events) through
//!    [`PtySessionBuilder`].
//!
//! Shared canned JSONC manifest responses are generated via
//! [`test_fixtures::jsonc_golden`].
//!
//! [2]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
//! [4]: crate::self_upgrade::UpgradeSource::ReleaseArchive
//! [5]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
//! [`cargo_install_rustup_test`]: mod@cargo_install_rustup_test
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`deferred_unhappy_path_tests`]: mod@deferred_unhappy_path_tests
//! [`deferred_upgrade_tests`]: mod@deferred_upgrade_tests
//! [`ExitOutcome::CheckFailed`]: crate::self_upgrade::ExitOutcome::CheckFailed
//! [`ExitOutcome::NoDeferredCheckRunning`]: crate::self_upgrade::ExitOutcome::NoDeferredCheckRunning
//! [`immediate_upgrade_tests`]: mod@immediate_upgrade_tests
//! [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
//! [`OscSequence`]: r3bl_tui::osc_codes::OscSequence
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`self_replace_test`]: mod@self_replace_test
//! [`self_replace`]: self_replace::self_replace
//! [`start_guard_and_disabled_tests`]: mod@start_guard_and_disabled_tests
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`test_fixtures`]: mod@test_fixtures
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [`UpToDate`]: crate::self_upgrade::UpToDate
//! [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane

#![rustfmt::skip]

pub mod test_fixtures;
pub mod start_guard_and_disabled_tests;
pub mod immediate_upgrade_tests;
pub mod deferred_upgrade_tests;
pub mod deferred_unhappy_path_tests;
pub mod self_replace_test;
pub mod cargo_install_rustup_test;

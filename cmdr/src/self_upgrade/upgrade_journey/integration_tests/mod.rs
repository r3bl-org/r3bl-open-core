// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! End-to-end integration tests for [`UpgradeJourney`].
//!
//! # Why Integration Tests Exist Alongside Sibling Unit Tests
//!
//! Unit tests in sibling modules (`check/`, `upgrade/`, and `typestates/`) verify
//! individual [Execution Plane] functions and individual [Control Plane] state
//! transitions in isolation.
//!
//! This module exercises the full [`UpgradeJourney::start`] ->
//! [`DeferredUpgradeOutcome::handle_upgrade_on_exit`][on-exit] lifecycle end-to-end,
//! spanning both the [Control Plane] and [Execution Plane] (including
//! [`PerProcessStartLatch`][latch], [`RemoteVersionCheckGate`], [`mockito`] HTTP servers,
//! `self_replace`, and [`PTY`] subprocess orchestration). Shared HTTP and manifest
//! fixtures live in [`test_fixtures`], and [`PerProcessStartLatch`][latch] is imported
//! directly by test files rather than barrel-exported from `upgrade_journey`.
//!
//! # Test Organization: 5 Use-Case Groups (9 Files)
//!
//! 1. **Single-Start Latch & Disabled Policy (in-process `#[serial]`)**
//!    - **`start_latch_and_disabled`**: Covers [`UpgradePolicy::Disabled`] (`0` HTTP
//!      calls) and monotonic [`PerProcessStartLatch`][latch] rejection of duplicate calls
//!      after `Disabled`, `Immediate`, or in-flight `Deferred` starts without handle
//!      stealing.
//! 2. **Immediate Upgrade (`--upgrade`, in-process `#[serial]`)**
//!    - **`immediate_upgrade_happy_paths`**: Covers `remote == local` (`0.0.25`)
//!      [`UpToDate`], `local > remote` (`0.0.26 > 0.0.25`, no downgrade), and `--upgrade`
//!      bypassing an in-cooldown `1h` window and refreshing [`mtime`].
//!    - **`immediate_upgrade_unhappy_paths`**: Covers manifest HTTP 500, connection
//!      refused, crate missing from manifest ([`CheckFailed`]), and
//!      [`CrateDistribution::ClosedSource`][cs] unreachable archive
//!      ([`UpgradeResolution::UpgradeFailed`][failed]), all leaving the gate
//!      `Open`.
//! 3. **Deferred Upgrade on Exit (`--upgrade-behavior`, in-process `#[serial]` + non-TTY
//!    subprocess)**
//!    - **`deferred_upgrade_happy_paths`**: Covers in-cooldown skip (`0` HTTP calls ->
//!      [`ExitOutcome::Skipped`]), not-in-cooldown check -> [`UpToDate`], in-flight check
//!      joined at exit with [`UpgradeActionOnExit::NotifyOnly`][notify], and
//!      [`UpgradeActionOnExit::PromptToUpgrade`][prompt] in an isolated non-TTY
//!      Coordinator / Worker subprocess (`Stdio::null()` ->
//!      [`UpgradeResolution::SkippedNonInteractive`][non-tty]).
//!    - **`deferred_upgrade_unhappy_paths`**: Covers in-flight delayed HTTP 500 failure
//!      joined at exit, connection refused, [`CrateDistribution::ClosedSource`][cs]
//!      unreachable archive, and dedicated check thread panic recovery
//!      ([`ThreadRunnerError::ThreadPanicked`][panicked]) without unwinding the
//!      main thread.
//! 4. **Binary Self-Replacement (isolated Coordinator / Worker subprocess on copied
//!    executable)**
//!    - **`self_replace_happy_paths`**: Verifies [`UpgradeSource::ReleaseArchive`] for
//!      both `Immediate` (`--upgrade`) and `Deferred(Auto)` (exit): 4-route `.tar.gz` and
//!      `.sha256` download, digest verification, in-place [`self_replace`] of the copied
//!      executable, sibling binary (`edi`, `env-source`) installation, and cooldown
//!      activation.
//!    - **`self_replace_unhappy_paths`**: Verifies
//!      [`CrateDistribution::ClosedSource`][cs] failure modes leave the copied executable
//!      untouched on disk and cooldown untouched: SHA-256 checksum mismatch, `GET
//!      *.tar.gz` HTTP 500 after `HEAD 200`, empty `.tar.gz` archive, and corrupt
//!      non-gzip archive bytes.
//! 5. **Open-Source Fallback via `rustup` & `cargo` (isolated Coordinator / Worker
//!    subprocess with stub binaries on `PATH` in [`PTY`])**
//!    - **`install_via_rustup_happy_paths`**: Verifies
//!      [`UpgradeSource::BuildFromSourceRustup`][rustup] via stub `rustup` and `cargo`
//!      emitting [`OSC`] `9;4` progress events inside [`PtySessionBuilder`] for
//!      `Immediate` `HEAD` unreachable, `Deferred(Auto)` `HEAD` unreachable, and
//!      mid-stream `GET *.tar.gz` HTTP 500 recovery (`try_fallback_to_source`).
//!    - **`install_via_rustup_unhappy_paths`**: Verifies empty `PATH`
//!      (`NoLocalToolchain`), stub `rustup` non-zero exit short-circuiting before
//!      `cargo`, and stub `cargo` emitting `OSC 9;4;2;0` (`BuildError`) + non-zero exit
//!      without activating cooldown.
//!
//! # Sync + Async `scenario_<name>()` Wrapper Policy
//!
//! Binaries that use `self_upgrade` mix synchronous (`fn main()`) and Tokio
//! (`#[tokio::main]`) entry points.
//!
//! Every in-process test scenario in Groups 1–3 (plus Group 3's non-TTY prompt
//! Coordinator) puts its test body in a single shared `scenario_<name>()` helper wrapped
//! by both:
//! - `#[test]` (`test_<name>_sync`), and
//! - `#[tokio::test]` (`test_<name>_async`).
//!
//! Running each scenario under both harnesses verifies that [`UpgradeJourney::start`] and
//! [`StdThreadTokioRunner`] never panic on nested Tokio runtimes when invoked from an
//! existing async runtime.
//!
//! # Cooldown Interaction Flow
//!
//! ```text
//! UpgradeJourney::start()
//! │
//! ├── Disabled
//! │     └─► Cooldown not consulted ──► Never touched (stays Open)
//! │
//! ├── Immediate (--upgrade)
//! │     └─► Cooldown bypassed unconditionally ──► Runs check + upgrade
//! │           ├── UpToDate / Upgraded ──► touch() ──► ClosedForCooldown
//! │           └── Failures            ──► untouched (keeps prior state)
//! │
//! └── Deferred (--upgrade-behavior)
//!       ├── Pre-check: ClosedForCooldown
//!       │     └─► Skips check (0 HTTP) ──► not re-touched (stays ClosedForCooldown)
//!       │
//!       └── Pre-check: Open
//!             └─► Spawns check thread
//!                   ├── UpToDate / Upgraded ──► touch() ──► ClosedForCooldown
//!                   └── NotifyOnly / NonTTY / DeclinedByUser / Failures ──► untouched
//! ```
//!
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`ExitOutcome::Skipped`]: crate::self_upgrade::ExitOutcome::Skipped
//! [`mockito`]: https://docs.rs/mockito
//! [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
//! [`OSC`]: r3bl_tui::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`RemoteVersionCheckGate`]: crate::self_upgrade::RemoteVersionCheckGate
//! [`self_replace`]: self_replace::self_replace
//! [`StdThreadTokioRunner`]: r3bl_tui::StdThreadTokioRunner
//! [`test_fixtures`]: mod@test_fixtures
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [`UpgradeSource::ReleaseArchive`]: crate::self_upgrade::UpgradeSource::ReleaseArchive
//! [`UpToDate`]: crate::self_upgrade::UpToDate
//! [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [cs]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [failed]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [latch]: super::per_process_start_latch::PerProcessStartLatch
//! [non-tty]: crate::self_upgrade::UpgradeResolution::SkippedNonInteractive
//! [notify]: crate::self_upgrade::UpgradeActionOnExit::NotifyOnly
//! [on-exit]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
//! [panicked]: crate::self_upgrade::ThreadRunnerError::ThreadPanicked
//! [prompt]: crate::self_upgrade::UpgradeActionOnExit::PromptToUpgrade
//! [rustup]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup

#![rustfmt::skip]

pub mod test_fixtures;

#[cfg(test)]
pub mod start_latch_and_disabled;
#[cfg(test)]
pub mod immediate_upgrade_happy_paths;
#[cfg(test)]
pub mod immediate_upgrade_unhappy_paths;
#[cfg(test)]
pub mod deferred_upgrade_happy_paths;
#[cfg(test)]
pub mod deferred_upgrade_unhappy_paths;
#[cfg(test)]
pub mod self_replace_happy_paths;
#[cfg(test)]
pub mod self_replace_unhappy_paths;
#[cfg(test)]
pub mod install_via_rustup_happy_paths;
#[cfg(test)]
pub mod install_via_rustup_unhappy_paths;

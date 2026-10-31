// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Update checking and in-place upgrades for R3BL binary products.
//!
//! This module provides the client-side upgrade engine and shared manifest schema for
//! open-source and closed-source products (binaries for Linux, macOS, and Windows),
//! downloading new releases and replacing the currently running binary.
//!
//! # Scope and Architectural Boundary
//!
//! This module is the client-side engine and not the release infrastructure or pipeline.
//! It implements the reusable client engine and shared manifest library, while the actual
//! hosting, packaging, and publishing pipeline lives elsewhere:
//!
//! ## Included in this module: The Engine
//!
//! 1. Read and Upgrade Path: Embedded into product binaries via [`upgrade_journey!`] to
//!    fetch [`R3BL_VERSIONS_JSON_URL`], check if a newer release exists, and upgrade the
//!    running binary in-place from [`R3BL_BINARIES_RELEASE_BASE_URL`] (or fall back to
//!    `crates.io`).
//! 2. Write Path: Exposes [`JsoncManifest::try_update_crate_in_jsonc`][update-jsonc] so
//!    external release tooling can update [`R3BL_VERSIONS_JSON_URL`] entries in-place
//!    while preserving existing comments and formatting.
//!
//! ## External to this repository: The Release and Distribution Pipeline
//!
//! - Cross-platform binary compilation, archive packaging, SHA-256 checksum generation,
//!   and publishing to [`R3BL_VERSIONS_JSON_URL`] and [`R3BL_BINARIES_RELEASE_BASE_URL`]
//!   are performed by external release tooling that depends on this (`r3bl-cmdr`) library
//!   crate.
//! - This is why this workspace intentionally contains no static
//!   [`R3BL_VERSIONS_JSON_URL`] file or release-publishing scripts.
//!
//! ## Shared Distribution Contract
//!
//! - Because both open-source crates in this workspace and external R3BL binary crates
//!   depend on `r3bl-cmdr` for self-upgrading, [`CrateDistribution`] supports both
//!   distribution models through a single unified schema:
//!   - [`CrateDistribution::OpenSource`]: Falls back to building the exact version pinned
//!     in [`R3BL_VERSIONS_JSON_URL`] from `crates.io` via [`PTY`] + `OSC 9;4` progress
//!     tracking when pre-built release archives are unreachable.
//!   - [`CrateDistribution::ClosedSource`]: Upgrades exclusively from pre-built GitHub
//!     release archives (with no `crates.io` fallback).
//!
//! # Centralized Version Manifest ([`R3BL_VERSIONS_JSON_URL`])
//!
//! The version manifest ([`JsoncManifest`]) is the central registry for all R3BL binary
//! crates. These include `r3bl-cmdr` (`giti`, `edi`, `env-source`), `r3bl-build-infra`,
//! and `r3bl-rust-analyzer-mcp-server`.
//!
//! Instead of making unauthenticated requests against multiple individual GitHub release
//! endpoints or `crates.io` APIs, all R3BL binaries query a single lightweight document
//! ([`R3BL_VERSIONS_JSON_URL`]) to discover current release states, distribution
//! channels, and upgrade strategies.
//!
//! # Main Entry Point: [`UpgradeJourney`] & Typestate Lifecycle
//!
//! To make illegal states unrepresentable and unify startup, dedicated-thread checks, and
//! shutdown under a single compiler-verified pipeline, the self-upgrade engine is built
//! around three pillars:
//!
//! 1. CLI-to-Domain Boundary ([`UpgradeCLIArgs`] -> [`UpgradePolicy`] ->
//!    [`UpgradeStrategy`] -> [`UpgradeActionOnExit`]):
//!    - [`UpgradeCLIArgs`] is an opaque [`clap`] adapter at the CLI edge whose private
//!      fields prevent invalid flag combinations from being constructed in Rust code.
//!      Binaries flatten [`UpgradeCLIArgs`] (mutually exclusive [`--upgrade`][u] or
//!      [`--upgrade-behavior`][ub]) into their argument parser and pass it to
//!      [`upgrade_journey!`], which converts it at the boundary via
//!      [`UpgradePolicy::from()`][policy-from] into the pure domain model:
//!      - [`--upgrade`][u] -> [`UpgradePolicy::Enabled`] wrapping
//!        [`UpgradeStrategy::Immediate`].
//!      - [`--upgrade-behavior`][ub] `auto` (default when omitted), `prompt-to-upgrade`,
//!        or `notify-only` -> [`UpgradePolicy::Enabled`] wrapping
//!        [`UpgradeStrategy::DedicatedThread`] (carrying the corresponding
//!        [`UpgradeActionOnExit`]).
//!      - [`--upgrade-behavior`][ub] `disable` -> [`UpgradePolicy::Disabled`] (where no
//!        [`UpgradeStrategy`] exists).
//! 2. [Typestate Pattern] ([`UpgradeJourney<State>`]):
//!    - [`UpgradeJourney<State>`] encodes the 9-state upgrade machine ([`NotStarted`],
//!      [`Disabled`], [`CooldownSkipped`], [`Checking`], [`UpToDate`],
//!      [`UpgradeAvailable`], [`CheckFailed`], [`Upgrading`], and [`Completed`]) in the
//!      type system by holding immutable [`UpgradeConfig`] alongside `State:
//!      `[`UpgradeJourneyStateMarker`].
//!    - Binaries construct [`UpgradeJourney<NotStarted>`] via [`upgrade_journey!`] and
//!      call [`.start()`], which atomically claims the process-wide singleton (returning
//!      [`StartupOutcome::AlreadyStarted`] on duplicate calls), or returns
//!      [`StartupOutcome::UpgradedAndExit`] (carrying [`ImmediateUpgradeResult`]) vs.
//!      [`StartupOutcome::ContinueApplication`].
//! 3. Coordinated Process-Wide Singleton ([`UpgradeCheckStatus`]):
//!    - [`UpgradeCheckStatus`] pairs the active [`UpgradeJourney<State>`] with the
//!      optional [`JoinHandle<()>`][jh] of the [`CHECK_THREAD_NAME`] OS thread inside a
//!      leaf [`ScopedMutex`].
//!    - At shutdown, [`UpgradeCheckStatus::handle_upgrade_on_exit`][9] takes and joins
//!      any in-flight check thread after releasing the mutex (preventing deadlock and
//!      ensuring short-lived invocations wait up to
//!      [`DEDICATED_THREAD_MANIFEST_TIMEOUT`][2] when the
//!      [`DEDICATED_THREAD_CHECK_COOLDOWN`][1] timestamp file was expired), then
//!      atomically transitions [`UpgradeAvailable`] -> [`Upgrading`] -> [`Completed`] for
//!      one-shot exit handling.
//!
//! ```text
//! upgrade_journey!(channel, cli_args).start()
//! │
//! ├─ Duplicate .start() ───────────────────────────► AlreadyStarted (no-op)
//! │
//! ├─ Disabled (--upgrade-behavior disable)
//! │  └─ Disabled ──────────────────────────────────► ContinueApplication (no-op at exit)
//! │
//! ├─ Enabled(Immediate) (--upgrade)
//! │  └─ Checking (5s timeout, bypasses 1h cooldown)
//! │     ├─ UpToDate ──► touch timestamp file ──────► UpgradedAndExit(Succeeded)
//! │     ├─ CheckFailed ────────────────────────────► UpgradedAndExit(Failed)
//! │     └─ Upgrading ──► perform_upgrade()
//! │        ├─ Completed(Upgraded) ──► touch file ──► UpgradedAndExit(Succeeded)
//! │        └─ Completed(UpgradeFailed) ────────────► UpgradedAndExit(Failed)
//! │
//! └─ Enabled(DedicatedThread(action)) (default: Auto | PromptToUpgrade | NotifyOnly)
//!    ├─ Active 1h cooldown ──► CooldownSkipped ────► ContinueApplication (no-op at exit)
//!    └─ Expired/missing timestamp file ──► spawns check thread (2s timeout) ──► ContinueApplication
//!       │
//!       └─ At exit: UpgradeCheckStatus::handle_upgrade_on_exit()
//!          1. Joins check thread outside lock (waits <= 2s if still Checking)
//!          2. UpToDate | CheckFailed ──────────────► silent no-op
//!          3. UpgradeAvailable ──► Upgrading ──► runs Auto / PromptToUpgrade / NotifyOnly
//!                                               └──► Completed (touches timestamp file if Upgraded)
//! ```
//!
//! # Error Handling Policy
//!
//! - **Background check ([`UpgradeStrategy::DedicatedThread`], fails silently)**: If a
//!   background version check fails (for example, while offline or when the manifest URL
//!   cannot be reached), the running binary is never interrupted and nothing is printed
//!   to the terminal. The engine records [`UpgradeCheckStatus::CheckFailed`], logs the
//!   error when [`DEBUG_SELF_UPGRADE`] is enabled, and leaves the `1h` cooldown timestamp
//!   file untouched so the next run can try again.
//! - **Immediate upgrade ([`UpgradeStrategy::Immediate`], fails loudly)**: When the user
//!   explicitly passes `--upgrade`, any check or upgrade failure prints a clear message
//!   and retry hint on [`UserFeedbackChannel`] and returns
//!   [`StartupOutcome::UpgradedAndExit`] with [`ImmediateUpgradeResult::Failed`] so the
//!   binary exits with status code `1`.
//!
//! # Upgrade Source Resolution (GitHub Release vs. Source Fallback)
//!
//! Once the engine determines that an upgrade is available (`remote_version >
//! current_version`), it must decide *how* to perform the upgrade before downloading or
//! running build commands. This decision resolves into an [`UpgradeSource`].
//!
//! - **Who resolves it**: Evaluated by [`UpgradeSource::try_resolve`] (and its pure
//!   decision function [`pure_evaluator::evaluate_upgrade_source_pure`]). If a binary
//!   download fails mid-stream, [`UpgradeSource::try_fallback_to_source`][10] evaluates
//!   fallback viability.
//! - **Where it is called**: Invoked by [`perform_upgrade`] immediately before
//!   dispatching to download or compilation routines.
//! - **When it runs**: Right after manifest evaluation confirms that a newer version is
//!   available, and immediately before any bytes are fetched or child processes spawned.
//!
//! ## Resolution Decision Tree
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │ Remote version is newer (remote > current)             │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!                             │
//! ┌───────────────────────────▼────────────────────────────┐
//! │ HTTP HEAD probe to GitHub Release asset URL            │
//! └─────────────┬────────────────────────────┬─────────────┘
//!               │ Found (200)                │ Network Error / Not Found
//!               │                            │
//! ┌─────────────▼─────────────┐    ┌─────────▼─────────────┐
//! │ UpgradeSource::           │    │ CrateDistribution?    │
//! │ GithubReleaseArtifact     │    └───┬───────────────┬───┘
//! └───────────────────────────┘        │ Closed        │ Open
//!                                      │               │
//!                              ┌───────▼───────┐ ┌─────▼────────────┐
//!                              │ Err(NoViable  │ │ Check $PATH for  │
//!                              │ UpgradeSource)│ │ rustup           │
//!                              └───────────────┘ └───┬──────────┬───┘
//!                                        rustup found│          │ missing
//!                                                    │          │
//!                                       ┌────────────▼┐  ┌──────▼────────┐
//!                                       │UpgradeSource│  │ Err(NoViable  │
//!                                       │::BuildFrom  │  │ UpgradeSource)│
//!                                       │SourceRustup │  └───────────────┘
//!                                       └─────────────┘
//! ```
//!
//! [10]: crate::self_upgrade::UpgradeSource::try_fallback_to_source
//! [1]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
//! [2]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
//! [9]: crate::self_upgrade::UpgradeCheckStatus::handle_upgrade_on_exit
//! [`.start()`]: crate::self_upgrade::UpgradeJourney::start
//! [`CHECK_THREAD_NAME`]: crate::self_upgrade::check_thread::CHECK_THREAD_NAME
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`Checking`]: crate::self_upgrade::Checking
//! [`clap`]: mod@clap
//! [`Completed`]: crate::self_upgrade::Completed
//! [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
//! [`CrateDistribution::OpenSource`]: crate::self_upgrade::CrateDistribution::OpenSource
//! [`CrateDistribution`]: crate::self_upgrade::CrateDistribution
//! [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
//! [`Disabled`]: crate::self_upgrade::Disabled
//! [`ImmediateUpgradeResult::Failed`]: crate::self_upgrade::ImmediateUpgradeResult::Failed
//! [`ImmediateUpgradeResult`]: crate::self_upgrade::ImmediateUpgradeResult
//! [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
//! [`NotStarted`]: crate::self_upgrade::NotStarted
//! [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
//! [`ScopedMutex`]: r3bl_tui::ScopedMutex
//! [`upgrade_journey!`]: crate::upgrade_journey
//! [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
//! [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
//! [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
//! [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
//! [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
//! [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourney<State>`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourneyStateMarker`]: crate::self_upgrade::UpgradeJourneyStateMarker
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [`UpgradePolicy::Enabled`]: crate::self_upgrade::UpgradePolicy::Enabled
//! [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
//! [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve
//! [`UpgradeSource`]: crate::self_upgrade::UpgradeSource
//! [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
//! [`Upgrading`]: crate::self_upgrade::Upgrading
//! [`UpToDate`]: crate::self_upgrade::UpToDate
//! [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
//! [jh]: std::thread::JoinHandle
//! [policy-from]: crate::self_upgrade::UpgradePolicy::from
//! [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
//! [u]: crate::self_upgrade::upgrade_cli_args::UPGRADE_ARG_LONG
//! [ub]: crate::self_upgrade::upgrade_cli_args::UPGRADE_BEHAVIOR_ARG_LONG
//! [update-jsonc]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc

// Private submodules.
mod check;
mod config;
mod constants;
mod parser;
mod upgrade;
mod upgrade_check_cache;
mod upgrade_journey_public_api;

// Public barrel re-exports.
pub use check::*;
pub use config::*;
pub use constants::*;
pub use parser::*;
pub use upgrade::*;
pub use upgrade_check_cache::*;
pub use upgrade_journey_public_api::*;

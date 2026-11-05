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
//! 1. Read and Upgrade Path:
//!    - Embedded into product binaries via [`upgrade_journey!`] to fetch
//!      [`R3BL_VERSIONS_JSONC_URL`], check if a newer release exists, and upgrade the
//!      running binary in-place from pre-compiled release archives (or fall back to
//!      `crates.io`).
//! 2. Write Path:
//!    - Exposes [`JsoncManifest::try_update_crate_in_jsonc`][update-jsonc] so external
//!      release tooling can update [`R3BL_VERSIONS_JSONC_URL`] entries in-place while
//!      preserving existing comments and formatting.
//!
//! ## External to this repository: The Release and Distribution Pipeline
//!
//! - Cross-platform binary compilation, archive packaging, SHA-256 checksum generation,
//!   and publishing to [`R3BL_VERSIONS_JSONC_URL`] and release download hosts are
//!   performed by external release tooling that depends on this (`r3bl-cmdr`) library
//!   crate.
//! - This is why this workspace intentionally contains no static
//!   [`R3BL_VERSIONS_JSONC_URL`] file or release-publishing scripts.
//!
//! ## Shared Distribution Contract
//!
//! - Because both open-source crates in this workspace and external R3BL binary crates
//!   depend on `r3bl-cmdr` for self-upgrading, [`CrateDistribution`] supports both
//!   distribution models through a single unified schema:
//!   - [`CrateDistribution::OpenSource`]: Falls back to building the exact version pinned
//!     in [`R3BL_VERSIONS_JSONC_URL`] from `crates.io` via [`PTY`] + `OSC 9;4` progress
//!     tracking when pre-built release archives are unreachable.
//!   - [`CrateDistribution::ClosedSource`]: Upgrades exclusively from pre-built release
//!     archives (with no `crates.io` fallback).
//!
//! # Centralized Version Manifest ([`R3BL_VERSIONS_JSONC_URL`])
//!
//! The version manifest ([`JsoncManifest`]) is the central registry for all R3BL binary
//! crates. These include `r3bl-cmdr` (`giti`, `edi`, `env-source`), `r3bl-build-infra`,
//! and `r3bl-rust-analyzer-mcp-server`.
//!
//! Instead of making unauthenticated requests against multiple individual GitHub release
//! endpoints or `crates.io` APIs, all R3BL binaries query a single lightweight document
//! ([`R3BL_VERSIONS_JSONC_URL`]) to discover current release states, distribution
//! channels, and upgrade strategies.
//!
//! # Main Entry Point: [`UpgradeJourney`] & Typestate Lifecycle
//!
//! To make illegal states unrepresentable and unify startup, dedicated-thread checks, and
//! shutdown under a single compiler-verified state machine, the [`self_upgrade`] engine
//! is built around three pillars:
//!
//! 1. CLI-to-Domain Boundary
//!   - [`UpgradeCLIArgs`] is an opaque [`clap`] adapter at the CLI edge whose private
//!     fields prevent invalid flag combinations from being constructed in Rust code.
//!   - [`UpgradePolicy`] is the pure domain model for upgrade behavior.
//!   - Binaries flatten [`UpgradeCLIArgs`] (mutually exclusive [`--upgrade`][u] or
//!     [`--upgrade-behavior`][ub]) into their argument parser and pass it to
//!     [`upgrade_journey!`], which converts it at the boundary into [`UpgradePolicy`] via
//!     [`UpgradePolicy::from()`][policy-from].
//!
//! 2. [Typestate Pattern]
//!   - [`UpgradeJourney<State>`] encodes the 9-state upgrade machine in the type system.
//!     It holds an immutable [`UpgradeConfig`] alongside a `State` generic type that
//!     implements [`UpgradeJourneyStateMarker`], and each transition consumes
//!     `UpgradeJourney<PrevState>` by value to return `UpgradeJourney<NextState>`.
//!     - Here are the states: [`NotStarted`], [`Disabled`], [`CooldownSkipped`],
//!       [`Checking`], [`UpToDate`], [`UpgradeAvailable`], [`CheckFailed`],
//!       [`Upgrading`], and [`Completed`] (see [State Machine & Lifecycle Flow] on
//!       [`UpgradeJourney`]).
//!     - State transitions are driven exclusively by the [Control Plane] on the
//!       caller/current/main thread and a dedicated thread; see [Thread Model] for
//!       details. The [Execution Plane] performs stateless network/disk work.
//!   - Binaries construct [`UpgradeJourney<NotStarted>`] via [`upgrade_journey!`] and
//!     call [`.start()`], which atomically claims the process-wide singleton:
//!     - It returns [`StartOutcome::AlreadyStarted`] on duplicate calls
//!     - Or it returns [`StartOutcome::Disabled`], [`StartOutcome::ImmediateUpgrade`]
//!       (carrying [`ImmediateUpgradeOutcome`]), or [`StartOutcome::DeferredUpgrade`]
//!       (carrying [`DeferredUpgradeOutcome`]).
//!
//! 3. Coordinated Process-Wide Singleton
//!   - [`SINGLETON`] holds the 2-variant [`SingletonState`] enum inside a leaf
//!     [`ScopedMutex`], encoding the single-start guard alongside the optional
//!     [`DeferredCheckHandle`] of the [`DEDICATED_THREAD_NAME`] OS thread.
//!   - At shutdown, [`handle_upgrade_on_exit`] takes the [`DeferredCheckHandle`] from
//!     [`SINGLETON`], releases the mutex, and joins any in-flight dedicated thread
//!     (preventing deadlock and ensuring short-lived invocations wait up to
//!     [`DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT`][2] when the
//!     [`DEFERRED_CHECK_COOLDOWN_DELAY`][1] timestamp file was expired), then transitions
//!     [`UpgradeAvailable`] -> [`Upgrading`] -> [`Completed`] for one-shot exit handling.
//!
//! ## Control Plane vs. Execution Plane
//!
//! [`self_upgrade`] separates **deciding what to do** from **doing the work**:
//!
//! 1. Control Plane (State Machine, Lifecycle & Threading)
//!    - [`upgrade_journey/`][`UpgradeJourney`] ([`UpgradeJourney`], [`StartOutcome`],
//!      [`ExitOutcome`], [`SingletonState`], and [`SINGLETON`]) decides *whether* and
//!      *when* to check or perform the upgrade.
//!    - It owns the 9-state [`UpgradeJourney<State>`] machine, `1h` cooldown gating,
//!      dedicated-thread spawning and joining, and the process-wide [`SINGLETON`].
//!    - See [Thread Model] on [`NotStarted`] for the thread-by-thread state transition
//!      flow across a deferred upgrade on exit ([`UpgradeStrategy::Deferred`], dedicated
//!      thread + current thread) and an immediate upgrade
//!      ([`UpgradeStrategy::Immediate`], current thread only).
//!
//! 2. Execution Plane (Network, Disk & Build Operations, etc.)
//!    - [`check/`][manifest] and [`upgrade/`][`perform_upgrade`] perform the actual HTTP
//!      manifest fetch ([`fetch_crate_metadata_from_manifest`]), [`mtime`] timestamp file
//!      reads/writes ([`CooldownPolicy`]), archive download, SHA-256 check, binary swap,
//!      [`PTY`] source build ([`perform_upgrade`]), and user prompt
//!      ([`prompt_user_to_upgrade`]).
//!    - These functions are stateless with respect to the state machine: they don't know
//!      or care which [`UpgradeJourney<State>`] is active, and they never import or touch
//!      [`UpgradeJourney`] or [`SINGLETON`]. Instead, they take only the configuration
//!      and metadata parameters they need (`&`[`UpgradeConfig`], `&`[`EndpointUrl`],
//!      `&`[`CrateMetadata`]) and return a result.
//!
//! # Error Handling Policy
//!
//! How the engine handles network or installation errors depends on which
//! [`UpgradeStrategy`] is active, specifically, whether the upgrade runs
//! opportunistically in the background during normal application use or was explicitly
//! requested by the user:
//!
//! 1. When running a deferred upgrade on exit ([`UpgradeStrategy::Deferred`]), errors
//!    never interrupt the user's workflow or alter the binary's exit code:
//!    - In this default mode ([`auto`], [`prompt-to-upgrade`], or [`notify-only`]), the
//!      user invoked the binary to do their normal work rather than to upgrade.
//!    - If the background version check fails (for example, while offline or when the
//!      manifest URL cannot be reached), nothing is printed to the terminal. The engine
//!      transitions to [`CheckFailed`], logs the error when [`DEBUG_SELF_UPGRADE`] is
//!      enabled, and leaves the `1h` cooldown timestamp file untouched so the next run
//!      can try again.
//!    - If a newer version was found and the exit upgrade itself fails during
//!      [`handle_upgrade_on_exit`], the error is reported on [`UserFeedbackChannel`] and
//!      recorded as [`UpgradeResolution::UpgradeFailed`] without failing the
//!      application's own exit status.
//!
//! 2. When running an immediate upgrade ([`UpgradeStrategy::Immediate`]), errors are
//!    reported loudly and cause the process to exit with status code `1`:
//!    - Because the user explicitly passed [`--upgrade`][u] with the sole intent of
//!      upgrading the binary right away, any failure must be surfaced immediately.
//!    - Any manifest check or installation failure prints a clear diagnostic and retry
//!      hint on [`UserFeedbackChannel`], leaves the `1h` cooldown timestamp file
//!      untouched, and returns [`StartOutcome::ImmediateUpgrade`] with
//!      [`ImmediateUpgradeOutcome::CheckFailed`] or
//!      [`ImmediateUpgradeOutcome::Completed`].
//!
//! # Upgrade Source Resolution (Release Archive vs. Source Fallback)
//!
//! Once the engine determines that an upgrade is available (`remote_version >
//! current_version`), it must decide *how* to perform the upgrade before downloading or
//! running build commands. This decision resolves into an [`UpgradeSource`].
//!
//! - Who resolves it:
//!   - Evaluated by [`UpgradeSource::try_resolve`] (and its pure decision function
//!     [`pure_evaluator::evaluate_upgrade_source_pure`]). If a binary download fails
//!     mid-stream, [`UpgradeSource::try_fallback_to_source`][10] evaluates fallback
//!     viability.
//! - Where it is called:
//!   - Invoked by [`perform_upgrade`] immediately before dispatching to download or
//!     compilation routines.
//! - When it runs:
//!   - Right after manifest evaluation confirms that a newer version is available, and
//!     immediately before any bytes are fetched or child processes spawned.
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
//! │ HTTP HEAD probe to release asset URL                   │
//! └─────────────┬────────────────────────────┬─────────────┘
//!               │ Found (200)                │ Network Error / Not Found
//!               │                            │
//! ┌─────────────▼─────────────┐    ┌─────────▼─────────────┐
//! │ UpgradeSource::           │    │ CrateDistribution?    │
//! │ ReleaseArchive            │    └───┬───────────────┬───┘
//! └───────────────────────────┘        │ Closed        │ Open
//!                                      │               │
//!                             ┌───────▼────────┐ ┌─────▼────────────┐
//!                             │ Err(NoViable   │ │ Check $PATH for  │
//!                             │ UpgradeSource) │ │ rustup           │
//!                             └────────────────┘ └───┬──────────┬───┘
//!                                       rustup found │          │ missing
//!                                                    │          │
//!                                      ┌─────────────▼─┐  ┌─────▼──────────┐
//!                                      │ UpgradeSource │  │ Err(NoViable   │
//!                                      │ ::BuildFrom   │  │ UpgradeSource) │
//!                                      │ SourceRustup  │  └────────────────┘
//!                                      └───────────────┘
//! ```
//!
//! [10]: crate::self_upgrade::UpgradeSource::try_fallback_to_source
//! [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
//! [2]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT
//! [`.start()`]: crate::self_upgrade::UpgradeJourney::start
//! [`auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`Checking`]: crate::self_upgrade::Checking
//! [`clap`]: mod@clap
//! [`Completed`]: crate::self_upgrade::Completed
//! [`CooldownPolicy`]: crate::self_upgrade::CooldownPolicy
//! [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
//! [`CrateDistribution::OpenSource`]: crate::self_upgrade::CrateDistribution::OpenSource
//! [`CrateDistribution`]: crate::self_upgrade::CrateDistribution
//! [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
//! [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
//! [`DEDICATED_THREAD_NAME`]: crate::self_upgrade::dedicated_thread::DEDICATED_THREAD_NAME
//! [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
//! [`DeferredCheckOutcome`]: crate::self_upgrade::DeferredCheckOutcome
//! [`DeferredUpgradeOutcome`]: crate::self_upgrade::DeferredUpgradeOutcome
//! [`Disabled`]: crate::self_upgrade::Disabled
//! [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
//! [`ExitOutcome`]: crate::self_upgrade::ExitOutcome
//! [`handle_upgrade_on_exit`]: crate::self_upgrade::handle_upgrade_on_exit
//! [`ImmediateUpgradeOutcome::CheckFailed`]: crate::self_upgrade::ImmediateUpgradeOutcome::CheckFailed
//! [`ImmediateUpgradeOutcome::Completed`]: crate::self_upgrade::ImmediateUpgradeOutcome::Completed
//! [`ImmediateUpgradeOutcome`]: crate::self_upgrade::ImmediateUpgradeOutcome
//! [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
//! [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
//! [`notify-only`]: crate::self_upgrade::UpgradeActionOnExit::NotifyOnly
//! [`NotStarted`]: crate::self_upgrade::NotStarted
//! [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
//! [`prompt-to-upgrade`]: crate::self_upgrade::UpgradeActionOnExit::PromptToUpgrade
//! [`prompt_user_to_upgrade`]: crate::self_upgrade::prompt_user_to_upgrade
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
//! [`ScopedMutex`]: r3bl_tui::ScopedMutex
//! [`self_upgrade`]: mod@crate::self_upgrade
//! [`SINGLETON`]: crate::self_upgrade::SINGLETON
//! [`SingletonState`]: crate::self_upgrade::SingletonState
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`StartOutcome::DeferredUpgrade`]: crate::self_upgrade::StartOutcome::DeferredUpgrade
//! [`StartOutcome::Disabled`]: crate::self_upgrade::StartOutcome::Disabled
//! [`StartOutcome::ImmediateUpgrade`]: crate::self_upgrade::StartOutcome::ImmediateUpgrade
//! [`StartOutcome`]: crate::self_upgrade::StartOutcome
//! [`upgrade_journey!`]: crate::upgrade_journey
//! [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
//! [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
//! [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
//! [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourney<State>`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeJourneyStateMarker`]: crate::self_upgrade::UpgradeJourneyStateMarker
//! [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
//! [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve
//! [`UpgradeSource`]: crate::self_upgrade::UpgradeSource
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
//! [`Upgrading`]: crate::self_upgrade::Upgrading
//! [`UpToDate`]: crate::self_upgrade::UpToDate
//! [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
//! [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
//! [jh]: std::thread::JoinHandle
//! [manifest]: crate::self_upgrade::fetch_crate_metadata_from_manifest
//! [policy-from]: crate::self_upgrade::UpgradePolicy::from
//! [State Machine & Lifecycle Flow]: crate::self_upgrade::UpgradeJourney#state-machine--lifecycle-flow
//! [Thread Model]: crate::self_upgrade::NotStarted#thread-model
//! [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
//! [u]: crate::self_upgrade::upgrade_cli_args::UPGRADE_ARG_LONG
//! [ub]: crate::self_upgrade::upgrade_cli_args::UPGRADE_BEHAVIOR_ARG_LONG
//! [update-jsonc]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc

// Private submodules.
mod check;
mod config;
mod constants;
mod jsonc_manifest;
mod upgrade;
mod upgrade_journey;

// Public barrel re-exports.
pub use check::*;
pub use config::*;
pub use constants::*;
pub use jsonc_manifest::*;
pub use upgrade::*;
pub use upgrade_journey::*;

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
//! 1. Read and Upgrade Path: The client runtime is embedded into product binaries to
//!    fetch [`R3BL_VERSIONS_JSON_URL`] ([`check_version_against_manifest`]), parse
//!    [`JsoncManifest::try_parse_jsonc`], and guard against downgrading local dev builds
//!    (`remote_version > current_version` via
//!    [`SelfBinaryUpgradeConfig::current_version`]). When an upgrade runs, it streams and
//!    verifies `.tar.gz` and `.sha256` release assets from
//!    [`R3BL_BINARIES_RELEASE_BASE_URL`], swaps binaries in-place via [`self_replace`]
//!    (plus any sibling binaries), or falls back to `crates.io`.
//! 2. Write Path: The shared schema and in-place JSONC mutator (via [`CST`] which is
//!    provided by the [JSONC parser] crate) is exposed via
//!    [`JsoncManifest::try_update_crate_in_jsonc`] as a library API so external release
//!    tooling can losslessly update `versions.jsonc` entries while preserving all
//!    existing comments, blank lines, and formatting.
//!
//! ## External to this repository: The Release and Distribution Pipeline
//!
//! - Cross-platform binary compilation, archive packaging, `.sha256` generation, and
//!   publishing of `versions.jsonc` to `r3bl.com` and `github.com/r3bl-org/r3bl-binaries`
//!   are performed by external maintainer release automation that links against this
//!   (`r3bl_tui`) crate.
//! - This is why this workspace intentionally contains no static `versions.jsonc` file or
//!   release-publishing scripts.
//!
//! ## Shared Distribution Contract
//!
//! - Because both open-source crates in this workspace and external R3BL binary crates
//!   link against `r3bl_tui` for self-upgrading, [`CrateDistribution`] supports both
//!   distribution models through a single unified schema:
//!   - [`CrateDistribution::OpenSource`]: Falls back to building the exact version pinned
//!     in `versions.jsonc` from `crates.io` via [`PTY`] + `OSC 9;4` progress tracking
//!     when pre-built release archives are unreachable.
//!   - [`CrateDistribution::ClosedSource`]: Upgrades exclusively from pre-built GitHub
//!     release archives (with no `crates.io` fallback).
//!
//! # Centralized Version Manifest ([`R3BL_VERSIONS_JSON_URL`])
//!
//! The version manifest ([`JsoncManifest`]) is the central registry for all R3BL
//! binary crates. These include `r3bl-cmdr` (`giti`, `edi`, `env-source`),
//! `r3bl-build-infra`, and `r3bl-rust-analyzer-mcp-server`.
//!
//! Instead of making unauthenticated requests against multiple individual GitHub release
//! endpoints or `crates.io` APIs, all R3BL binaries query a single lightweight document
//! ([`R3BL_VERSIONS_JSON_URL`]) to discover current release states, distribution
//! channels, and upgrade strategies.
//!
//! # Invocation Modes and Error Handling
//!
//! Self updates & upgrades run in one of two modes:
//!
//! 1. Normal execution (startup check + exit handler, silent failure):
//!    - When the binary starts, a check is initiated to determine whether an update is
//!      available. The result of checking is stored in a shared memory location, so that
//!      it can be read at the time of exit to perform the actual upgrade if needed.
//!    - There are 2 entry points for this (one sync and another async), since this engine
//!      is agnostic to the binary's async or sync nature:
//!        - **Async**: spawns [`tokio::task`] ([`start_background_version_check`]) in
//!          async binaries.
//!        - **Sync**: spawns dedicated [`std::thread`] with an isolated runtime
//!          ([`start_background_version_check_sync`]) in synchronous binaries, ensuring 0
//!          ms user-perceived startup latency.
//!    - When the spawned task or thread finishes fetching and parsing the manifest, it
//!      evaluates [`SemVer`] and atomically populates the in-memory cache
//!      ([`UpgradeCheckCache`]) with [`UpgradeCheckStatus`].
//!    - At process exit, [`handle_upgrade_at_exit`] (or [`handle_upgrade_at_exit_sync`])
//!      reads the cached state from [`UpgradeCheckCache`] via [`UpgradeCheckCache::get`]
//!      with 0 ms latency (no second HTTP round-trip), executing actions according to
//!      [`UpdateBehavior`].
//!    - If a crate has not yet been published to the remote manifest, if the remote
//!      server is down, or if a network timeout or manifest parse error occurs, the
//!      engine logs at [`tracing::debug!`] (gated behind [`SCRIPT_MOD_DEBUG`]) and marks
//!      the state as [`UpgradeCheckStatus::CheckFailed`].
//!    - Users running routine operations (such as `CLI` commands or `MCP` server
//!      sessions) when the server is down or unreachable are never interrupted with error
//!      dialogs or failed exit codes.
//!
//! 2. Explicit upgrade (`--upgrade` flag):
//!    - Runs when a user or script passes `--upgrade`. There are 2 entry points depending
//!      on the binary's runtime model:
//!        - **Async**: [`run_upgrade_cli_arg_interceptor`] in async binaries (such as
//!          `giti` and `edi`).
//!        - **Sync**: [`run_upgrade_cli_arg_interceptor_sync`] in synchronous binaries
//!          (such as `env-source` and `rust-analyzer-mcp-server`), running on an isolated
//!          runtime ([`run_upgrade_sync`]).
//!    - If the manifest server is down, unreachable, or cannot be parsed, the failure is
//!      reported to [`stderr`] and the process exits with status code `1`.
//!
//! # Command-Line Interface (`CLI`) Integration
//!
//! All workspace binaries share a unified command-line interface for upgrades and
//! update checks by flattening [`UpgradeCLIArgs`] into their top-level argument parser:
//!
//! - `--upgrade`: Triggers an immediate self-upgrade and exits.
//! - `--update-behavior`: Selects the exit check policy ([`UpdateBehavior`]).
//!
//! These flags are mutually exclusive to prevent contradictory or silently ignored
//! arguments. See [`UpgradeCLIArgs`] and [`UpdateBehavior`] for details.
//!
//! # Upgrade Strategy Resolution (GitHub Release vs. Source Fallback)
//!
//! Once the engine determines that an upgrade is available (`remote_version >
//! current_version`), it must decide *how* to perform the upgrade before downloading or
//! running build commands. This decision resolves into an [`UpgradeSource`].
//!
//! - **Who resolves it**: Evaluated by [`UpgradeSource::try_resolve`] (and its pure
//!   decision function [`pure_evaluator::evaluate_upgrade_source_pure`]). If a binary
//!   download fails mid-stream, [`UpgradeSource::try_fallback_to_source`] evaluates
//!   fallback viability.
//! - **Where it is called**: Invoked by [`execute_upgrade_with_metadata`] immediately
//!   before dispatching to download or compilation routines.
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
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │ HTTP HEAD probe to GitHub Release asset URL            │
//! └─────────────┬────────────────────────────┬─────────────┘
//!               │ Found (200)                │ Network Error / Not Found
//!               ▼                            ▼
//! ┌───────────────────────────┐    ┌───────────────────────┐
//! │ UpgradeSource::           │    │ CrateDistribution?    │
//! │ GithubReleaseArtifact     │    └───┬───────────────┬───┘
//! └───────────────────────────┘        │ Closed        │ Open
//!                                      ▼               ▼
//!                              ┌───────────────┐ ┌──────────────────┐
//!                              │ Err(NoViable  │ │ Check $PATH for  │
//!                              │ UpgradeSource)│ │ rustup           │
//!                              └───────────────┘ └───┬──────────┬───┘
//!                                        rustup found│          │ missing
//!                                                    ▼          ▼
//!                                      ┌─────────────┐   ┌───────────────┐
//!                                      │UpgradeSource│   │ Err(NoViable  │
//!                                      │::BuildFrom  │   │ UpgradeSource)│
//!                                      │SourceRustup │   └───────────────┘
//!                                      └─────────────┘
//! ```
//!
//! [`check_version_against_manifest`]: crate::core::script::self_update::check_version_against_manifest
//! [`CrateDistribution::ClosedSource`]: crate::core::script::self_update::CrateDistribution::ClosedSource
//! [`CrateDistribution::OpenSource`]: crate::core::script::self_update::CrateDistribution::OpenSource
//! [`CrateDistribution`]: crate::core::script::self_update::CrateDistribution
//! [`CST`]: jsonc_parser::cst::CstRootNode
//! [`execute_upgrade_with_metadata`]: crate::core::script::self_update::upgrade_execution::execute_upgrade_with_metadata
//! [`handle_upgrade_at_exit_sync`]: crate::core::script::self_update::handle_upgrade_at_exit_sync
//! [`handle_upgrade_at_exit`]: crate::core::script::self_update::handle_upgrade_at_exit
//! [`JsoncManifest::try_parse_jsonc`]: crate::core::script::self_update::JsoncManifest::try_parse_jsonc
//! [`JsoncManifest::try_update_crate_in_jsonc`]: crate::core::script::self_update::JsoncManifest::try_update_crate_in_jsonc
//! [`JsoncManifest`]: crate::core::script::self_update::JsoncManifest
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`pure_evaluator::evaluate_upgrade_source_pure`]: crate::core::script::self_update::pure_evaluator::evaluate_upgrade_source_pure
//! [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::self_update::R3BL_BINARIES_RELEASE_BASE_URL
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
//! [`run_upgrade_cli_arg_interceptor_sync`]: crate::core::script::self_update::run_upgrade_cli_arg_interceptor_sync
//! [`run_upgrade_cli_arg_interceptor`]: crate::core::script::self_update::run_upgrade_cli_arg_interceptor
//! [`run_upgrade_sync`]: crate::core::script::self_update::run_upgrade_sync
//! [`SCRIPT_MOD_DEBUG`]: crate::core::script::SCRIPT_MOD_DEBUG
//! [`SelfBinaryUpgradeConfig::current_version`]: crate::core::script::self_update::SelfBinaryUpgradeConfig::current_version
//! [`SemVer`]: https://semver.org/
//! [`start_background_version_check_sync`]: crate::core::script::self_update::start_background_version_check_sync
//! [`start_background_version_check`]: crate::core::script::self_update::start_background_version_check
//! [`stderr`]: std::io::stderr
//! [`UpdateBehavior`]: crate::core::script::self_update::UpdateBehavior
//! [`UpgradeCheckCache::get`]: crate::core::script::self_update::UpgradeCheckCache::get
//! [`UpgradeCheckCache`]: crate::core::script::self_update::UpgradeCheckCache
//! [`UpgradeCheckStatus::CheckFailed`]: crate::core::script::self_update::UpgradeCheckStatus::CheckFailed
//! [`UpgradeCheckStatus`]: crate::core::script::self_update::UpgradeCheckStatus
//! [`UpgradeCLIArgs`]: crate::core::script::self_update::UpgradeCLIArgs
//! [`UpgradeSource::try_fallback_to_source`]: crate::core::script::self_update::UpgradeSource::try_fallback_to_source
//! [`UpgradeSource::try_resolve`]: crate::core::script::self_update::UpgradeSource::try_resolve
//! [`UpgradeSource`]: crate::core::script::self_update::UpgradeSource
//! [JSONC parser]: mod@jsonc_parser

// Private submodules.
mod constants;
mod entry_point;
mod parser;
mod update;
mod upgrade;

// Public barrel re-exports.
pub use constants::*;
pub use entry_point::*;
pub use parser::*;
pub use update::*;
pub use upgrade::*;

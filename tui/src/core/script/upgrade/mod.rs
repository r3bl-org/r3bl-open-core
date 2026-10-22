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
//!    [`VersionsManifest::try_parse_jsonc`], and guard against downgrading local dev
//!    builds (`remote_version > current_version` via
//!    [`SelfBinaryUpgradeConfig::current_version`]). When an upgrade runs, it streams and
//!    verifies `.tar.gz` and `.sha256` release assets from
//!    [`R3BL_BINARIES_RELEASE_BASE_URL`], swaps binaries in-place via [`self_replace`]
//!    (plus any sibling binaries), or falls back to `crates.io`.
//! 2. Write Path: The shared schema and in-place JSONC mutator (via [`CST`] which is
//!    provided by the [JSONC parser] crate) is exposed via
//!    [`VersionsManifest::try_update_crate_in_jsonc`] as a library API so external
//!    release tooling can losslessly update `versions.jsonc` entries while preserving all
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
//! # Centralized Version Manifest (`r3bl.com/versions.jsonc`)
//!
//! The version manifest ([`VersionsManifest`]) is the central registry for all R3BL
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
//!      it can be at the time of exit to perform the actual upgrade if needed.
//!    - There are 2 entry points for this one sync and another async, since this engine
//!      is agnostic to the binary's async or sync nature:
//!        - **Async**: spawns [`tokio::task`] ([`start_background_version_check`]) in
//!          async binaries.
//!        - **Sync**: spawns dedicated [`std::thread`] with an isolated runtime
//!          ([`start_background_version_check_sync`]) in synchronous binaries, ensuring 0
//!          ms user-perceived startup latency.
//!    - When the spawned task or thread finishes fetching and parsing the manifest, it
//!      populates the in-memory cache ([`CACHED_CRATE_METADATA`]) and updates the atomic
//!      status flag ([`UPGRADE_CHECK_STATE`]).
//!    - At process exit, [`handle_upgrade_at_exit`] checks the atomic state and retrieves
//!      the resolved version metadata from the in-memory cache via
//!      [`get_cached_crate_metadata`] with 0 ms latency (no second HTTP round-trip),
//!      executing actions according to [`UpdateBehavior`].
//!    - If a crate has not yet been published to the remote manifest, if the remote
//!      server is down, or if a network timeout or manifest parse error occurs, the
//!      engine logs at [`tracing::debug!`] (gated behind [`SCRIPT_MOD_DEBUG`]) and marks
//!      the state as [`UpgradeCheckResult::FailedCheck`].
//!    - Users running routine operations (such as `CLI` commands or `MCP` server
//!      sessions) when the server is down or unreachable are never interrupted with error
//!      dialogs or failed exit codes.
//!
//! 2. Explicit upgrade (`--upgrade` flag):
//!    - Runs when a user or script passes `--upgrade`
//!      ([`run_upgrade_cli_arg_interceptor`]).
//!    - If the manifest server is down, unreachable, or cannot be parsed, the failure is
//!      reported to [`stderr`] and the process exits with status code `1`.
//!
//! [`CACHED_CRATE_METADATA`]: crate::core::script::upgrade::CACHED_CRATE_METADATA
//! [`check_version_against_manifest`]: crate::core::script::upgrade::check_version_against_manifest
//! [`CrateDistribution::ClosedSource`]: crate::core::script::upgrade::CrateDistribution::ClosedSource
//! [`CrateDistribution::OpenSource`]: crate::core::script::upgrade::CrateDistribution::OpenSource
//! [`CrateDistribution`]: crate::core::script::upgrade::CrateDistribution
//! [`CST`]: jsonc_parser::cst::CstRootNode
//! [`get_cached_crate_metadata`]: crate::core::script::upgrade::get_cached_crate_metadata
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::upgrade::R3BL_BINARIES_RELEASE_BASE_URL
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
//! [`run_upgrade_cli_arg_interceptor`]: crate::core::script::upgrade::run_upgrade_cli_arg_interceptor
//! [`SCRIPT_MOD_DEBUG`]: crate::core::script::SCRIPT_MOD_DEBUG
//! [`SelfBinaryUpgradeConfig::current_version`]: crate::core::script::upgrade::SelfBinaryUpgradeConfig::current_version
//! [`start_background_version_check_sync`]: crate::core::script::upgrade::start_background_version_check_sync
//! [`start_background_version_check`]: crate::core::script::upgrade::start_background_version_check
//! [`stderr`]: std::io::stderr
//! [`UpdateBehavior`]: crate::core::script::upgrade::UpdateBehavior
//! [`UPGRADE_CHECK_STATE`]: crate::core::script::upgrade::UPGRADE_CHECK_STATE
//! [`UpgradeCheckResult::FailedCheck`]: crate::core::script::upgrade::UpgradeCheckResult::FailedCheck
//! [`VersionsManifest::try_parse_jsonc`]: crate::core::script::upgrade::VersionsManifest::try_parse_jsonc
//! [`VersionsManifest::try_update_crate_in_jsonc`]: crate::core::script::upgrade::VersionsManifest::try_update_crate_in_jsonc
//! [`VersionsManifest`]: crate::core::script::upgrade::VersionsManifest
//! [JSONC parser]: mod@jsonc_parser

// Private submodules.
mod constants;
mod github_release;
mod run_upgrade;
mod source_build;
mod strategy_resolver;
mod target_triple;
mod ui_strings;
mod upgrade_types;
mod version_check;
mod versions_manifest;

// Public barrel re-exports.
pub use constants::*;
pub use github_release::*;
pub use run_upgrade::*;
pub use source_build::*;
pub use strategy_resolver::*;
pub use target_triple::*;
pub use upgrade_types::*;
pub use version_check::*;
pub use versions_manifest::*;

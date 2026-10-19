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
//!    parse [`R3BL_VERSIONS_JSON_URL`] via [`VersionsManifest::try_parse_jsonc`], stream
//!    and verify `.tar.gz` and `.sha256` release assets from
//!    [`R3BL_BINARIES_RELEASE_BASE_URL`], swap binaries in-place via [`self_replace`], or
//!    fall back to `crates.io` (only for open source crates/binaries).
//! 2. Write Path: The shared schema and in-place JSONC mutator (via [`CST`] which is
//!    provided by the [JSONC parser] crate) is exposed via
//!    [`VersionsManifest::update_crate_in_jsonc_str`] as a library API so external
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
//!   distribution models ([`CrateDistribution::OpenSource`] with `crates.io` fallback,
//!   and [`CrateDistribution::ClosedSource`] via pre-built binary releases only) through
//!   a single unified schema.
//!
//! # Centralized Version Manifest (`r3bl.com/versions.jsonc`)
//!
//! The version manifest ([`VersionsManifest`]) is the central registry for all R3BL
//! binary products. These include `r3bl-cmdr`, `edi`, `giti`, `r3bl-build-infra`, and
//! `r3bl-rust-analyzer-mcp-server`.
//!
//! Instead of making unauthenticated requests against multiple individual GitHub release
//! endpoints or `crates.io` APIs all R3BL binaries query a single lightweight document
//! ([`R3BL_VERSIONS_JSON_URL`]) to discover current release states, distribution
//! channels, and upgrade strategies.
//!
//! # How Upgrades Work
//!
//! 1. Discovery & version guarding:
//!    - [`check_version_against_manifest`] fetches [`R3BL_VERSIONS_JSON_URL`], parses
//!      [`VersionsManifest`], and extracts the crate's [`CrateVersionMetadata`]:
//!      `version` ([`CrateVersion`]), `crate_distribution` ([`CrateDistribution`]), and
//!      `released_on`.
//!    - [`CrateVersion`] enforces strict `"MAJOR.MINOR.PATCH"` semantic versioning
//!      (rejecting `v` prefixes, pre-release tags, and build metadata).
//!    - Compares the remote [`CrateVersion`] strictly against
//!      [`UpgradeConfig::local_version`] (`env!("CARGO_PKG_VERSION")` from the calling
//!      binary, `remote_version > local_version`) so local development builds are never
//!      downgraded.
//! 2. Primary engine: pre-built binary download:
//!    - Streams `.tar.gz` archives from [`R3BL_BINARIES_RELEASE_BASE_URL`] across Linux,
//!      macOS, and Windows with live `%` and `MiB` progress, verifies `.sha256`
//!      checksums, unpacks the archive, and atomically swaps the running executable
//!      in-place via [`self_replace`] (plus updates any sibling binaries).
//! 3. Distribution routing & fallback:
//!    - Open-source binaries ([`CrateDistribution::OpenSource`]): When the pre-compiled
//!      binary release is unreachable, falls back to building the exact version pinned in
//!      `versions.jsonc` from `crates.io` via [`PTY`] + `OSC 9;4` progress tracking
//!      (`rustup toolchain install nightly --force` + `cargo +nightly install <crate>
//!      --version <pinned_ver> --force`).
//!    - Closed-source binaries ([`CrateDistribution::ClosedSource`]): Downloads
//!      exclusively from GitHub releases (no `crates.io` fallback), using `released_on`
//!      to validate the user's active maintenance window before upgrading.
//!
//! # Invocation Modes and Error Handling
//!
//! Upgrades run in one of two modes:
//!
//! 1. Background version check (silent failure):
//!    - Runs in the background on startup ([`start_background_version_check`]) or before
//!      exit ([`handle_upgrade_at_exit`]).
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
//! [`check_version_against_manifest`]: crate::core::script::upgrade::check_version_against_manifest
//! [`CrateDistribution::ClosedSource`]: crate::core::script::upgrade::CrateDistribution::ClosedSource
//! [`CrateDistribution::OpenSource`]: crate::core::script::upgrade::CrateDistribution::OpenSource
//! [`CrateDistribution`]: crate::core::script::upgrade::CrateDistribution
//! [`CrateVersion`]: crate::core::script::upgrade::CrateVersion
//! [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
//! [`CST`]: jsonc_parser::cst::CstRootNode
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::upgrade::R3BL_BINARIES_RELEASE_BASE_URL
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
//! [`run_upgrade_cli_arg_interceptor`]: crate::core::script::upgrade::run_upgrade_cli_arg_interceptor
//! [`SCRIPT_MOD_DEBUG`]: crate::core::script::SCRIPT_MOD_DEBUG
//! [`start_background_version_check`]: crate::core::script::upgrade::start_background_version_check
//! [`stderr`]: std::io::stderr
//! [`UpgradeCheckResult::FailedCheck`]: crate::core::script::upgrade::UpgradeCheckResult::FailedCheck
//! [`UpgradeConfig::local_version`]: crate::core::script::upgrade::UpgradeConfig::local_version
//! [`VersionsManifest::try_parse_jsonc`]: crate::core::script::upgrade::VersionsManifest::try_parse_jsonc
//! [`VersionsManifest::update_crate_in_jsonc_str`]: crate::core::script::upgrade::VersionsManifest::update_crate_in_jsonc_str
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

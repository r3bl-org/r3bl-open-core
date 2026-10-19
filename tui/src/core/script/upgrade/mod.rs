// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Update checking and in-place upgrades for R3BL binary products.
//!
//! This module handles update discovery for open-source and closed-source tools,
//! downloads new releases, and replaces the running binary on Linux, macOS, and Windows.
//!
//! # Centralized Version Manifest (`r3bl.com/versions.jsonc`)
//!
//! The version manifest ([`VersionsManifest`]) is the central registry for all R3BL
//! binary applications and developer tools (including `r3bl-cmdr`, `edi`, `giti`,
//! `r3bl-build-infra`, and `r3bl-rust-analyzer-mcp-server`).
//!
//! Instead of making unauthenticated requests against multiple individual GitHub release
//! endpoints or `crates.io` APIs across dozens of executables, all R3BL binaries query
//! a single lightweight document ([`R3BL_VERSIONS_JSON_URL`]) to discover current release
//! states, distribution channels, and upgrade strategies.
//!
//! # How Upgrades Work
//!
//! 1. **Discovery & version guarding (`r3bl.com/versions.jsonc` via
//!    [`VersionsManifest`])**:
//!    - Queries [`R3BL_VERSIONS_JSON_URL`] for `version` ([`CrateVersion`]),
//!      `crate_distribution` ([`CrateDistribution`]), and `released_on`.
//!    - Requires strict `"MAJOR.MINOR.PATCH"` semantic versioning (rejecting `v`
//!      prefixes, pre-release tags, and build metadata).
//!    - Compares the remote [`CrateVersion`] strictly against the local compile-time
//!      version (`remote_version > local_version`) so local development builds and newer
//!      working trees are never downgraded or overwritten.
//! 2. **Primary engine: pre-built binary download
//!    ([`R3BL_BINARIES_RELEASE_BASE_URL`])**:
//!    - Streams pre-compiled `.tar.gz` archives across Linux, macOS, and Windows with
//!      live `%` and `MiB` progress, verifies `.sha256` checksums, unpacks the archive,
//!      and atomically swaps the running executable in-place via [`self_replace`] (plus
//!      updates any sibling binaries in the archive).
//! 3. **Distribution routing & fallback**:
//!    - **Open-source binaries** ([`CrateDistribution::OpenSource`]): When the
//!      pre-compiled binary release is unreachable, automatically falls back to building
//!      the exact version pinned in `versions.jsonc` from `crates.io` via [`PTY`] + `OSC
//!      9;4` progress tracking (`rustup toolchain install nightly --force` + `cargo
//!      +nightly install <crate> --version <pinned_ver> --force`).
//!    - **Closed-source binaries** ([`CrateDistribution::ClosedSource`]): Downloads
//!      exclusively from GitHub releases (no `crates.io` fallback), using `released_on`
//!      to validate the user's active maintenance window before upgrading.
//!
//! # Invocation Modes and Error Handling
//!
//! Upgrades run in one of two modes:
//!
//! 1. **Background version check (silent failure)**:
//!    - Runs in the background on startup ([`start_background_version_check`]) or before
//!      exit ([`handle_upgrade_at_exit`]).
//!    - Network timeouts, offline environments, and manifest parse errors log at
//!      [`tracing::debug!`] (gated behind [`SCRIPT_MOD_DEBUG`]) and mark the state as
//!      [`UpgradeCheckResult::FailedCheck`].
//!    - Offline or air-gapped users running routine operations (such as `CLI` commands or
//!      `MCP` server sessions) are never interrupted with error dialogs or failed exit
//!      codes.
//!
//! 2. **Explicit upgrade (`--upgrade` flag)**:
//!    - Runs when a user or script passes `--upgrade`
//!      ([`run_upgrade_cli_arg_interceptor`]).
//!    - If the manifest cannot be reached or parsed, the failure is reported to
//!      [`stderr`] and the process exits with status code `1`.
//!
//! [`CrateDistribution::ClosedSource`]: crate::core::script::upgrade::CrateDistribution::ClosedSource
//! [`CrateDistribution::OpenSource`]: crate::core::script::upgrade::CrateDistribution::OpenSource
//! [`CrateVersion`]: crate::core::script::upgrade::CrateVersion
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::upgrade::R3BL_BINARIES_RELEASE_BASE_URL
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
//! [`run_upgrade_cli_arg_interceptor`]: crate::core::script::upgrade::run_upgrade_cli_arg_interceptor
//! [`SCRIPT_MOD_DEBUG`]: crate::core::script::SCRIPT_MOD_DEBUG
//! [`start_background_version_check`]: crate::core::script::upgrade::start_background_version_check
//! [`stderr`]: std::io::stderr
//! [`UpgradeCheckResult::FailedCheck`]: crate::core::script::upgrade::UpgradeCheckResult::FailedCheck
//! [`VersionsManifest`]: crate::core::script::upgrade::VersionsManifest

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

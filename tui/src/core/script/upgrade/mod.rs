// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Update checking and in-place upgrades for R3BL binary products.
//!
//! This module handles update discovery for open-source and closed-source tools,
//! downloads new releases, and replaces the running binary on Linux, macOS, and Windows.
//!
//! # How Upgrades Work
//!
//! 1. **Check version manifest (`r3bl.com/versions.jsonc` via [`VersionsManifest`])**:
//!    Queries [`R3BL_VERSIONS_JSON_URL`] for `version`, `crate_distribution`, and
//!    `released_on`. Compares versions strictly (`remote_version > local_version`) so
//!    local development builds are never downgraded.
//! 2. **Download pre-built binary (`github.com/r3bl-org/r3bl-binaries`)**: Streams
//!    pre-compiled `.tar.gz` archives across Linux, macOS, and Windows with live `%` and
//!    `MiB` progress, verifies `.sha256` checksums, unpacks the archive, and atomically
//!    swaps the running executable in-place via [`self_replace`] (plus updates any
//!    sibling binaries in the archive).
//! 3. **Source compilation fallback (open source only)**: When `crate_distribution` is
//!    [`CrateDistribution::OpenSource`] and the binary release is unreachable,
//!    automatically falls back to building the exact version pinned in `versions.jsonc`
//!    via [`PTY`] + `OSC 9;4` progress tracking (`rustup toolchain install nightly
//!    --force` + `cargo +nightly install <crate> --version <pinned_ver> --force`).
//!
//! # Invocation Modes and Error Handling
//!
//! Upgrades run in one of two modes:
//!
//! 1. **Background version check (silent failure)**:
//!    - Runs in the background on startup ([`start_background_version_check`]) or before exit
//!      ([`handle_upgrade_at_exit`]).
//!    - Network timeouts, offline environments, and manifest parse errors log at
//!      [`tracing::debug!`] (gated behind [`SCRIPT_MOD_DEBUG`]) and mark the state as
//!      [`UpgradeCheckResult::FailedCheck`].
//!    - Offline or air-gapped users running routine operations (such as CLI commands or
//!      MCP server sessions) are never interrupted with error dialogs or failed exit
//!      codes.
//!
//! 2. **Explicit upgrade (`--upgrade` flag)**:
//!    - Runs when a user or script passes `--upgrade`
//!      ([`run_upgrade_cli_arg_interceptor`]).
//!    - If the manifest cannot be reached or parsed, the failure is reported to
//!      [`stderr`] and the process exits with status code `1`.
//!
//! [`CrateDistribution::OpenSource`]: crate::core::script::upgrade::CrateDistribution::OpenSource
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
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

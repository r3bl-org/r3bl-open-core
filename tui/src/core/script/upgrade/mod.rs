// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unified hybrid binary self-upgrade engine for open-source and closed-source R3BL
//! binaries.
//!
//! # Architecture
//!
//! 1. **Sole Source of Truth (`r3bl.com/versions.json`)**:
//!    Queries `https://r3bl.com/versions.json` for `version`, `open_source`, and
//!    `released_on`. Compares versions strictly (`remote_version > local_version`) so
//!    local development builds ahead of `versions.json` are never downgraded.
//! 2. **Primary Engine (`github.com/r3bl-org/r3bl-binaries`)**:
//!    Streams pre-compiled `.tar.gz` (Linux/macOS) or `.zip` (Windows) archives with live
//!    `%` and `MB` progress, verifies the `.sha256` checksum, unpacks the archive, and
//!    atomically swaps the running executable in-place via [`self_replace`] (plus updates
//!    any sibling binaries in the archive).
//! 3. **Fallback Engine (`crates.io` Source Build — Open Source Only)**:
//!    When `open_source == true` and the binary release is unreachable, automatically
//!    falls back to building the exact version pinned in `versions.json` via [`PTY`] +
//!    `OSC 9;4` progress tracking (`rustup toolchain install nightly --force` +
//!    `cargo +nightly install <crate> --version <pinned_ver> --force`).
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

// Private submodules.
mod constants;
mod github_release;
mod run_upgrade;
mod source_build;
mod strategy_resolver;
mod target_triple;
mod types;
pub mod ui_strings;
mod version_check;

// Public barrel re-exports.
pub use constants::*;
pub use github_release::*;
pub use run_upgrade::*;
pub use source_build::*;
pub use strategy_resolver::*;
pub use target_triple::*;
pub use types::*;
pub use version_check::*;

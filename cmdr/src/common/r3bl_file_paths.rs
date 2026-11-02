// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized cross-platform filesystem path resolver for all R3BL persistent
//! configuration and machine-scoped cache/state files across **Linux**, **macOS**, and
//! **Windows**.
//!
//! All R3BL open-source and closed-source products consolidate their user-level files
//! under a single top-level [`R3BL_TOP_LEVEL_DIR`] (`"r3bl"`) umbrella directory inside
//! the OS-specific user configuration (`<config_dir>`, via [`dirs::config_dir`]) and
//! cache (`<cache_dir>`, via [`dirs::cache_dir`]) folders.
//!
//! # Platform Base Directories
//!
//! The `<config_dir>` and `<cache_dir>` base paths are resolved dynamically per operating
//! system by the [`dirs`] crate (note that `.config` and `.cache` are the Linux XDG
//! folder names; macOS and Windows use their native OS equivalents):
//!
//! - **Linux**:
//!   - `<config_dir>/r3bl` ([`try_get_r3bl_config_dir`]): `~/.config/r3bl`
//!     (`$XDG_CONFIG_HOME/r3bl`)
//!   - `<cache_dir>/r3bl` ([`try_get_r3bl_cache_dir`]): `~/.cache/r3bl`
//!     (`$XDG_CACHE_HOME/r3bl`)
//! - **macOS**:
//!   - `<config_dir>/r3bl` ([`try_get_r3bl_config_dir`]): `~/Library/Application
//!     Support/r3bl`
//!   - `<cache_dir>/r3bl` ([`try_get_r3bl_cache_dir`]): `~/Library/Caches/r3bl`
//! - **Windows**:
//!   - `<config_dir>/r3bl` ([`try_get_r3bl_config_dir`]): `%APPDATA%\r3bl`
//!     (`C:\Users\<user>\AppData\Roaming\r3bl`)
//!   - `<cache_dir>/r3bl` ([`try_get_r3bl_cache_dir`]): `%LOCALAPPDATA%\r3bl`
//!     (`C:\Users\<user>\AppData\Local\r3bl`)
//!
//! # Directory Hierarchy
//!
//! Inside `<config_dir>/r3bl` and `<cache_dir>/r3bl`, the layout is identical across
//! Linux, macOS, and Windows:
//!
//! ```text
//! <config_dir>/r3bl/               <- Machine-agnostic portable config
//! │                                   (e.g. `~/.config/r3bl/` on Linux;
//! │                                   safe to sync as-is across machines)
//! └── <crate_name>/                <- e.g., <config_dir>/r3bl/r3bl-cmdr/
//!     ├── edi.jsonc                <- Per-binary JSONC config
//!     └── giti.jsonc               <- Per-binary JSONC config
//!
//! <cache_dir>/r3bl/                <- Machine-scoped state/cache
//! │                                   (e.g. `~/.cache/r3bl/` on Linux;
//! │                                   in backups, namespace by host)
//! ├── proxy_machine_id             <- Machine-wide analytics proxy ID
//! └── <crate_name>/                <- e.g., <cache_dir>/r3bl/r3bl-cmdr/
//!     └── self_upgrade_last_check  <- Per-crate 0-byte `mtime` cooldown file
//! ```
//!
//! # Architectural Boundary: `<config_dir>/r3bl` vs `<cache_dir>/r3bl`
//!
//! 1. **Portable User Configuration (`<config_dir>/r3bl/<crate_name>/*` via
//!    [`dirs::config_dir`])**:
//!    - Scoped by crate/product name (`<crate_name>`, such as `"r3bl-cmdr"`), containing
//!      per-binary `.jsonc` configuration files (such as `edi.jsonc` or `giti.jsonc`).
//!    - Completely machine-agnostic: safe to copy, synchronize, or commit into dotfiles
//!      repositories **as-is** across multiple machines without colliding machine
//!      identity or runtime state.
//!
//! 2. **Machine-Scoped Runtime State & Cache (`<cache_dir>/r3bl/...` via
//!    [`dirs::cache_dir`])**:
//!    - Stores files tied to a specific machine's identity or runtime state, including
//!      the top-level [`PROXY_MACHINE_ID_FILENAME`] (`proxy_machine_id`) and per-crate
//!      [`SELF_UPGRADE_LAST_CHECK_FILENAME`] (`<crate_name>/self_upgrade_last_check`).
//!    - Because these files are bound to an individual host, they must **not** be copied
//!      or synchronized flat as-is across multiple machines.
//!    - In backup and restore workflows, `<cache_dir>/r3bl/...` **can** be backed up and
//!      restored on the same machine, provided it is namespaced by the machine's hostname
//!      in the backup archive (for example,
//!      `<backup_root>/mymachine.local/.cache/r3bl/...` on Linux).
//!    - If a machine starts fresh without restoring `<cache_dir>/r3bl/`, every file under
//!      this directory safely regenerates on demand.

use dirs::{cache_dir, config_dir};
use std::path::{Path, PathBuf};

/// Top-level R3BL umbrella directory name (`"r3bl"`) used inside both
/// [`dirs::config_dir`] and [`dirs::cache_dir`].
pub const R3BL_TOP_LEVEL_DIR: &str = "r3bl";

/// Filename (`"proxy_machine_id"`) for the machine-wide analytics proxy ID stored at the
/// top level of [`try_get_r3bl_cache_dir`] (`<cache_dir>/r3bl/proxy_machine_id`).
pub const PROXY_MACHINE_ID_FILENAME: &str = "proxy_machine_id";

/// Filename (`"self_upgrade_last_check"`) for the per-crate 0-byte [`mtime`] cooldown
/// timestamp file stored inside [`try_get_crate_cache_dir`]
/// (`<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check`).
///
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
pub const SELF_UPGRADE_LAST_CHECK_FILENAME: &str = "self_upgrade_last_check";

/// File extension (`"jsonc"`) for per-binary configuration files stored inside
/// [`try_get_crate_config_dir`] (`<config_dir>/r3bl/<crate_name>/<bin_name>.jsonc`).
pub const CONFIG_FILE_EXT: &str = "jsonc";

/// Resolves the top-level R3BL configuration directory `<config_dir>/r3bl`.
///
/// Returns `None` if [`dirs::config_dir`] cannot determine the user's configuration
/// directory on the current platform.
#[must_use]
pub fn try_get_r3bl_config_dir() -> Option<PathBuf> {
    Some(config_dir()?.join(R3BL_TOP_LEVEL_DIR))
}

/// Resolves the configuration directory for a specific R3BL crate
/// `<config_dir>/r3bl/<crate_name>`.
///
/// Returns `None` if [`dirs::config_dir`] cannot determine the user's configuration
/// directory.
#[must_use]
pub fn try_get_crate_config_dir(crate_name: &str) -> Option<PathBuf> {
    Some(try_get_r3bl_config_dir()?.join(crate_name))
}

/// Resolves the JSONC configuration file path for a specific binary inside a crate
/// `<config_dir>/r3bl/<crate_name>/<bin_name>.jsonc`.
///
/// Returns `None` if [`dirs::config_dir`] cannot determine the user's configuration
/// directory.
#[must_use]
pub fn try_get_bin_config_file_path(crate_name: &str, bin_name: &str) -> Option<PathBuf> {
    Some(
        try_get_crate_config_dir(crate_name)?
            .join(format!("{bin_name}.{CONFIG_FILE_EXT}")),
    )
}

/// Resolves the top-level R3BL cache directory `<cache_dir>/r3bl`.
///
/// Returns `None` if [`dirs::cache_dir`] cannot determine the user's cache directory on
/// the current platform.
#[must_use]
pub fn try_get_r3bl_cache_dir() -> Option<PathBuf> {
    Some(cache_dir()?.join(R3BL_TOP_LEVEL_DIR))
}

/// Resolves the cache directory for a specific R3BL crate
/// `<cache_dir>/r3bl/<crate_name>`.
///
/// Returns `None` if [`dirs::cache_dir`] cannot determine the user's cache directory.
#[must_use]
pub fn try_get_crate_cache_dir(crate_name: &str) -> Option<PathBuf> {
    Some(try_get_r3bl_cache_dir()?.join(crate_name))
}

/// Resolves the machine-wide analytics proxy ID file path
/// `<cache_dir>/r3bl/proxy_machine_id`.
///
/// Lives directly under the top-level [`R3BL_TOP_LEVEL_DIR`] cache folder because the
/// proxy machine ID represents the host machine rather than an individual R3BL product.
///
/// Returns `None` if [`dirs::cache_dir`] cannot determine the user's cache directory.
#[must_use]
pub fn try_get_proxy_machine_id_file_path() -> Option<PathBuf> {
    Some(try_get_r3bl_cache_dir()?.join(PROXY_MACHINE_ID_FILENAME))
}

/// Resolves the 0-byte [`mtime`] cooldown timestamp file path for a specific crate
/// `<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check`.
///
/// Returns `None` if [`dirs::cache_dir`] cannot determine the user's cache directory.
///
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
#[must_use]
pub fn try_get_self_upgrade_last_check_file_path(crate_name: &str) -> Option<PathBuf> {
    Some(resolve_self_upgrade_last_check_in_base_dir(
        &cache_dir()?,
        crate_name,
    ))
}

/// Composes `<base_dir>/r3bl/<crate_name>/self_upgrade_last_check` inside an arbitrary
/// `base_dir` (such as [`dirs::cache_dir`] in production or a temporary directory in
/// isolated unit tests).
#[must_use]
pub fn resolve_self_upgrade_last_check_in_base_dir(
    base_dir: &Path,
    crate_name: &str,
) -> PathBuf {
    base_dir
        .join(R3BL_TOP_LEVEL_DIR)
        .join(crate_name)
        .join(SELF_UPGRADE_LAST_CHECK_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_path_hierarchy() {
        if let Some(r3bl_config) = try_get_r3bl_config_dir() {
            assert!(r3bl_config.ends_with(PathBuf::from("r3bl")));
        }

        if let Some(crate_config) = try_get_crate_config_dir("r3bl-cmdr") {
            let expected = PathBuf::from("r3bl").join("r3bl-cmdr");
            assert!(crate_config.ends_with(&expected));
        }

        if let Some(bin_config) = try_get_bin_config_file_path("r3bl-cmdr", "edi") {
            let expected = PathBuf::from("r3bl").join("r3bl-cmdr").join("edi.jsonc");
            assert!(bin_config.ends_with(&expected));
        }
    }

    #[test]
    fn test_cache_dir_path_hierarchy() {
        if let Some(r3bl_cache) = try_get_r3bl_cache_dir() {
            assert!(r3bl_cache.ends_with(PathBuf::from("r3bl")));
        }

        if let Some(proxy_id_path) = try_get_proxy_machine_id_file_path() {
            let expected = PathBuf::from("r3bl").join("proxy_machine_id");
            assert!(proxy_id_path.ends_with(&expected));
        }

        if let Some(crate_cache) = try_get_crate_cache_dir("r3bl-cmdr") {
            let expected = PathBuf::from("r3bl").join("r3bl-cmdr");
            assert!(crate_cache.ends_with(&expected));
        }

        if let Some(cooldown_path) =
            try_get_self_upgrade_last_check_file_path("r3bl-cmdr")
        {
            let expected = PathBuf::from("r3bl")
                .join("r3bl-cmdr")
                .join("self_upgrade_last_check");
            assert!(cooldown_path.ends_with(&expected));
        }
    }
}

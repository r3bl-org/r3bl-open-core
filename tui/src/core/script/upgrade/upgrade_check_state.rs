// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Status representation and process-wide singleton cache for background version checks.
//!
//! # Background Version Check & Cache Lifecycle
//!
//! To keep application startup instantaneous (0 ms user-perceived latency) while
//! eliminating redundant network round-trips at shutdown, the upgrade engine coordinates
//! background check results through the [`UpgradeCheckCache`] singleton:
//!
//! ```text
//! 1. Binary Startup (0 ms non-blocking):
//!    ├── start_background_version_check()      ──► Spawns Tokio task (for async binaries)
//!    └── start_background_version_check_sync() ──► Spawns dedicated OS thread (for sync binaries)
//!    Main thread immediately proceeds with user CLI/TUI logic
//!
//! 2. Concurrent Manifest Query:
//!    check_version_against_manifest() ───► HTTP GET versions.jsonc
//!    ├── Evaluates SemVer (remote > local)
//!    └── Atomically updates UpgradeCheckCache:
//!        ├── UpgradeAvailable(metadata) (if strictly newer)
//!        ├── UpToDate(metadata)         (if equal or ahead)
//!        └── CheckFailed                (if network or parse error)
//!
//! 3. Binary Exit (0 ms in-memory read):
//!    handle_upgrade_at_exit()
//!    ├── UpgradeCheckCache::get()
//!    └── Matches UpgradeCheckStatus::UpgradeAvailable(metadata) (no 2nd HTTP request!)
//! ```
//!
//! [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
//! [`start_background_version_check_sync`]: crate::core::script::upgrade::start_background_version_check_sync
//! [`start_background_version_check`]: crate::core::script::upgrade::start_background_version_check
//! [`UpgradeCheckCache`]: crate::core::script::upgrade::UpgradeCheckCache
//! [`UpgradeCheckStatus`]: crate::core::script::upgrade::UpgradeCheckStatus

use super::CrateVersionMetadata;
use crate::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex,
            scoped_mutex};
use std::sync::LazyLock;

// -------------------------------------------------------------------------------------------------
// Static Upgrade Check Cache
// -------------------------------------------------------------------------------------------------

/// Thread-safe process-wide singleton cache storing the [`UpgradeCheckStatus`] from
/// [`R3BL_VERSIONS_JSON_URL`].
///
/// # Singleton Pattern
///
/// Operates as a process-wide in-memory singleton backed by a static [`ScopedMutex`].
/// Callers interact exclusively through associated functions ([`UpgradeCheckCache::get`],
/// [`UpgradeCheckCache::set`], [`UpgradeCheckCache::is_upgrade_available`], and
/// [`UpgradeCheckCache::try_get_crate_metadata`]) without managing instances directly.
///
/// # Purpose
///
/// Bridges the gap between startup and shutdown without making redundant HTTP requests.
/// When the background check completes, [`check_version_against_manifest`] stores the
/// status and parsed manifest metadata here via [`UpgradeCheckCache::set`].
///
/// At application exit, [`handle_upgrade_at_exit`] reads the status via
/// [`UpgradeCheckCache::get`] in 0 ms without needing to re-fetch or re-parse
/// [`R3BL_VERSIONS_JSON_URL`].
///
/// # Deadlock Prevention Policy: [`PanicOnSpecificLockNesting`] ([`SPECIFIC`])
///
/// This cache is strictly a leaf lock: its `read` and `write` closures only perform
/// memory copies/clones and never acquire other locks. We use [`SPECIFIC`] rather than
/// `ANY` because caller threads may already hold component-level locks (such as
/// [`OutputDevice`] or [`Readline`]) when querying the upgrade status during UI
/// drawing or application shutdown.
///
/// [`check_version_against_manifest`]: crate::core::script::upgrade::check_engine::check_version_against_manifest
/// [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
/// [`OutputDevice`]: crate::OutputDevice
/// [`PanicOnSpecificLockNesting`]: crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
/// [`Readline`]: crate::Readline
/// [`ScopedMutex`]: crate::ScopedMutex
/// [`SPECIFIC`]: crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`UpgradeCheckCache::get`]: crate::core::script::upgrade::UpgradeCheckCache::get
/// [`UpgradeCheckCache::is_upgrade_available`]: crate::core::script::upgrade::UpgradeCheckCache::is_upgrade_available
/// [`UpgradeCheckCache::set`]: crate::core::script::upgrade::UpgradeCheckCache::set
/// [`UpgradeCheckCache::try_get_crate_metadata`]: crate::core::script::upgrade::UpgradeCheckCache::try_get_crate_metadata
/// [`UpgradeCheckStatus`]: crate::core::script::upgrade::UpgradeCheckStatus
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpgradeCheckCache {
    pub status: UpgradeCheckStatus,
}

mod impl_upgrade_check_cache {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    static SINGLETON: LazyLock<
        ScopedMutex<UpgradeCheckCache, { PanicOnSpecificLockNesting }>,
    > = LazyLock::new(|| scoped_mutex!(SPECIFIC, UpgradeCheckCache::default()));

    impl UpgradeCheckCache {
        /// Returns the current [`UpgradeCheckStatus`] snapshot from the cache.
        ///
        /// [`UpgradeCheckStatus`]: crate::core::script::upgrade::UpgradeCheckStatus
        #[must_use]
        pub fn get() -> UpgradeCheckStatus {
            SINGLETON.read(|cache| cache.status.clone())
        }

        /// Sets the current [`UpgradeCheckStatus`] in the cache.
        ///
        /// [`UpgradeCheckStatus`]: crate::core::script::upgrade::UpgradeCheckStatus
        pub fn set(status: UpgradeCheckStatus) {
            SINGLETON.write(|cache| {
                cache.status = status;
            });
        }

        /// Returns `true` if a strictly newer version was found during the version check.
        #[must_use]
        pub fn is_upgrade_available() -> bool {
            matches!(Self::get(), UpgradeCheckStatus::UpgradeAvailable(_))
        }

        /// Returns a clone of the cached [`CrateVersionMetadata`] if the background
        /// version check has completed successfully (either
        /// [`UpgradeCheckStatus::UpToDate`] or
        /// [`UpgradeCheckStatus::UpgradeAvailable`]).
        ///
        /// # Consumers
        ///
        /// This function is called by:
        /// - [`handle_upgrade_at_exit`]: to display version notices or execute an
        ///   automatic upgrade at shutdown without making a second network call.
        /// - [`resolve_metadata_for_upgrade`]: to reuse existing metadata before falling
        ///   back to a direct network query.
        ///
        /// Returns `None` if the background check is still in flight, has failed, or if
        /// update checks were disabled via [`UpdateBehavior::Disable`].
        ///
        /// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
        /// [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
        /// [`resolve_metadata_for_upgrade`]: crate::core::script::upgrade::metadata_resolver::resolve_metadata_for_upgrade
        /// [`UpdateBehavior::Disable`]: crate::core::script::upgrade::UpdateBehavior::Disable
        /// [`UpgradeCheckStatus::UpgradeAvailable`]: crate::core::script::upgrade::UpgradeCheckStatus::UpgradeAvailable
        /// [`UpgradeCheckStatus::UpToDate`]: crate::core::script::upgrade::UpgradeCheckStatus::UpToDate
        #[must_use]
        pub fn try_get_crate_metadata() -> Option<CrateVersionMetadata> {
            match Self::get() {
                UpgradeCheckStatus::UpToDate(meta)
                | UpgradeCheckStatus::UpgradeAvailable(meta) => Some(meta),
                UpgradeCheckStatus::NotChecked
                | UpgradeCheckStatus::Checking
                | UpgradeCheckStatus::CheckFailed => None,
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Upgrade Check Status Enum
// -------------------------------------------------------------------------------------------------

/// Status representation of the background version check.
///
/// Stores the lifecycle phase and any resolved [`CrateVersionMetadata`].
///
/// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpgradeCheckStatus {
    /// Background version check has not yet started.
    #[default]
    NotChecked,

    /// Background version check is currently in flight.
    Checking,

    /// Local binary version is greater than or equal to the remote
    /// [`R3BL_VERSIONS_JSON_URL`] version. Carries the remote
    /// [`CrateVersionMetadata`].
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    UpToDate(CrateVersionMetadata),

    /// Remote [`R3BL_VERSIONS_JSON_URL`] version is strictly greater than local
    /// binary version. Carries the remote [`CrateVersionMetadata`].
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    UpgradeAvailable(CrateVersionMetadata),

    /// Network or parsing error occurred while querying [`R3BL_VERSIONS_JSON_URL`].
    ///
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    CheckFailed,
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{super::{CrateDistribution, CrateVersion},
                *};
    use crate::assert_eq2;
    use chrono::Utc;
    use serial_test::serial;

    fn make_test_metadata(version_str: &str) -> CrateVersionMetadata {
        CrateVersionMetadata {
            version: CrateVersion::try_from(version_str).unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        }
    }

    #[test]
    #[serial]
    fn test_upgrade_check_state_transitions_and_accessors() {
        // Initial state
        UpgradeCheckCache::set(UpgradeCheckStatus::NotChecked);
        assert_eq2!(UpgradeCheckCache::get(), UpgradeCheckStatus::NotChecked);
        assert!(!UpgradeCheckCache::is_upgrade_available());
        assert!(UpgradeCheckCache::try_get_crate_metadata().is_none());

        // In-flight
        UpgradeCheckCache::set(UpgradeCheckStatus::Checking);
        assert_eq2!(UpgradeCheckCache::get(), UpgradeCheckStatus::Checking);
        assert!(!UpgradeCheckCache::is_upgrade_available());
        assert!(UpgradeCheckCache::try_get_crate_metadata().is_none());

        // Up to date
        let meta_up_to_date = make_test_metadata("0.1.0");
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta_up_to_date.clone()));
        assert_eq2!(
            UpgradeCheckCache::get(),
            UpgradeCheckStatus::UpToDate(meta_up_to_date.clone())
        );
        assert!(!UpgradeCheckCache::is_upgrade_available());
        assert_eq2!(
            UpgradeCheckCache::try_get_crate_metadata(),
            Some(meta_up_to_date.clone())
        );

        // Upgrade available
        let meta_upgrade = make_test_metadata("0.2.0");
        UpgradeCheckCache::set(UpgradeCheckStatus::UpgradeAvailable(
            meta_upgrade.clone(),
        ));
        assert_eq2!(
            UpgradeCheckCache::get(),
            UpgradeCheckStatus::UpgradeAvailable(meta_upgrade.clone())
        );
        assert!(UpgradeCheckCache::is_upgrade_available());
        assert_eq2!(
            UpgradeCheckCache::try_get_crate_metadata(),
            Some(meta_upgrade.clone())
        );

        // Failed check
        UpgradeCheckCache::set(UpgradeCheckStatus::CheckFailed);
        assert_eq2!(UpgradeCheckCache::get(), UpgradeCheckStatus::CheckFailed);
        assert!(!UpgradeCheckCache::is_upgrade_available());
        assert!(UpgradeCheckCache::try_get_crate_metadata().is_none());
    }
}

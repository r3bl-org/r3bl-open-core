// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::manifest_timeouts;
use crate::{CrateName, DEBUG_SELF_UPGRADE, try_get_self_upgrade_last_check_file_path};
use std::{fs::{self, File},
          path::PathBuf,
          time::{Duration, SystemTime}};

/// Cooldown [`mtime`] timestamp file policy governing version-check caching.
///
/// Controls where (or whether) the cooldown timestamp file is stored on disk, read via
/// [`Self::check_status`], and updated via [`Self::touch`]. This type is part of the
/// [Execution Plane] and is called by the [Control Plane]:
/// - [`Self::check_status`] is called by [`UpgradeJourney::start`] to gate [`NotStarted`]
///   into [`CooldownSkipped`] vs. [`Checking`].
/// - [`Self::touch`] is called only when [`Checking`] resolves to [`UpToDate`] or when
///   [`Upgrading`] resolves to [`Completed`] with [`UpgradeResolution::Upgraded`].
///
/// Production binaries use [`Self::DefaultCacheDir`], whereas unit tests pass
/// [`Self::Disabled`] or [`Self::CustomTimestampFilePath`] so tests remain completely
/// hermetic and never touch the filesystem cache directory on the machine in which
/// the tests run.
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`Completed`]: crate::self_upgrade::Completed
/// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CooldownPolicy {
    /// Production policy storing a 0-byte [`mtime`] timestamp file at
    /// `<cache_dir>/`[`r3bl`][1]`/<crate_name>/`[`self_upgrade_last_check`][2] with the
    /// specified cooldown [`Duration`].
    ///
    /// [1]: crate::common::r3bl_file_paths::R3BL_TOP_LEVEL_DIR
    /// [2]: crate::common::r3bl_file_paths::SELF_UPGRADE_LAST_CHECK_FILENAME
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    DefaultCacheDir(Duration),

    /// Custom timestamp file path and cooldown [`Duration`] for isolated filesystem unit
    /// tests.
    #[cfg(any(test, doc))]
    CustomTimestampFilePath(PathBuf, Duration),

    /// Disables disk cooldown reads and writes for deterministic unit tests.
    #[cfg(any(test, doc))]
    Disabled,
}

impl Default for CooldownPolicy {
    fn default() -> Self {
        Self::DefaultCacheDir(manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY)
    }
}

impl CooldownPolicy {
    /// Creates a [`CooldownPolicy::CustomTimestampFilePath`] with the default `1h`
    /// cooldown.
    #[cfg(any(test, doc))]
    #[must_use]
    pub fn new_custom_path(arg_path: impl Into<PathBuf>) -> Self {
        Self::CustomTimestampFilePath(
            arg_path.into(),
            manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY,
        )
    }

    /// Creates a [`CooldownPolicy::CustomTimestampFilePath`] with a custom cooldown
    /// duration.
    #[cfg(any(test, doc))]
    #[must_use]
    pub fn new_custom_path_with_duration(
        arg_path: impl Into<PathBuf>,
        duration: Duration,
    ) -> Self {
        Self::CustomTimestampFilePath(arg_path.into(), duration)
    }

    /// Returns the cooldown duration before a fresh [`mtime`] timestamp file expires, or
    /// [`None`] if cooldown is disabled.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[must_use]
    pub fn try_get_cooldown_duration(&self) -> Option<Duration> {
        match self {
            Self::DefaultCacheDir(duration) => Some(*duration),
            #[cfg(any(test, doc))]
            Self::CustomTimestampFilePath(_, duration) => Some(*duration),
            #[cfg(any(test, doc))]
            Self::Disabled => None,
        }
    }

    /// Checks whether `crate_name` has a fresh [`mtime`] cooldown timestamp file within
    /// its configured duration.
    ///
    /// Performs at most a single `stat` syscall (`fs::metadata`) and reads zero file
    /// bytes.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[must_use]
    pub fn check_status(&self, crate_name: &CrateName) -> CooldownStatus {
        let Some(duration) = self.try_get_cooldown_duration() else {
            return CooldownStatus::ExpiredOrMissing;
        };

        let Some(timestamp_file_path) = self.resolve_file_path(crate_name) else {
            return CooldownStatus::ExpiredOrMissing;
        };

        let Ok(metadata) = fs::metadata(&timestamp_file_path) else {
            return CooldownStatus::ExpiredOrMissing;
        };

        if !metadata.is_file() {
            return CooldownStatus::ExpiredOrMissing;
        }

        let Ok(modified) = metadata.modified() else {
            return CooldownStatus::ExpiredOrMissing;
        };

        // If `modified > now` (clock skew), `duration_since` returns `Err`, which safely
        // treats the timestamp file as expired rather than permanently suppressing
        // checks.
        let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
            return CooldownStatus::ExpiredOrMissing;
        };

        if elapsed < duration {
            CooldownStatus::Active
        } else {
            CooldownStatus::ExpiredOrMissing
        }
    }

    /// Touches (creates or updates [`mtime`] on) the 0-byte cooldown timestamp file for
    /// `crate_name`.
    ///
    /// Must be called **only** when a check confirms the binary is [`UpToDate`] or after
    /// an upgrade succeeds ([`UpgradeResolution::Upgraded`]). Never call on network/parse
    /// failures or unapplied available upgrades so subsequent invocations retry cleanly.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    pub fn touch(&self, crate_name: &CrateName) {
        let Some(timestamp_file_path) = self.resolve_file_path(crate_name) else {
            return;
        };

        if let Some(parent_dir) = timestamp_file_path.parent()
            && let Err(err) = fs::create_dir_all(parent_dir)
        {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Could not create upgrade-check cache directory",
                    path = ?parent_dir,
                    error = ?err
                );
            });
            return;
        }

        match File::create(&timestamp_file_path) {
            Ok(file) => {
                // Explicitly set `mtime` to `SystemTime::now()` so re-truncating an
                // existing 0-byte timestamp file reliably refreshes `mtime` across all
                // filesystems.
                if let Err(err) = file.set_modified(SystemTime::now()) {
                    DEBUG_SELF_UPGRADE.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "Could not update mtime on upgrade-check cooldown timestamp file",
                            path = ?timestamp_file_path,
                            error = ?err
                        );
                    });
                }
            }
            Err(err) => {
                DEBUG_SELF_UPGRADE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Could not touch upgrade-check cooldown timestamp file",
                        path = ?timestamp_file_path,
                        error = ?err
                    );
                });
            }
        }
    }

    /// Resolves the filesystem path of the 0-byte [`mtime`] timestamp file for
    /// `crate_name` according to the policy variant.
    ///
    /// Returns `None` when the policy is [`CooldownPolicy::Disabled`] or when
    /// [`dirs::cache_dir`] cannot determine a platform cache directory.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[must_use]
    pub fn resolve_file_path(&self, crate_name: &CrateName) -> Option<PathBuf> {
        match self {
            #[cfg(any(test, doc))]
            Self::Disabled => None,
            #[cfg(any(test, doc))]
            Self::CustomTimestampFilePath(path, _) => Some(path.clone()),
            Self::DefaultCacheDir(_) => {
                try_get_self_upgrade_last_check_file_path(crate_name)
            }
        }
    }
}

/// Result of checking whether a crate's [`mtime`] cooldown timestamp file is still within
/// [`DEFERRED_CHECK_COOLDOWN_DELAY`][1].
///
/// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooldownStatus {
    /// Timestamp file exists, is a regular file, and its [`mtime`] age is strictly less
    /// than [`DEFERRED_CHECK_COOLDOWN_DELAY`][1]; skip spawning the dedicated check
    /// thread.
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    Active,

    /// Cooldown is disabled, timestamp file does not exist, not a regular file, [`mtime`]
    /// could not be read, system clock skewed backwards, or [`mtime`] age has reached or
    /// exceeded [`DEFERRED_CHECK_COOLDOWN_DELAY`][1].
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    ExpiredOrMissing,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve_self_upgrade_last_check_in_base_dir;
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use std::time::Duration;

    #[test]
    fn test_cooldown_disabled_never_touches_disk_and_is_expired() {
        let crate_name = "r3bl-cmdr".into();
        let policy = CooldownPolicy::Disabled;
        assert_eq2!(policy.try_get_cooldown_duration(), None);
        assert_eq2!(policy.resolve_file_path(&crate_name), None);
        policy.touch(&crate_name);
        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );
    }

    #[test]
    fn test_default_cache_dir_path_structure() {
        let crate_name = "r3bl-cmdr".into();
        let policy = CooldownPolicy::default();
        if let Some(path) = policy.resolve_file_path(&crate_name) {
            let expected_suffix = PathBuf::from("r3bl")
                .join("r3bl-cmdr")
                .join("self_upgrade_last_check");
            assert!(path.ends_with(&expected_suffix));
        }
    }

    #[test]
    fn test_custom_timestamp_file_lifecycle_missing_active_expired_and_clock_skew() {
        let crate_name = "r3bl-cmdr".into();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path =
            resolve_self_upgrade_last_check_in_base_dir(&temp_dir, &crate_name);
        let policy = CooldownPolicy::new_custom_path(timestamp_file_path.clone());
        let window = Duration::from_secs(3600);

        // 1. Missing timestamp file -> ExpiredOrMissing.
        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );

        // 2. Touch timestamp file -> creates parent dir + 0-byte file -> Active within 1h
        //    window.
        policy.touch(&crate_name);
        assert!(timestamp_file_path.exists());
        assert_eq2!(fs::metadata(&timestamp_file_path).unwrap().len(), 0);
        assert_eq2!(policy.check_status(&crate_name), CooldownStatus::Active);

        // 3. Backdate real file `mtime` beyond 1h -> ExpiredOrMissing, then re-touch
        //    existing 0-byte timestamp file -> refreshes back to Active.
        let backdated = SystemTime::now() - (window + Duration::from_secs(10));
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(backdated)
            .unwrap();
        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );
        policy.touch(&crate_name);
        assert_eq2!(policy.check_status(&crate_name), CooldownStatus::Active);

        // 4. Future `mtime` (`modified > now` backward clock skew) -> ExpiredOrMissing.
        let future_mtime = SystemTime::now() + Duration::from_secs(60);
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(future_mtime)
            .unwrap();
        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );
    }

    #[test]
    fn test_custom_duration_natural_expiry_without_backdating() {
        let crate_name = "r3bl-cmdr".into();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path =
            resolve_self_upgrade_last_check_in_base_dir(&temp_dir, &crate_name);
        let short_duration = Duration::from_millis(20);
        let policy = CooldownPolicy::new_custom_path_with_duration(
            timestamp_file_path,
            short_duration,
        );
        assert_eq2!(policy.try_get_cooldown_duration(), Some(short_duration));

        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );

        policy.touch(&crate_name);
        assert_eq2!(policy.check_status(&crate_name), CooldownStatus::Active);

        // Sleep 25ms to exceed 20ms short duration naturally:
        std::thread::sleep(Duration::from_millis(25));
        assert_eq2!(
            policy.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );
    }

    #[test]
    fn test_record_successful_check_handles_io_create_failure_gracefully() {
        let crate_name = "r3bl-cmdr".into();
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. `File::create` fails when target path is an existing directory (temp_dir).
        let policy_dir_target = CooldownPolicy::new_custom_path(temp_dir.to_path_buf());
        policy_dir_target.touch(&crate_name);
        assert_eq2!(
            policy_dir_target.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );

        // 2. `fs::create_dir_all` fails when parent path is blocked by an existing file.
        let blocking_file = temp_dir.join("blocking_file");
        File::create(&blocking_file).unwrap();
        let policy_parent_err =
            CooldownPolicy::new_custom_path(blocking_file.join("timestamp"));
        policy_parent_err.touch(&crate_name);
        assert_eq2!(
            policy_parent_err.check_status(&crate_name),
            CooldownStatus::ExpiredOrMissing
        );
    }
}

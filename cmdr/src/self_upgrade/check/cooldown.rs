// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{CooldownPolicy, UpgradeConfig, check_cooldown_cache,
                   manifest_timeouts};
use crate::DEBUG_SELF_UPGRADE;
use dirs::cache_dir;
use std::{fs::{self, File},
          path::PathBuf,
          time::{Duration, SystemTime}};

/// Result of checking whether a crate's `mtime` cooldown timestamp file is still within
/// [`DEDICATED_THREAD_CHECK_COOLDOWN`].
///
/// [`DEDICATED_THREAD_CHECK_COOLDOWN`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooldownStatus {
    /// Timestamp file exists and its `mtime` age is strictly less than
    /// [`DEDICATED_THREAD_CHECK_COOLDOWN`]; skip spawning the dedicated check thread.
    ///
    /// [`DEDICATED_THREAD_CHECK_COOLDOWN`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
    Active,

    /// Cooldown is disabled, timestamp file does not exist, `mtime` could not be read,
    /// system clock skewed backwards, or `mtime` age has reached or exceeded
    /// [`DEDICATED_THREAD_CHECK_COOLDOWN`].
    ///
    /// [`DEDICATED_THREAD_CHECK_COOLDOWN`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
    ExpiredOrMissing,
}

/// Resolves the filesystem path of the 0-byte `mtime` timestamp file for `config`
/// according to [`UpgradeConfig::get_cooldown_policy`].
///
/// Returns `None` when the policy is [`CooldownPolicy::Disabled`] or when
/// [`dirs::cache_dir`] cannot determine a platform cache directory.
#[must_use]
pub fn resolve_cooldown_timestamp_file_path(config: &UpgradeConfig) -> Option<PathBuf> {
    match config.get_cooldown_policy() {
        CooldownPolicy::Disabled => None,
        CooldownPolicy::CustomTimestampFilePath(path) => Some(path.clone()),
        CooldownPolicy::DefaultCacheDir => {
            let base_dir = cache_dir()?;
            let file_name = format!(
                "{}.{}",
                config.get_crate_name(),
                check_cooldown_cache::LAST_CHECK_FILE_EXT
            );
            Some(
                base_dir
                    .join(check_cooldown_cache::CACHE_TOP_LEVEL_DIR)
                    .join(check_cooldown_cache::UPGRADE_CHECK_CACHE_SUBDIR)
                    .join(file_name),
            )
        }
    }
}

/// Checks whether `config` has a fresh `mtime` cooldown timestamp file within
/// [`DEDICATED_THREAD_CHECK_COOLDOWN`].
///
/// Performs at most a single `stat` syscall (`fs::metadata`) and reads zero file bytes.
///
/// [`DEDICATED_THREAD_CHECK_COOLDOWN`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
#[must_use]
pub fn check_cooldown_status(config: &UpgradeConfig) -> CooldownStatus {
    check_cooldown_status_at(
        config,
        SystemTime::now(),
        manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN,
    )
}

/// Evaluates cooldown freshness against an explicit `now` timestamp and `cooldown_window`
/// for deterministic testing without thread sleeps.
#[must_use]
pub(crate) fn check_cooldown_status_at(
    config: &UpgradeConfig,
    now: SystemTime,
    cooldown_window: Duration,
) -> CooldownStatus {
    let Some(timestamp_file_path) = resolve_cooldown_timestamp_file_path(config) else {
        return CooldownStatus::ExpiredOrMissing;
    };

    let Ok(metadata) = fs::metadata(&timestamp_file_path) else {
        return CooldownStatus::ExpiredOrMissing;
    };

    let Ok(modified) = metadata.modified() else {
        return CooldownStatus::ExpiredOrMissing;
    };

    // If `modified > now` (clock skew), `duration_since` returns `Err`, which safely
    // treats the timestamp file as expired rather than permanently suppressing checks.
    let Ok(elapsed) = now.duration_since(modified) else {
        return CooldownStatus::ExpiredOrMissing;
    };

    if elapsed < cooldown_window {
        CooldownStatus::Active
    } else {
        CooldownStatus::ExpiredOrMissing
    }
}

/// Touches (creates or updates `mtime` on) the 0-byte cooldown timestamp file for
/// `config`.
///
/// Must be called **only** when a check confirms the binary is [`UpToDate`] or after an
/// upgrade succeeds ([`UpgradeResolution::Upgraded`]). Never call on network/parse
/// failures or unapplied available upgrades so subsequent invocations retry cleanly.
///
/// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub fn record_successful_check(config: &UpgradeConfig) {
    let Some(timestamp_file_path) = resolve_cooldown_timestamp_file_path(config) else {
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
            // Explicitly set `mtime` to `SystemTime::now()` so re-truncating an existing
            // 0-byte timestamp file reliably refreshes `mtime` across all filesystems.
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

#[cfg(test)]
mod tests {
    use super::{super::super::{CrateIdentity, CrateVersion, UpgradePolicy,
                               UserFeedbackChannel},
                *};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};

    fn make_config_with_policy(policy: CooldownPolicy) -> UpgradeConfig {
        UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::default(),
            None,
            None,
            policy,
        )
    }

    #[test]
    fn test_cooldown_disabled_never_touches_disk_and_is_expired() {
        let config = make_config_with_policy(CooldownPolicy::Disabled);
        assert_eq2!(resolve_cooldown_timestamp_file_path(&config), None);
        record_successful_check(&config);
        assert_eq2!(
            check_cooldown_status(&config),
            CooldownStatus::ExpiredOrMissing
        );
    }

    #[test]
    fn test_default_cache_dir_path_structure() {
        let config = make_config_with_policy(CooldownPolicy::DefaultCacheDir);
        if let Some(path) = resolve_cooldown_timestamp_file_path(&config) {
            let expected_suffix = PathBuf::from("r3bl")
                .join("upgrade-check")
                .join("r3bl-cmdr.last_check");
            assert!(path.ends_with(&expected_suffix));
        }
    }

    #[test]
    fn test_custom_timestamp_file_lifecycle_missing_active_expired_and_clock_skew() {
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("nested").join("r3bl-cmdr.last_check");
        let config = make_config_with_policy(CooldownPolicy::CustomTimestampFilePath(
            timestamp_file_path.clone(),
        ));
        let window = Duration::from_secs(3600);

        // 1. Missing timestamp file -> ExpiredOrMissing.
        assert_eq2!(
            check_cooldown_status(&config),
            CooldownStatus::ExpiredOrMissing
        );

        // 2. Touch timestamp file -> creates parent dir + 0-byte file -> Active within 1h
        //    window.
        record_successful_check(&config);
        assert!(timestamp_file_path.exists());
        assert_eq2!(fs::metadata(&timestamp_file_path).unwrap().len(), 0);
        assert_eq2!(check_cooldown_status(&config), CooldownStatus::Active);

        let modified = fs::metadata(&timestamp_file_path)
            .unwrap()
            .modified()
            .unwrap();
        assert_eq2!(
            check_cooldown_status_at(
                &config,
                modified + Duration::from_secs(3599),
                window,
            ),
            CooldownStatus::Active
        );

        // 3. Exact boundary (`elapsed == window`) -> ExpiredOrMissing.
        assert_eq2!(
            check_cooldown_status_at(&config, modified + window, window),
            CooldownStatus::ExpiredOrMissing
        );

        // 4. Backdate real file `mtime` beyond 1h -> ExpiredOrMissing, then re-touch
        // existing 0-byte timestamp file -> refreshes back to Active.
        let backdated = SystemTime::now() - (window + Duration::from_secs(10));
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(backdated)
            .unwrap();
        assert_eq2!(
            check_cooldown_status(&config),
            CooldownStatus::ExpiredOrMissing
        );
        record_successful_check(&config);
        assert_eq2!(check_cooldown_status(&config), CooldownStatus::Active);

        // 5. Backward clock skew (`now < modified`) -> ExpiredOrMissing.
        assert_eq2!(
            check_cooldown_status_at(&config, SystemTime::UNIX_EPOCH, window),
            CooldownStatus::ExpiredOrMissing
        );
    }
}

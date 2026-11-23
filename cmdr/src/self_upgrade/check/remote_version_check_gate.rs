// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::manifest_timeouts;
use crate::{CrateName, r3bl_file_paths};
use std::{fs::{self, File},
          path::PathBuf,
          time::{Duration, SystemTime, SystemTimeError}};

/// A gate controlling whether deferred remote version upgrade checks are either:
/// 1. allowed to proceed: [`GateStatus::Open`].
/// 2. skipped/blocked: [`GateStatus::ClosedForCooldown`].
///
/// This gate controls a cooldown timestamp file. It:
/// 1. determines where this file is stored on disk,
/// 2. reads this file via [`Self::try_check_gate_status`], and
/// 3. creates or updates this file via [`Self::try_touch_timestamp_file`].
///
/// This type is part of the [Execution Plane] and is called by the [Control Plane]:
/// - [`Self::try_check_gate_status`] is called only for deferred upgrades (immediate
///   bypasses this).
/// - [`Self::try_touch_timestamp_file`] is called for both immediate and deferred
///   upgrades.
///
/// Production binaries construct this via [`Self::try_new`]. Tests use [`Self::Disabled`]
/// or [`Self::new_temp_dir`] so that tests remain completely hermetic and never touch the
/// user cache directory on the host machine where the tests run.
///
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteVersionCheckGate {
    /// Gate is enabled: stores the 0-byte [`mtime`] timestamp file
    /// [`SELF_UPGRADE_LAST_CHECK_FILE`][1] inside `dir` with the specified cooldown
    /// `duration`.
    ///
    /// [1]: crate::common::r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    Enabled {
        /// Directory containing [`SELF_UPGRADE_LAST_CHECK_FILE`][1].
        ///
        /// [1]: crate::common::r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE
        dir: PathBuf,

        /// Cooldown window during which [`Self::try_check_gate_status`] returns
        /// [`GateStatus::ClosedForCooldown`].
        duration: Duration,
    },

    /// Gate is disabled: performs no disk reads or writes and always reports
    /// [`GateStatus::Open`].
    ///
    /// Used in production when [`dirs::cache_dir`] is unavailable (for example, minimal
    /// containers) and in deterministic unit tests.
    Disabled,
}

impl RemoteVersionCheckGate {
    /// Creates an [`Self::Enabled`] gate using `<cache_dir>/`[`r3bl`][1]`/<crate_name>/`,
    /// or returns [`None`] if [`dirs::cache_dir`] is unavailable on the current platform.
    ///
    /// Defaults to [`DEFERRED_CHECK_COOLDOWN_DELAY`][2] (`1h`) when `maybe_duration` is
    /// [`None`].
    ///
    /// [1]: crate::common::r3bl_file_paths::constants::R3BL_ROOT_DIR
    /// [2]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    #[must_use]
    pub fn try_new(
        crate_name: &CrateName,
        maybe_duration: Option<Duration>,
    ) -> Option<Self> {
        let dir = r3bl_file_paths::cache::try_get_crate_cache_dir(crate_name)?;
        let duration =
            maybe_duration.unwrap_or(manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY);
        Some(Self::Enabled { dir, duration })
    }

    /// Touches (creates or updates [`mtime`] on) the 0-byte "cooldown timestamp" file so
    /// subsequent [`Self::try_check_gate_status`] calls return
    /// [`GateStatus::ClosedForCooldown`] for the configured duration.
    ///
    /// Must be called **only** when a check confirms the binary is [`UpToDate`] or after
    /// an upgrade succeeds ([`UpgradeResolution::Upgraded`]). Never call on network/parse
    /// failures or unapplied available upgrades so subsequent invocations retry cleanly.
    ///
    /// # Errors
    ///
    /// Returns [`GateTouchOutcomeError`] if creating the cache directory, creating the
    /// timestamp file, or updating its [`mtime`] fails. Always returns
    /// `Ok(`[`GateTouchOutcome::NotTouched`]`)` when [`Self::Disabled`].
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    pub fn try_touch_timestamp_file(
        &self,
    ) -> Result<GateTouchOutcome, GateTouchOutcomeError> {
        // Early return if disabled.
        let Self::Enabled { dir, .. } = self else {
            return Ok(GateTouchOutcome::NotTouched);
        };

        // 1. Create `dir` lazily on write rather than at construction time to keep
        //    construction side-effect-free (for `--no-upgrade` scenario and unit tests).
        // 2. Also, this makes the code resilient if another process deletes the cache
        //    directory before write (TOCTOU).
        fs::create_dir_all(dir).map_err(|src_err| {
            GateTouchOutcomeError::CreateDirFailed {
                dir_path: dir.clone(),
                src_err,
            }
        })?;

        let timestamp_file_path =
            dir.join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);
        let timestamp_file = File::create(&timestamp_file_path).map_err(|src_err| {
            GateTouchOutcomeError::CreateFileFailed {
                file_path: timestamp_file_path.clone(),
                src_err,
            }
        })?;

        // Explicitly set `mtime` to `SystemTime::now()` so re-truncating an existing
        // 0-byte timestamp file reliably refreshes `mtime` across all filesystems.
        timestamp_file
            .set_modified(SystemTime::now())
            .map_err(|src_err| GateTouchOutcomeError::TouchFileFailed {
                file_path: timestamp_file_path,
                src_err,
            })?;

        Ok(GateTouchOutcome::Touched)
    }

    /// Checks whether the gate is [`GateStatus::ClosedForCooldown`] or
    /// [`GateStatus::Open`] by comparing the elapsed time between the current time
    /// ([`SystemTime::now()`]) and the cooldown timestamp file's [`mtime`] against the
    /// configured [`Duration`].
    ///
    /// - Returns `Ok(`[`GateStatus::ClosedForCooldown`]`)` when the timestamp file
    ///   exists, is a regular file, and `now - mtime < duration`.
    /// - Returns `Ok(`[`GateStatus::Open`]`)` when the gate is [`Self::Disabled`] or the
    ///   timestamp file exists and `now - mtime >= duration`.
    ///
    /// Performs at most a single [`stat`] syscall ([`fs::metadata`]) and reads zero file
    /// bytes.
    ///
    /// # Errors
    ///
    /// Returns [`GateStatusError`] if the timestamp file is missing, unreadable, not a
    /// regular file, or has a future [`mtime`]. Callers decide how to map errors to
    /// open/closed behavior (see [`UpgradeJourney::check_gate_status_fail_open`][1]).
    /// Always returns `Ok(`[`GateStatus::Open`]`)` when [`Self::Disabled`].
    ///
    /// [1]: crate::self_upgrade::UpgradeJourney::check_gate_status_fail_open
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    /// [`stat`]: https://en.wikipedia.org/wiki/Stat_(Unix)
    pub fn try_check_gate_status(&self) -> Result<GateStatus, GateStatusError> {
        // Early return if disabled.
        let Self::Enabled { dir, duration } = self else {
            return Ok(GateStatus::Open);
        };

        // Construct the path to the timestamp file.
        let timestamp_file_path =
            dir.join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);

        // Query metadata for the timestamp file.
        let metadata = fs::metadata(&timestamp_file_path).map_err(|src_err| {
            GateStatusError::MetadataReadFailed {
                file_path: timestamp_file_path.clone(),
                src_err,
            }
        })?;

        // The timestamp file must be a regular file; not a directory, FIFO, socket, etc.
        if !metadata.is_file() {
            return Err(GateStatusError::NotARegularFile {
                file_path: timestamp_file_path,
            });
        }

        // Read the file's modification timestamp (`mtime`).
        let modified_time = metadata.modified().map_err(|src_err| {
            GateStatusError::ModifiedTimeReadFailed {
                file_path: timestamp_file_path.clone(),
                src_err,
            }
        })?;

        // Compute elapsed time since `mtime`. If `modified_time > now` (backward clock
        // skew), `duration_since` returns `Err`.
        let elapsed =
            SystemTime::now()
                .duration_since(modified_time)
                .map_err(|src_err| GateStatusError::ClockSkew {
                    file_path: timestamp_file_path,
                    src_err,
                })?;

        // Finally, check if the elapsed time is within the cooldown duration.
        if elapsed < *duration {
            Ok(GateStatus::ClosedForCooldown)
        } else {
            Ok(GateStatus::Open)
        }
    }
}

#[cfg(any(test, doc))]
use r3bl_tui::TempDirChildPath;

/// Test fixtures.
#[cfg(any(test, doc))]
impl RemoteVersionCheckGate {
    /// Creates an isolated test gate inside `dir` with an optional custom cooldown
    /// duration (defaults to [`DEFERRED_CHECK_COOLDOWN_DELAY`][1], `1h`).
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    #[must_use]
    pub fn new_temp_dir(dir: TempDirChildPath, maybe_duration: Option<Duration>) -> Self {
        let duration =
            maybe_duration.unwrap_or(manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY);
        Self::Enabled {
            dir: dir.as_path_buf(),
            duration,
        }
    }

    /// Creates the cooldown timestamp file (if missing) and backdates its [`mtime`] past
    /// the configured cooldown window so [`Self::try_check_gate_status`] returns
    /// [`GateStatus::Open`].
    ///
    /// # Panics
    ///
    /// Panics if:
    /// 1. `self` is [`Self::Disabled`] or its `dir` is outside [`std::env::temp_dir`] (to
    ///    keep tests hermetic and never mutate the real cache directory on the host).
    /// 2. Creating/backdating the file fails.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    pub fn backdate_timestamp_file_for_testing(&self) {
        let Self::Enabled { dir, duration } = self else {
            panic!(
                "backdate_timestamp_file_for_testing requires RemoteVersionCheckGate::Enabled"
            );
        };
        assert!(
            dir.starts_with(std::env::temp_dir()),
            "backdate_timestamp_file_for_testing requires dir inside temp_dir: {}",
            dir.display()
        );

        // Create the timestamp file (if missing) or update its mtime.
        self.try_touch_timestamp_file()
            .expect("creating or updating cooldown timestamp file must succeed");

        // Backdate the timestamp of the existing file.
        let timestamp_file_path =
            dir.join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);
        let backdated_mtime = SystemTime::now() - (*duration + Duration::from_secs(10));
        File::options()
            .write(true)
            .open(&timestamp_file_path)
            .expect("cooldown timestamp file must exist after try_touch_timestamp_file()")
            .set_modified(backdated_mtime)
            .expect("backdating cooldown timestamp file mtime must succeed");
    }
}

/// Result of checking whether a crate is currently in cooldown based on its [`mtime`]
/// timestamp file and the gate's configured cooldown duration (defaulting to
/// [`DEFERRED_CHECK_COOLDOWN_DELAY`][1]).
///
/// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    /// Timestamp file exists, is a regular file, and its [`mtime`] age is strictly less
    /// than the gate's cooldown duration (default
    /// [`DEFERRED_CHECK_COOLDOWN_DELAY`][1]); skip spawning the dedicated check thread.
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    ClosedForCooldown,

    /// Gate is disabled, or the timestamp file exists and its [`mtime`] age has reached
    /// or exceeded the gate's cooldown duration (default
    /// [`DEFERRED_CHECK_COOLDOWN_DELAY`][1]).
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    Open,
}

/// Failure returned by [`RemoteVersionCheckGate::try_check_gate_status`] when the
/// cooldown timestamp file does not exist or cannot be evaluated cleanly.
///
/// Callers in the [Control Plane] decide how to map errors to open/closed behavior (see
/// [`UpgradeJourney::check_gate_status_fail_open`][1]).
///
/// [1]: crate::self_upgrade::UpgradeJourney::check_gate_status_fail_open
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GateStatusError {
    /// Failed to query filesystem metadata for the cooldown timestamp file (including
    /// [`std::io::ErrorKind::NotFound`] when the file does not exist yet).
    #[error(
        "Could not read metadata for upgrade-check cooldown timestamp file: {}",
        file_path.display()
    )]
    MetadataReadFailed {
        /// Timestamp file path passed to [`fs::metadata`].
        file_path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Path exists at the timestamp file location, but is not a regular file (for
    /// example, a directory, FIFO, or socket).
    #[error(
        "Upgrade-check cooldown timestamp path is not a regular file: {}",
        file_path.display()
    )]
    NotARegularFile {
        /// Path that was expected to be a regular file.
        file_path: PathBuf,
    },

    /// Failed to read [`mtime`] from the timestamp file's metadata.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[error(
        "Could not read mtime for upgrade-check cooldown timestamp file: {}",
        file_path.display()
    )]
    ModifiedTimeReadFailed {
        /// Timestamp file path whose metadata lacked a readable `mtime`.
        file_path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Timestamp file's [`mtime`] is in the future relative to [`SystemTime::now()`]
    /// (backward system clock skew).
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[error(
        "Upgrade-check cooldown timestamp file mtime is in the future (clock skew): {}",
        file_path.display()
    )]
    ClockSkew {
        /// Timestamp file path whose `mtime` was ahead of `SystemTime::now()`.
        file_path: PathBuf,

        /// Underlying [`SystemTimeError`] describing how far ahead `mtime` is.
        #[source]
        src_err: SystemTimeError,
    },
}

/// Outcome returned by [`RemoteVersionCheckGate::try_touch_timestamp_file`] on
/// success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateTouchOutcome {
    /// Cooldown timestamp file was created or updated; the gate is now in cooldown.
    Touched,

    /// Gate is [`RemoteVersionCheckGate::Disabled`]; no disk I/O was performed.
    NotTouched,
}

/// Failure returned by [`RemoteVersionCheckGate::try_touch_timestamp_file`] when
/// creating the cache directory, creating the timestamp file, or refreshing its [`mtime`]
/// fails.
///
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GateTouchOutcomeError {
    /// Failed to create the cache directory containing the cooldown timestamp file.
    #[error("Could not create upgrade-check cache directory: {}", dir_path.display())]
    CreateDirFailed {
        /// Cache directory path passed to [`fs::create_dir_all`].
        dir_path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Failed to create or open the 0-byte cooldown timestamp file.
    #[error(
        "Could not create upgrade-check cooldown timestamp file: {}",
        file_path.display()
    )]
    CreateFileFailed {
        /// Timestamp file path passed to [`File::create`].
        file_path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Failed to update [`mtime`] on the cooldown timestamp file.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[error(
        "Could not update mtime on upgrade-check cooldown timestamp file: {}",
        file_path.display()
    )]
    TouchFileFailed {
        /// Timestamp file path passed to [`File::set_modified`].
        file_path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use std::{io::ErrorKind, time::Duration};

    #[test]
    fn test_cooldown_disabled_is_always_not_in_cooldown() {
        let gate = RemoteVersionCheckGate::Disabled;
        assert_eq2!(
            gate.try_touch_timestamp_file().unwrap(),
            GateTouchOutcome::NotTouched
        );
        assert_eq2!(gate.try_check_gate_status().unwrap(), GateStatus::Open);
    }

    #[test]
    fn test_host_cache_dir_path_structure() {
        let crate_name = "r3bl-cmdr".into();
        if let Some(RemoteVersionCheckGate::Enabled { dir, .. }) =
            RemoteVersionCheckGate::try_new(&crate_name, None)
        {
            let expected_suffix = PathBuf::from("r3bl").join("r3bl-cmdr");
            assert!(dir.ends_with(&expected_suffix));
        }
    }

    #[test]
    fn test_temp_dir_lifecycle_in_and_not_in_cooldown() {
        let temp_dir = try_create_temp_dir().unwrap();
        let sub_dir = TempDirChildPath::try_new(&temp_dir, "sub").unwrap();
        let timestamp_file_path = sub_dir
            .as_path_buf()
            .join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);
        let gate = RemoteVersionCheckGate::new_temp_dir(sub_dir, None);

        // 1. Missing timestamp file -> Err(MetadataReadFailed { src_err: NotFound }).
        assert!(matches!(
            gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));

        // 2. Touch timestamp file -> creates parent dir + 0-byte file ->
        //    Ok(ClosedForCooldown) within 1h window.
        assert_eq2!(
            gate.try_touch_timestamp_file().unwrap(),
            GateTouchOutcome::Touched
        );
        assert!(timestamp_file_path.exists());
        assert_eq2!(fs::metadata(&timestamp_file_path).unwrap().len(), 0);
        assert_eq2!(
            gate.try_check_gate_status().unwrap(),
            GateStatus::ClosedForCooldown
        );

        // 3. Backdate real file `mtime` beyond 1h via
        //    `backdate_timestamp_file_for_testing` -> Ok(Open), then re-touch existing
        //    0-byte timestamp file -> refreshes back to Ok(ClosedForCooldown).
        gate.backdate_timestamp_file_for_testing();
        assert_eq2!(gate.try_check_gate_status().unwrap(), GateStatus::Open);
        assert_eq2!(
            gate.try_touch_timestamp_file().unwrap(),
            GateTouchOutcome::Touched
        );
        assert_eq2!(
            gate.try_check_gate_status().unwrap(),
            GateStatus::ClosedForCooldown
        );

        // 4. Future `mtime` (`modified > now` backward clock skew) -> Err(ClockSkew).
        let future_mtime = SystemTime::now() + Duration::from_secs(60);
        File::options()
            .write(true)
            .open(&timestamp_file_path)
            .unwrap()
            .set_modified(future_mtime)
            .unwrap();
        assert!(matches!(
            gate.try_check_gate_status(),
            Err(GateStatusError::ClockSkew { .. })
        ));
    }

    #[test]
    fn test_custom_duration_transitions_to_not_in_cooldown_naturally() {
        let temp_dir = try_create_temp_dir().unwrap();
        let dir = TempDirChildPath::try_new(&temp_dir, "short_duration").unwrap();
        let short_duration = Duration::from_millis(20);
        let gate = RemoteVersionCheckGate::new_temp_dir(dir, Some(short_duration));

        assert!(matches!(
            gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));

        assert_eq2!(
            gate.try_touch_timestamp_file().unwrap(),
            GateTouchOutcome::Touched
        );
        assert_eq2!(
            gate.try_check_gate_status().unwrap(),
            GateStatus::ClosedForCooldown
        );

        // Sleep 25ms to exceed 20ms short duration naturally.
        std::thread::sleep(Duration::from_millis(25));
        assert_eq2!(gate.try_check_gate_status().unwrap(), GateStatus::Open);
    }

    #[test]
    fn test_try_touch_timestamp_file_returns_err_on_io_failure() {
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. `File::create` fails when the target timestamp file path is an existing
        //    directory; checking gate status reports `NotARegularFile`.
        let blocking_dir =
            temp_dir.join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);
        fs::create_dir(&blocking_dir).unwrap();
        let gate_dir_target = RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        );
        assert!(matches!(
            gate_dir_target.try_touch_timestamp_file(),
            Err(GateTouchOutcomeError::CreateFileFailed { .. })
        ));
        assert!(matches!(
            gate_dir_target.try_check_gate_status(),
            Err(GateStatusError::NotARegularFile { .. })
        ));

        // 2. `fs::create_dir_all` fails when directory path is blocked by an existing
        //    file; checking gate status reports `MetadataReadFailed` (e.g. `ENOTDIR` on
        //    Unix or `NotFound` on Windows).
        let blocking_file = temp_dir.join("blocking_file");
        File::create(&blocking_file).unwrap();
        let gate_parent_err = RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "blocking_file/sub").unwrap(),
            None,
        );
        assert!(matches!(
            gate_parent_err.try_touch_timestamp_file(),
            Err(GateTouchOutcomeError::CreateDirFailed { .. })
        ));
        assert!(matches!(
            gate_parent_err.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { .. })
        ));
    }

    #[test]
    #[should_panic(
        expected = "backdate_timestamp_file_for_testing requires RemoteVersionCheckGate::Enabled"
    )]
    fn test_backdate_timestamp_file_for_testing_panics_when_disabled() {
        RemoteVersionCheckGate::Disabled.backdate_timestamp_file_for_testing();
    }

    #[test]
    #[should_panic(
        expected = "backdate_timestamp_file_for_testing requires dir inside temp_dir"
    )]
    fn test_backdate_timestamp_file_for_testing_panics_for_host_cache_dir() {
        let crate_name = "r3bl-cmdr".into();
        RemoteVersionCheckGate::try_new(&crate_name, Some(Duration::from_secs(60)))
            .unwrap()
            .backdate_timestamp_file_for_testing();
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CrateName, CrateVersion, R3BL_BINARIES_RELEASE_BASE_URL,
                    R3BL_VERSIONS_JSON_URL},
            CrateIdentity, UpgradePolicy, UserFeedbackChannel};
use std::path::PathBuf;

/// An immutable configuration for the [`self_upgrade`] engine, held inside
/// [`UpgradeJourney`].
///
/// > See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// > [`self_upgrade`] module documentation for the complete typestate lifecycle.
///
/// This engine has no side effects from environment variables (`CI=<bool>` does **not**
/// influence its behavior):
/// - **Production binaries** construct this automatically via [`upgrade_journey!`] by
///   providing CLI args (from the binary) and the desired [`UserFeedbackChannel`].
///   - This macro creates an [`UpgradeConfig`] converting [`UpgradeCLIArgs`] into
///     [`UpgradePolicy`] and using defaults for the URLs and the default
///     [`CooldownPolicy`].
/// - **Tests and custom callers** construct [`UpgradeConfig::new()`] directly to pass
///   explicit [`UpgradePolicy`] variants, custom URL overrides, or
///   [`CooldownPolicy::Disabled`] / [`CooldownPolicy::CustomTimestampFilePath`].
/// - **Typestate narrowing**: [`UpgradeConfig`] stays unchanged across all
///   [`UpgradeJourney`] state transitions, while [`UpgradeJourney::start()`] unpacks
///   [`UpgradePolicy`] into narrowed state witnesses ([`UpgradeStrategy`] in [`Checking`]
///   and [`UpgradeActionOnExit`] in [`UpgradeAvailable`]).
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`CooldownPolicy::CustomTimestampFilePath`]: CooldownPolicy::CustomTimestampFilePath
/// [`CooldownPolicy::Disabled`]: CooldownPolicy::Disabled
/// [`CooldownPolicy`]: CooldownPolicy
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
/// [`UpgradeConfig::new()`]: Self::new
/// [`UpgradeJourney::start()`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeConfig {
    /// Identity of the binary (crate name & version).
    crate_identity: CrateIdentity,

    /// Channel where progress messages, spinners, and notifications are written.
    user_feedback_channel: UserFeedbackChannel,

    /// Self-upgrade policy derived from CLI flags ([`--upgrade`][u],
    /// [`--upgrade-behavior`][ub]).
    ///
    /// [u]: crate::self_upgrade::UpgradeCLIArgs::upgrade
    /// [ub]: crate::self_upgrade::UpgradeCLIArgs::upgrade_behavior
    upgrade_policy: UpgradePolicy,

    /// URL for the centralized JSONC version manifest.
    jsonc_manifest_url: String,

    /// Base URL for downloading pre-compiled binary release archives.
    binaries_release_base_url: String,

    /// Cooldown timestamp file policy governing `1h` [`mtime`] version-check caching.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/Modification_time
    cooldown_policy: CooldownPolicy,
}

impl UpgradeConfig {
    /// Creates a new [`UpgradeConfig`].
    ///
    /// # Arguments
    ///
    /// * `crate_identity`: The binary's [`CrateIdentity`] (`crate_name` and
    ///   `current_version`).
    /// * `user_feedback_channel`: Output channel where upgrade messages, spinners, and
    ///   notices are written.
    /// * `upgrade_policy`: Resolved [`UpgradePolicy`] governing startup and exit behavior
    ///   for this invocation.
    /// * `maybe_jsonc_manifest_url`: Optional URL override for the centralized JSONC
    ///   version manifest. When `None`, falls back to [`R3BL_VERSIONS_JSON_URL`].
    /// * `maybe_binaries_release_base_url`: Optional base URL override for downloading
    ///   pre-compiled release archives. When `None`, falls back to
    ///   [`R3BL_BINARIES_RELEASE_BASE_URL`].
    /// * `cooldown_policy`: [`CooldownPolicy`] controlling `1h` `mtime` timestamp file
    ///   caching.
    ///
    /// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::self_upgrade::R3BL_BINARIES_RELEASE_BASE_URL
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
    #[must_use]
    pub fn new(
        crate_identity: CrateIdentity,
        user_feedback_channel: UserFeedbackChannel,
        upgrade_policy: UpgradePolicy,
        maybe_jsonc_manifest_url: Option<String>,
        maybe_binaries_release_base_url: Option<String>,
        cooldown_policy: CooldownPolicy,
    ) -> Self {
        let jsonc_manifest_url = maybe_jsonc_manifest_url
            .unwrap_or_else(|| R3BL_VERSIONS_JSON_URL.to_string());
        let binaries_release_base_url = maybe_binaries_release_base_url
            .unwrap_or_else(|| R3BL_BINARIES_RELEASE_BASE_URL.to_string());

        Self {
            crate_identity,
            user_feedback_channel,
            upgrade_policy,
            jsonc_manifest_url,
            binaries_release_base_url,
            cooldown_policy,
        }
    }

    /// Name of the binary's crate (e.g., `"r3bl-cmdr"`).
    #[must_use]
    pub fn get_crate_name(&self) -> &CrateName { &self.crate_identity.crate_name }

    /// Current semantic version of the binary.
    #[must_use]
    pub fn get_current_version(&self) -> CrateVersion {
        self.crate_identity.current_version
    }

    /// Channel where progress messages, spinners, and notifications are written.
    #[must_use]
    pub fn get_user_feedback_channel(&self) -> UserFeedbackChannel {
        self.user_feedback_channel
    }

    /// Self-upgrade policy governing startup and exit behavior for this session.
    #[must_use]
    pub fn get_upgrade_policy(&self) -> UpgradePolicy { self.upgrade_policy }

    /// URL for the centralized JSONC version manifest.
    #[must_use]
    pub fn get_jsonc_manifest_url(&self) -> &str { &self.jsonc_manifest_url }

    /// Base URL for downloading pre-compiled binary release archives.
    #[must_use]
    pub fn get_binaries_release_base_url(&self) -> &str {
        &self.binaries_release_base_url
    }

    /// Cooldown timestamp file policy governing `1h` `mtime` version-check caching.
    #[must_use]
    pub fn get_cooldown_policy(&self) -> &CooldownPolicy { &self.cooldown_policy }
}

/// Cooldown timestamp file policy carried by immutable [`UpgradeConfig`].
///
/// Controls where (or whether) the `1h` `mtime` upgrade-check cooldown timestamp file is
/// read and written by [`check_cooldown_status`] and [`record_successful_check`].
/// Production binaries use [`Self::DefaultCacheDir`], whereas unit tests pass
/// [`Self::Disabled`] or [`Self::CustomTimestampFilePath`] so tests remain completely
/// hermetic and never touch the developer's user cache directory.
///
/// [`check_cooldown_status`]: crate::self_upgrade::check_cooldown_status
/// [`record_successful_check`]: crate::self_upgrade::record_successful_check
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CooldownPolicy {
    /// Production policy storing a 0-byte `mtime` timestamp file at
    /// `<cache_dir>/r3bl/upgrade-check/<crate_name>.last_check` via [`dirs::cache_dir`].
    #[default]
    DefaultCacheDir,

    /// Custom timestamp file path for isolated filesystem unit tests.
    CustomTimestampFilePath(PathBuf),

    /// Disables disk cooldown reads and writes for deterministic unit tests.
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_upgrade_config_field_initialization() {
        let config = UpgradeConfig::new(
            CrateIdentity::new(
                "r3bl-cmdr",
                CrateVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
            ),
            UserFeedbackChannel::Stderr,
            UpgradePolicy::default(),
            None,
            None,
            CooldownPolicy::DefaultCacheDir,
        );

        assert_eq2!(config.get_crate_name(), &CrateName::from("r3bl-cmdr"));
        assert_eq2!(
            config.get_current_version(),
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3,
            }
        );
        assert_eq2!(
            config.get_user_feedback_channel(),
            UserFeedbackChannel::Stderr
        );
        assert_eq2!(config.get_upgrade_policy(), UpgradePolicy::default());
        assert_eq2!(config.get_jsonc_manifest_url(), R3BL_VERSIONS_JSON_URL);
        assert_eq2!(
            config.get_binaries_release_base_url(),
            R3BL_BINARIES_RELEASE_BASE_URL
        );
        assert_eq2!(
            config.get_cooldown_policy(),
            &CooldownPolicy::DefaultCacheDir
        );

        let custom_timestamp_file_path = PathBuf::from("/tmp/r3bl-test.last_check");
        let custom_config = UpgradeConfig::new(
            CrateIdentity::new(
                "r3bl-cmdr",
                CrateVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
            ),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Disabled,
            Some("https://example.com/custom_manifest.jsonc".to_string()),
            Some("https://example.com/releases/download".to_string()),
            CooldownPolicy::CustomTimestampFilePath(custom_timestamp_file_path.clone()),
        );
        assert_eq2!(custom_config.get_upgrade_policy(), UpgradePolicy::Disabled);
        assert_eq2!(
            custom_config.get_jsonc_manifest_url(),
            "https://example.com/custom_manifest.jsonc"
        );
        assert_eq2!(
            custom_config.get_binaries_release_base_url(),
            "https://example.com/releases/download"
        );
        assert_eq2!(
            custom_config.get_cooldown_policy(),
            &CooldownPolicy::CustomTimestampFilePath(custom_timestamp_file_path)
        );
    }
}

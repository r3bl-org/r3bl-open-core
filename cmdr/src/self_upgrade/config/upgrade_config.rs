// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CrateName, CrateVersion, EndpointUrl, manifest_timeouts},
            CrateIdentity, UpgradePolicy, UserFeedbackChannel};
#[cfg(any(test, doc))]
use std::path::PathBuf;
use std::time::Duration;

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
///   - This macro converts [`UpgradeCLIArgs`] into [`UpgradePolicy`] and passes the
///     canonical [`EndpointUrl::default_manifest_url()`] and default [`CooldownPolicy`].
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
/// [`EndpointUrl::default_manifest_url()`]: crate::self_upgrade::EndpointUrl::default_manifest_url
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
    jsonc_manifest_url: EndpointUrl,

    /// Cooldown timestamp file policy governing `1h` [`mtime`] version-check caching.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
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
    /// * `jsonc_manifest_url`: Endpoint URL for the centralized JSONC version manifest.
    /// * `cooldown_policy`: [`CooldownPolicy`] controlling `1h` `mtime` timestamp file
    ///   caching.
    #[must_use]
    pub fn new(
        crate_identity: CrateIdentity,
        user_feedback_channel: UserFeedbackChannel,
        upgrade_policy: UpgradePolicy,
        jsonc_manifest_url: EndpointUrl,
        cooldown_policy: CooldownPolicy,
    ) -> Self {
        Self {
            crate_identity,
            user_feedback_channel,
            upgrade_policy,
            jsonc_manifest_url,
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
    pub fn get_jsonc_manifest_url(&self) -> &EndpointUrl { &self.jsonc_manifest_url }

    /// Cooldown timestamp file policy governing `1h` `mtime` version-check caching.
    #[must_use]
    pub fn get_cooldown_policy(&self) -> &CooldownPolicy { &self.cooldown_policy }
}

/// Cooldown timestamp file policy carried by immutable [`UpgradeConfig`].
///
/// Controls where (or whether) the `1h` ([`Self::cooldown_duration`]) [`mtime`]
/// upgrade-check cooldown timestamp file is read and written by
/// [`check_cooldown_status`] and [`record_successful_check`]. Production binaries use
/// [`Self::DefaultCacheDir`], whereas unit tests pass [`Self::Disabled`] or
/// [`Self::CustomTimestampFilePath`] so tests remain completely hermetic and never touch
/// the developer's user cache directory.
///
/// [`check_cooldown_status`]: crate::self_upgrade::check_cooldown_status
/// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
/// [`record_successful_check`]: crate::self_upgrade::record_successful_check
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CooldownPolicy {
    /// Production policy storing a 0-byte [`mtime`] timestamp file at
    /// `<cache_dir>/`[`r3bl`][1]`/<crate_name>/`[`self_upgrade_last_check`][2] via
    /// [`resolve_cooldown_timestamp_file_path`][3].
    ///
    /// [1]: crate::common::r3bl_file_paths::R3BL_TOP_LEVEL_DIR
    /// [2]: crate::common::r3bl_file_paths::SELF_UPGRADE_LAST_CHECK_FILENAME
    /// [3]: crate::self_upgrade::check::resolve_cooldown_timestamp_file_path
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[default]
    DefaultCacheDir,

    /// Custom timestamp file path for isolated filesystem unit tests.
    #[cfg(any(test, doc))]
    CustomTimestampFilePath(PathBuf),

    /// Disables disk cooldown reads and writes for deterministic unit tests.
    #[cfg(any(test, doc))]
    Disabled,
}

impl CooldownPolicy {
    /// Returns the cooldown duration ([`DEDICATED_THREAD_CHECK_COOLDOWN`][1], `1h`)
    /// before a fresh [`mtime`] timestamp file expires.
    ///
    /// [1]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    #[must_use]
    pub const fn cooldown_duration(&self) -> Duration {
        manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::{JsoncScalar, R3BL_VERSIONS_JSONC_URL},
                *};
    use crate::resolve_self_upgrade_last_check_in_base_dir;
    use r3bl_tui::assert_eq2;
    use std::path::Path;

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
            EndpointUrl::default_manifest_url(),
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
        assert_eq2!(
            &**config.get_jsonc_manifest_url(),
            R3BL_VERSIONS_JSONC_URL
        );
        assert_eq2!(
            config.get_cooldown_policy(),
            &CooldownPolicy::DefaultCacheDir
        );
        assert_eq2!(
            config.get_cooldown_policy().cooldown_duration(),
            manifest_timeouts::DEDICATED_THREAD_CHECK_COOLDOWN
        );

        let custom_timestamp_file_path =
            resolve_self_upgrade_last_check_in_base_dir(Path::new("/tmp"), "r3bl-cmdr");
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
            EndpointUrl::try_parse("https://example.com/custom_manifest.jsonc").unwrap(),
            CooldownPolicy::CustomTimestampFilePath(custom_timestamp_file_path.clone()),
        );
        assert_eq2!(custom_config.get_upgrade_policy(), UpgradePolicy::Disabled);
        assert_eq2!(
            &**custom_config.get_jsonc_manifest_url(),
            "https://example.com/custom_manifest.jsonc"
        );
        assert_eq2!(
            custom_config.get_cooldown_policy(),
            &CooldownPolicy::CustomTimestampFilePath(custom_timestamp_file_path)
        );
    }
}

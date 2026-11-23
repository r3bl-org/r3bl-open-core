// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CrateName, CrateVersion, EndpointUrl, RemoteVersionCheckGate},
            UpgradePolicy, UserFeedbackChannel};

/// An immutable configuration for the [`self_upgrade`] engine, held inside
/// [`UpgradeJourney`].
///
/// > See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// > [`self_upgrade`] module documentation for the complete typestate lifecycle.
///
/// - This engine has no side effects from environment variables (`CI=<bool>` does **not**
///   influence its behavior).
/// - Production binaries construct this struct automatically via [`upgrade_journey!`] by
///   providing CLI args (from the binary) and the desired [`UserFeedbackChannel`] along
///   with a bunch of defaults for all the remaining fields to [`UpgradeConfig::new()`].
/// - Tests construct [`UpgradeConfig::new()`] directly to pass explicit [`UpgradePolicy`]
///   variants, custom URL overrides, or [`RemoteVersionCheckGate::Disabled`] /
///   [`RemoteVersionCheckGate::new_temp_dir`].
///
/// [`UpgradeConfig`] stays unchanged across all [`UpgradeJourney`] state transitions
/// (typestate narrowing), while [`UpgradeJourney::start()`] unpacks [`UpgradePolicy`]
/// into narrowed state witnesses ([`UpgradeStrategy`] in [`Checking`] and
/// [`UpgradeActionOnExit`] in [`UpgradeAvailable`]).
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`RemoteVersionCheckGate::Disabled`]: RemoteVersionCheckGate::Disabled
/// [`RemoteVersionCheckGate::new_temp_dir`]: RemoteVersionCheckGate::new_temp_dir
/// [`RemoteVersionCheckGate`]: RemoteVersionCheckGate
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
    /// URL for the centralized JSONC version manifest.
    jsonc_manifest_url: EndpointUrl,

    /// Name of the crate as registered in [`R3BL_VERSIONS_JSONC_URL`] (e.g.,
    /// `"r3bl-cmdr"`).
    ///
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    crate_name: CrateName,

    /// Current semantic version of the binary compiled from `Cargo.toml`.
    current_version: CrateVersion,

    /// Self-upgrade policy derived from CLI flags ([`--upgrade`][u],
    /// [`--upgrade-behavior`][ub]).
    ///
    /// [u]: crate::self_upgrade::UpgradeCLIArgs::upgrade
    /// [ub]: crate::self_upgrade::UpgradeCLIArgs::upgrade_behavior
    upgrade_policy: UpgradePolicy,

    /// Channel where progress messages, spinners, and notifications are written.
    user_feedback_channel: UserFeedbackChannel,

    /// Cooldown timestamp file policy governing `1h` [`mtime`] version-check caching.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    remote_version_check_gate: RemoteVersionCheckGate,
}

impl UpgradeConfig {
    /// Creates a new [`UpgradeConfig`]. Production code / binaries dont't call this
    /// directly, but use [`upgrade_journey!`] instead.
    ///
    /// # Arguments
    ///
    /// * `crate_name`: Name of the binary's crate as registered in
    ///   [`R3BL_VERSIONS_JSONC_URL`] (e.g., `"r3bl-cmdr"`).
    /// * `current_version`: Current [`CrateVersion`] of the running binary.
    /// * `user_feedback_channel`: Output channel where upgrade messages, spinners, and
    ///   notices are written.
    /// * `upgrade_policy`: Resolved [`UpgradePolicy`] governing startup and exit behavior
    ///   for this invocation.
    /// * `jsonc_manifest_url`: Endpoint URL for the centralized JSONC version manifest.
    /// * `remote_version_check_gate`: [`RemoteVersionCheckGate`] controlling `1h`
    ///   [`mtime`] timestamp file caching.
    ///
    /// [`mtime`]: https://en.wikipedia.org/wiki/MAC_times
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    /// [`upgrade_journey!`]: crate::upgrade_journey!
    #[must_use]
    pub fn new(
        arg_crate_name: impl Into<CrateName>,
        current_version: CrateVersion,
        user_feedback_channel: UserFeedbackChannel,
        upgrade_policy: UpgradePolicy,
        jsonc_manifest_url: EndpointUrl,
        remote_version_check_gate: RemoteVersionCheckGate,
    ) -> Self {
        Self {
            crate_name: arg_crate_name.into(),
            current_version,
            user_feedback_channel,
            upgrade_policy,
            jsonc_manifest_url,
            remote_version_check_gate,
        }
    }

    /// Name of the binary's crate (e.g., `"r3bl-cmdr"`).
    #[must_use]
    pub fn get_crate_name(&self) -> &CrateName { &self.crate_name }

    /// Current semantic version of the binary.
    #[must_use]
    pub fn get_current_version(&self) -> CrateVersion { self.current_version }

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
    pub fn get_remote_version_check_gate(&self) -> &RemoteVersionCheckGate {
        &self.remote_version_check_gate
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::{JsoncScalar, R3BL_VERSIONS_JSONC_URL, manifest_timeouts},
                *};
    use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};

    #[test]
    fn test_upgrade_config_field_initialization() {
        let crate_name = CrateName::from("r3bl-cmdr");
        let default_gate = RemoteVersionCheckGate::try_new(&crate_name, None)
            .unwrap_or(RemoteVersionCheckGate::Disabled);
        let config = UpgradeConfig::new(
            crate_name.clone(),
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
            UserFeedbackChannel::Stderr,
            UpgradePolicy::default(),
            EndpointUrl::default_manifest_url(),
            default_gate.clone(),
        );

        assert_eq2!(config.get_crate_name(), &crate_name);
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
        let manifest_url_str: &str = config.get_jsonc_manifest_url();
        assert_eq2!(manifest_url_str, R3BL_VERSIONS_JSONC_URL);
        assert_eq2!(config.get_remote_version_check_gate(), &default_gate);
        if let RemoteVersionCheckGate::Enabled { duration, .. } =
            config.get_remote_version_check_gate()
        {
            assert_eq2!(*duration, manifest_timeouts::DEFERRED_CHECK_COOLDOWN_DELAY);
        }

        let temp_dir = try_create_temp_dir().unwrap();
        let custom_config = UpgradeConfig::new(
            config.get_crate_name().clone(),
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Disabled,
            EndpointUrl::try_parse("https://example.com/custom_manifest.jsonc").unwrap(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "custom").unwrap(),
                None,
            ),
        );
        assert_eq2!(custom_config.get_upgrade_policy(), UpgradePolicy::Disabled);
        let custom_manifest_url_str: &str = custom_config.get_jsonc_manifest_url();
        assert_eq2!(
            custom_manifest_url_str,
            "https://example.com/custom_manifest.jsonc"
        );
        assert_eq2!(
            custom_config.get_remote_version_check_gate(),
            &RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "custom").unwrap(),
                None,
            )
        );
    }
}

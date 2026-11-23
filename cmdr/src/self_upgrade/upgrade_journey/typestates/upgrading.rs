// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Completed, StateMarker, UpgradeFailedError, UpgradeResolution,
            super::thread_runner::ThreadRunner};
use crate::{CrateMetadata, DEBUG_SELF_UPGRADE, EndpointUrl, PromptUpgradeDecision,
            UpgradeActionOnExit, UpgradeError, UpgradeJourney,
            format_upgrade_available_notice, get_current_bin_name, perform_upgrade,
            prompt_user_to_upgrade};

/// Typestate while an upgrade or interactive exit prompt is actively executing on the
/// current thread (`Checking -> Upgrading` for [`UpgradeStrategy::Immediate`];
/// `UpgradeAvailable -> Upgrading` for [`UpgradeStrategy::Deferred`]). This type
/// is part of the [Control Plane] (see [Thread Model]).
///
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrading {
    /// Verified remote [`CrateMetadata`] being installed or presented to the user.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Base URL for downloading pre-compiled binary release archives from the manifest.
    pub binaries_release_base_url: EndpointUrl,
}
impl StateMarker for Upgrading {}

impl UpgradeJourney<Upgrading> {
    #[must_use]
    pub(in crate::self_upgrade) fn into_completed(
        self,
        resolution: UpgradeResolution,
    ) -> UpgradeJourney<Completed> {
        UpgradeJourney {
            config: self.config,
            state: Completed {
                metadata: self.state.metadata,
                resolution,
            },
        }
    }

    /// Executes an immediate (`--upgrade`) installation via [`perform_upgrade`], touches
    /// the `1h` cooldown timestamp file if and only if installation succeeded, and
    /// consumes `self` into [`UpgradeJourney<Completed>`].
    ///
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    #[must_use]
    pub(in crate::self_upgrade) async fn execute_immediate_upgrade(
        self,
    ) -> UpgradeJourney<Completed> {
        match perform_upgrade(
            &self.config,
            &self.state.metadata,
            &self.state.binaries_release_base_url,
        )
        .await
        {
            Ok(()) => {
                self.touch_cooldown_on_upgraded();
                self.into_completed(UpgradeResolution::Upgraded)
            }
            Err(err) => self.into_completed(UpgradeResolution::UpgradeFailed(
                UpgradeFailedError::from(err),
            )),
        }
    }

    /// Executes the upgrade action on exit (`NotifyOnly`, `Auto`, or `PromptToUpgrade`),
    /// touches the `1h` cooldown timestamp file if and only if the upgrade succeeded, and
    /// consumes `self` into [`UpgradeJourney<Completed>`].
    #[must_use]
    pub(in crate::self_upgrade) fn execute_action_on_exit(
        self,
        action_on_exit: UpgradeActionOnExit,
    ) -> UpgradeJourney<Completed> {
        let bin_name = get_current_bin_name();
        let current_version_str = self.config.get_current_version().to_string();
        let remote_version_str = self.state.metadata.version.to_string();

        self.config
            .get_user_feedback_channel()
            .println(format_upgrade_available_notice(
                &bin_name,
                &current_version_str,
                &remote_version_str,
            ));

        let resolution = match action_on_exit {
            UpgradeActionOnExit::NotifyOnly => UpgradeResolution::NotifiedOnly,
            UpgradeActionOnExit::Auto => {
                let config = &self.config;
                let metadata = &self.state.metadata;
                let binaries_release_base_url = &self.state.binaries_release_base_url;
                ThreadRunner::run_blocking(config, async || {
                    resolve_upgrade_result(
                        perform_upgrade(config, metadata, binaries_release_base_url).await,
                    )
                })
                .unwrap_or_else(|err| {
                    UpgradeResolution::UpgradeFailed(UpgradeFailedError::from(err))
                })
            }
            UpgradeActionOnExit::PromptToUpgrade => {
                let config = &self.config;
                let metadata = &self.state.metadata;
                let binaries_release_base_url = &self.state.binaries_release_base_url;
                ThreadRunner::run_blocking(
                    config,
                    async || match prompt_user_to_upgrade(
                        config.get_crate_name(),
                        &remote_version_str,
                    )
                    .await
                    {
                        PromptUpgradeDecision::SkippedNonInteractive => {
                            UpgradeResolution::SkippedNonInteractive
                        }
                        PromptUpgradeDecision::DeclinedByUser => {
                            UpgradeResolution::DeclinedByUser
                        }
                        PromptUpgradeDecision::UpgradeNow => resolve_upgrade_result(
                            perform_upgrade(config, metadata, binaries_release_base_url)
                                .await,
                        ),
                    },
                )
                .unwrap_or_else(|err| {
                    UpgradeResolution::UpgradeFailed(UpgradeFailedError::from(err))
                })
            }
        };

        if matches!(resolution, UpgradeResolution::Upgraded) {
            self.touch_cooldown_on_upgraded();
        }

        self.into_completed(resolution)
    }

    fn touch_cooldown_on_upgraded(&self) {
        if let Err(err) = self
            .config
            .get_remote_version_check_gate()
            .try_touch_timestamp_file()
        {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Could not enter cooldown after upgrade succeeded",
                    crate_name = %self.config.get_crate_name(),
                    error = ?err
                );
            });
        }
    }
}

fn resolve_upgrade_result(res_upgrade: Result<(), UpgradeError>) -> UpgradeResolution {
    match res_upgrade {
        Ok(()) => UpgradeResolution::Upgraded,
        Err(err) => UpgradeResolution::UpgradeFailed(UpgradeFailedError::from(err)),
    }
}

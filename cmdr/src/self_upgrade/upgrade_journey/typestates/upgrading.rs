// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Completed, UpgradeJourneyStateMarker, UpgradeResolution,
            super::tokio_thread_runner::run_in_tokio_logged};
use crate::{CrateMetadata, EndpointUrl, PromptUpgradeDecision, UpgradeActionOnExit,
            UpgradeError, UpgradeJourney, format_upgrade_available_notice,
            get_current_bin_name, perform_upgrade, prompt_user_to_upgrade};
use r3bl_tui::TokioRuntimeOutcome;

/// Typestate while an upgrade or interactive exit prompt is actively executing after the
/// `SINGLETON` leaf lock has been released (`Checking -> Upgrading` for
/// [`UpgradeStrategy::Immediate`]; `UpgradeAvailable -> Upgrading` for
/// [`UpgradeStrategy::DedicatedThread`]).
///
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrading {
    /// Verified remote [`CrateMetadata`] being installed or presented to the user.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Base URL for downloading pre-compiled binary release archives from the manifest.
    pub binaries_release_base_url: EndpointUrl,
}
impl UpgradeJourneyStateMarker for Upgrading {}

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

    /// Executes the upgrade action on exit (`NotifyOnly`, `Auto`, or `PromptToUpgrade`)
    /// outside the `SINGLETON` leaf lock, touches the `1h` cooldown timestamp file if and
    /// only if the upgrade succeeded, and consumes `self` into
    /// [`UpgradeJourney<Completed>`].
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
                match run_in_tokio_logged(config, || {
                    perform_upgrade(config, metadata, binaries_release_base_url)
                }) {
                    TokioRuntimeOutcome::Completed(Ok(())) => UpgradeResolution::Upgraded,
                    TokioRuntimeOutcome::Completed(Err(err)) => {
                        UpgradeResolution::UpgradeFailed(err)
                    }
                    TokioRuntimeOutcome::FailedToStart(err)
                    | TokioRuntimeOutcome::Panicked(err) => {
                        UpgradeResolution::UpgradeFailed(UpgradeError::Other(format!(
                            "{err:#}"
                        )))
                    }
                }
            }
            UpgradeActionOnExit::PromptToUpgrade => {
                let config = &self.config;
                let metadata = &self.state.metadata;
                let binaries_release_base_url = &self.state.binaries_release_base_url;
                match run_in_tokio_logged(config, async || match prompt_user_to_upgrade(
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
                    PromptUpgradeDecision::UpgradeNow => {
                        match perform_upgrade(config, metadata, binaries_release_base_url)
                            .await
                        {
                            Ok(()) => UpgradeResolution::Upgraded,
                            Err(err) => UpgradeResolution::UpgradeFailed(err),
                        }
                    }
                }) {
                    TokioRuntimeOutcome::Completed(res) => res,
                    TokioRuntimeOutcome::FailedToStart(err)
                    | TokioRuntimeOutcome::Panicked(err) => {
                        UpgradeResolution::UpgradeFailed(UpgradeError::Other(format!(
                            "{err:#}"
                        )))
                    }
                }
            }
        };

        if matches!(resolution, UpgradeResolution::Upgraded) {
            self.config
                .get_cooldown_policy()
                .touch(self.config.get_crate_name());
        }

        self.into_completed(resolution)
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{ImmediateUpgradeResult, StartupOutcome,
                    tokio_thread_runner::{run_in_tokio_logged,
                                          run_on_dedicated_check_thread}},
            Checking, CooldownSkipped, Disabled, UpgradeJourneyStateMarker,
            UpgradeResolution};
use crate::{CooldownStatus, DEBUG_SELF_UPGRADE, ImmediateCheckOutcome,
            ManifestCheckError, UpgradeCheckStatus, UpgradeConfig, UpgradeError,
            UpgradeJourney, UpgradePolicy, UpgradeStrategy, check_for_immediate_upgrade,
            check_for_upgrade, check_thread, format_already_up_to_date_msg,
            format_upgrade_failed_retry_msg, get_current_bin_name, perform_upgrade};
use r3bl_tui::TokioRuntimeOutcome;

/// Initial typestate before [`UpgradeJourney::start`] is called.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotStarted;
impl UpgradeJourneyStateMarker for NotStarted {}

impl UpgradeJourney<NotStarted> {
    /// Constructs a new [`UpgradeJourney<NotStarted>`] from immutable [`UpgradeConfig`].
    ///
    /// In CLI binaries, prefer [`upgrade_journey!`], which accepts [`UpgradeCLIArgs`] and
    /// converts it into [`UpgradePolicy`] inside [`UpgradeConfig`] at the CLI boundary.
    ///
    /// [`upgrade_journey!`]: crate::upgrade_journey
    /// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
    /// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
    /// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
    #[must_use]
    pub fn new(config: UpgradeConfig) -> Self {
        Self {
            config,
            state: NotStarted,
        }
    }

    /// Single entry point for the self-upgrade engine. Atomically claims
    /// [`UpgradeCheckStatus::NotStarted`] and dispatches based on
    /// [`UpgradeConfig::get_upgrade_policy()`]:
    ///
    /// - **Duplicate `.start()` call**: Leaves [`UpgradeCheckStatus`] untouched, spawns
    ///   zero threads, logs via [`DEBUG_SELF_UPGRADE`], and returns
    ///   [`StartupOutcome::AlreadyStarted`].
    /// - **`UpgradePolicy::Enabled(UpgradeStrategy::Immediate)`** (`--upgrade`): Queries
    ///   the manifest with a `5s` timeout (bypassing the `1h` cooldown), runs
    ///   [`perform_upgrade`] if a newer version exists, updates [`UpgradeCheckStatus`]
    ///   through [`Checking`] -> [`UpToDate`] / [`CheckFailed`] / [`Upgrading`] ->
    ///   [`Completed`], touches the `1h` cooldown timestamp file on [`UpToDate`] or
    ///   [`UpgradeResolution::Upgraded`], and returns
    ///   [`StartupOutcome::UpgradedAndExit`].
    /// - **`UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(action_on_exit))`**:
    ///   Checks the `1h` `mtime` cooldown timestamp file via
    ///   [`CooldownPolicy::check_status`]. If active, records
    ///   [`UpgradeCheckStatus::CooldownSkipped`] with zero threads and returns
    ///   [`StartupOutcome::ContinueApplication`]. Otherwise records
    ///   [`UpgradeCheckStatus::Checking`], spawns a single `"self-upgrade-check"` OS
    ///   thread (storing its `JoinHandle` for
    ///   [`UpgradeCheckStatus::handle_upgrade_on_exit`]), and returns
    ///   [`StartupOutcome::ContinueApplication`].
    /// - **[`UpgradePolicy::Disabled`]** (`--upgrade-behavior disable`): Records
    ///   [`UpgradeCheckStatus::Disabled`] with zero threads and zero I/O, returning
    ///   [`StartupOutcome::ContinueApplication`].
    ///
    /// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
    /// [`self_upgrade`] module documentation.
    ///
    /// [`CheckFailed`]: crate::self_upgrade::CheckFailed
    /// [`Checking`]: crate::self_upgrade::Checking
    /// [`Completed`]: crate::self_upgrade::Completed
    /// [`CooldownPolicy::check_status`]: crate::self_upgrade::CooldownPolicy::check_status
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`UpgradeCheckStatus::Checking`]: crate::self_upgrade::UpgradeCheckStatus::Checking
    /// [`UpgradeCheckStatus::CooldownSkipped`]: crate::self_upgrade::UpgradeCheckStatus::CooldownSkipped
    /// [`UpgradeCheckStatus::Disabled`]: crate::self_upgrade::UpgradeCheckStatus::Disabled
    /// [`UpgradeCheckStatus::handle_upgrade_on_exit`]: crate::self_upgrade::UpgradeCheckStatus::handle_upgrade_on_exit
    /// [`UpgradeCheckStatus::NotStarted`]: crate::self_upgrade::UpgradeCheckStatus::NotStarted
    /// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
    /// [`UpgradeConfig::get_upgrade_policy()`]: crate::self_upgrade::UpgradeConfig::get_upgrade_policy
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    /// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
    #[must_use]
    pub fn start(self) -> StartupOutcome {
        match self.config.get_upgrade_policy() {
            UpgradePolicy::Disabled => {
                let config_for_log = self.config.clone();
                let disabled = self.into_disabled();
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Disabled(disabled),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartupOutcome::ContinueApplication
            }
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate) => {
                let config_for_log = self.config.clone();
                let checking = self.into_checking(UpgradeStrategy::Immediate);
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Checking(checking.clone()),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartupOutcome::UpgradedAndExit(Self::run_immediate_upgrade(checking))
            }
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(action_on_exit)) => {
                if !UpgradeCheckStatus::is_not_started() {
                    return log_duplicate_start(&self.config);
                }

                let config_for_log = self.config.clone();
                match self
                    .config
                    .get_cooldown_policy()
                    .check_status(self.config.get_crate_name())
                {
                    CooldownStatus::Active => {
                        let skipped = self.into_cooldown_skipped();
                        if !UpgradeCheckStatus::try_claim_not_started(
                            UpgradeCheckStatus::CooldownSkipped(skipped),
                        ) {
                            return log_duplicate_start(&config_for_log);
                        }
                        StartupOutcome::ContinueApplication
                    }
                    CooldownStatus::ExpiredOrMissing => {
                        let checking = self.into_checking(
                            UpgradeStrategy::DedicatedThread(action_on_exit),
                        );
                        if !UpgradeCheckStatus::try_claim_not_started(
                            UpgradeCheckStatus::Checking(checking.clone()),
                        ) {
                            return log_duplicate_start(&config_for_log);
                        }

                        let spawn_checking = checking.clone();
                        match std::thread::Builder::new()
                            .name(check_thread::CHECK_THREAD_NAME.to_string())
                            .spawn(move || {
                                run_on_dedicated_check_thread(
                                    spawn_checking,
                                    check_for_upgrade,
                                );
                            }) {
                            Ok(handle) => {
                                UpgradeCheckStatus::set_thread_check_handle(handle);
                            }
                            Err(io_err) => {
                                let error =
                                    ManifestCheckError::RuntimeOrThreadInitFailed {
                                        reason: format!(
                                            "Failed to spawn dedicated self-upgrade check thread: {io_err:#}"
                                        ),
                                    };
                                DEBUG_SELF_UPGRADE.then(|| {
                                    // % is Display, ? is Debug.
                                    tracing::debug!(
                                        message = "Could not spawn dedicated self-upgrade check thread",
                                        crate_name = %checking.get_config().get_crate_name(),
                                        error = %error
                                    );
                                });
                                UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                                    checking.into_check_failed(error),
                                ));
                            }
                        }

                        StartupOutcome::ContinueApplication
                    }
                }
            }
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_disabled(self) -> UpgradeJourney<Disabled> {
        UpgradeJourney {
            config: self.config,
            state: Disabled,
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_cooldown_skipped(
        self,
    ) -> UpgradeJourney<CooldownSkipped> {
        UpgradeJourney {
            config: self.config,
            state: CooldownSkipped,
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_checking(
        self,
        strategy: UpgradeStrategy,
    ) -> UpgradeJourney<Checking> {
        UpgradeJourney {
            config: self.config,
            state: Checking { strategy },
        }
    }

    fn run_immediate_upgrade(
        checking: UpgradeJourney<Checking>,
    ) -> ImmediateUpgradeResult {
        let config = checking.get_config().clone();
        let fallback_checking = checking.clone();

        match run_in_tokio_logged(&config, async move || -> ImmediateUpgradeResult {
            match check_for_immediate_upgrade(checking).await {
                ImmediateCheckOutcome::CheckFailed(failed) => {
                    let bin_name = get_current_bin_name();
                    let msg = format_upgrade_failed_retry_msg(
                        &bin_name,
                        &failed.get_state().error.to_string(),
                    );
                    failed
                        .get_config()
                        .get_user_feedback_channel()
                        .eprintln(msg);
                    ImmediateUpgradeResult::Failed
                }
                ImmediateCheckOutcome::UpToDate(up_to_date) => {
                    up_to_date.get_config().get_user_feedback_channel().println(
                        format_already_up_to_date_msg(
                            up_to_date.get_config().get_crate_name(),
                            &up_to_date.get_config().get_current_version().to_string(),
                        ),
                    );
                    ImmediateUpgradeResult::Succeeded
                }
                ImmediateCheckOutcome::Upgrading(upgrading) => {
                    match perform_upgrade(
                        upgrading.get_config(),
                        &upgrading.get_state().metadata,
                        &upgrading.get_state().binaries_release_base_url,
                    )
                    .await
                    {
                        Ok(()) => {
                            upgrading
                                .get_config()
                                .get_cooldown_policy()
                                .touch(upgrading.get_config().get_crate_name());
                            let completed =
                                upgrading.into_completed(UpgradeResolution::Upgraded);
                            UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                                completed,
                            ));
                            ImmediateUpgradeResult::Succeeded
                        }
                        Err(err) => {
                            let completed = upgrading
                                .into_completed(UpgradeResolution::UpgradeFailed(err));
                            UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                                completed,
                            ));
                            ImmediateUpgradeResult::Failed
                        }
                    }
                }
            }
        }) {
            TokioRuntimeOutcome::Completed(result) => result,
            TokioRuntimeOutcome::FailedToStart(err)
            | TokioRuntimeOutcome::Panicked(err) => {
                let error_str = format!("{err:#}");
                let bin_name = get_current_bin_name();
                config
                    .get_user_feedback_channel()
                    .eprintln(format_upgrade_failed_retry_msg(&bin_name, &error_str));

                if let UpgradeCheckStatus::Upgrading(upgrading) =
                    UpgradeCheckStatus::get()
                {
                    UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                        upgrading.into_completed(UpgradeResolution::UpgradeFailed(
                            UpgradeError::Other(error_str),
                        )),
                    ));
                } else {
                    UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                        fallback_checking.into_check_failed(
                            ManifestCheckError::RuntimeOrThreadInitFailed {
                                reason: error_str,
                            },
                        ),
                    ));
                }
                ImmediateUpgradeResult::Failed
            }
        }
    }
}

fn log_duplicate_start(config: &UpgradeConfig) -> StartupOutcome {
    DEBUG_SELF_UPGRADE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "UpgradeJourney::start called more than once in this process; ignoring duplicate",
            crate_name = %config.get_crate_name(),
        );
    });
    StartupOutcome::AlreadyStarted
}

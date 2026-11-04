// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{ImmediateUpgradeResult, StartOutcome,
                    tokio_thread_runner::{run_in_tokio_logged,
                                          run_on_dedicated_check_thread}},
            Checking, CooldownSkipped, Disabled, ImmediateCheckOutcome,
            UpgradeJourneyStateMarker, UpgradeResolution};
use crate::{CooldownStatus, DEBUG_SELF_UPGRADE, ManifestCheckError, UpgradeCheckStatus,
            UpgradeConfig, UpgradeError, UpgradeJourney, UpgradePolicy, UpgradeStrategy,
            check_thread, format_already_up_to_date_msg,
            format_upgrade_failed_retry_msg, get_current_bin_name};
use r3bl_tui::TokioRuntimeOutcome;

/// Initial typestate of [`UpgradeJourney<NotStarted>`] before [`UpgradeJourney::start`]
/// is called.
///
/// # Thread Model
///
/// As part of the [Control Plane], depending on the active [`UpgradeStrategy`], state
/// transitions in [`SINGLETON`] are driven either across a dedicated check thread and the
/// current thread, or entirely on the current thread:
///
/// ## Deferred Upgrade on Exit
///
/// Under [`UpgradeStrategy::DedicatedThread`] (`--upgrade-behavior`), the **Current
/// Thread** starts the check and finishes the upgrade at exit, while the **Dedicated
/// Check Thread** (`"self-upgrade-check"`) checks the manifest while the app runs:
///
/// ```text
/// CURRENT THREAD                                  DEDICATED CHECK THREAD ("self-upgrade-check")
/// ──────────────                                  ─────────────────────────────────────────────
/// 1. Calls journey.start()
///    • If 1h cooldown active:
///      claims SINGLETON = CooldownSkipped & returns
///    • Otherwise: claims SINGLETON = Checking
///      and spawns "self-upgrade-check" ─────────► 2. Calls checking
///    │                                                 .check_for_dedicated_thread_upgrade()
///    ▼                                                 .await (2s timeout)
///    Returns ContinueApplication                     and sets SINGLETON =
///    (App runs normal TUI / CLI work)                  UpToDate | UpgradeAvailable | CheckFailed
///    │                                               via UpgradeCheckStatus::set(...)
///    ▼                                               │
/// 3. At exit, calls                                  │
///    UpgradeCheckStatus::handle_upgrade_on_exit()    │
///    • Joins check thread (outside lock) ◄───────────┘
///    • If UpgradeAvailable: claims Upgrading,
///      calls upgrading.execute_action_on_exit(),
///      and sets SINGLETON = Completed
///      via UpgradeCheckStatus::set(Completed)
/// ```
///
/// ## Immediate Upgrade
///
/// Under [`UpgradeStrategy::Immediate`] (`--upgrade`), no dedicated check thread is
/// spawned; the **Current Thread** runs both the check and the upgrade inside
/// [`UpgradeJourney::start`] before returning [`StartOutcome::UpgradedAndExit`]:
///
/// ```text
/// CURRENT THREAD (inside journey.start() -> Self::run_immediate_upgrade())
/// ────────────────────────────────────────────────────────────────────────
/// 1. Claims SINGLETON = Checking
///    via try_claim_not_started(Checking)
///    │
/// 2. Calls checking.check_for_immediate_upgrade().await (5s timeout)
///    and sets SINGLETON = UpToDate | CheckFailed | Upgrading
///    via UpgradeCheckStatus::set(outcome.into_check_status())
///    │
/// 3. If Upgrading:
///    Calls upgrading.execute_immediate_upgrade().await
///    and sets SINGLETON = Completed
///    via UpgradeCheckStatus::set(Completed)
///    │
/// 4. Returns StartOutcome::UpgradedAndExit(Succeeded | Failed) to caller,
///    and the binary exits immediately.
/// ```
///
/// [`SINGLETON`]: crate::self_upgrade::SINGLETON
/// [`StartOutcome::UpgradedAndExit`]: crate::self_upgrade::StartOutcome::UpgradedAndExit
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
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

    /// This method is called by `main()` at startup to launch the [`self_upgrade`]
    /// engine. This is part of the [Control Plane].
    ///
    /// Atomically claims [`UpgradeCheckStatus::NotStarted`] in [`SINGLETON`] and
    /// dispatches based on [`UpgradeConfig::get_upgrade_policy()`]:
    ///
    /// 1. **Duplicate `.start()` calls**:
    ///    - Leaves [`SINGLETON`] untouched, spawns zero threads, logs via
    ///      [`DEBUG_SELF_UPGRADE`], and returns [`StartOutcome::AlreadyStarted`].
    ///
    /// 2. **Immediate upgrade ([`UpgradeStrategy::Immediate`], `--upgrade`)**:
    ///    - Checks the manifest (`5s` timeout, bypassing the `1h` cooldown) and runs
    ///      [`perform_upgrade`] on the current thread if a newer version exists,
    ///      returning [`StartOutcome::UpgradedAndExit`].
    ///
    /// 3. **Deferred upgrade on exit ([`UpgradeStrategy::DedicatedThread`],
    ///   `--upgrade-behavior`)**:
    ///    - Checks the `1h` cooldown timestamp file via
    ///      [`CooldownPolicy::check_status`][status]; if active, records
    ///      [`UpgradeCheckStatus::CooldownSkipped`], or if expired, spawns the dedicated
    ///      `"self-upgrade-check"` thread for
    ///      [`UpgradeCheckStatus::handle_upgrade_on_exit`] and returns
    ///      [`StartOutcome::ContinueApplication`].
    ///
    /// 4. **Disabled ([`UpgradePolicy::Disabled`], `--upgrade-behavior disable`)**:
    ///    - Records [`UpgradeCheckStatus::Disabled`][disabled] with zero threads and zero
    ///      I/O, returning [`StartOutcome::ContinueApplication`].
    ///
    /// See [Thread Model] on [`NotStarted`] for the thread-by-thread state transition
    /// diagram, and [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
    /// [`self_upgrade`] module documentation.
    ///
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [disabled]: crate::self_upgrade::UpgradeCheckStatus::Disabled
    /// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
    /// [status]: crate::self_upgrade::CooldownPolicy::check_status
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    #[must_use]
    pub fn start(self) -> StartOutcome {
        match self.config.get_upgrade_policy() {
            UpgradePolicy::Disabled => {
                let config_for_log = self.config.clone();
                let disabled = self.into_disabled();
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Disabled(disabled),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartOutcome::ContinueApplication
            }
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate) => {
                let config_for_log = self.config.clone();
                let checking = self.into_checking(UpgradeStrategy::Immediate);
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Checking(checking.clone()),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartOutcome::UpgradedAndExit(Self::run_immediate_upgrade(checking))
            }
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(action_on_exit)) => {
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
                        StartOutcome::ContinueApplication
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
                                let outcome = run_on_dedicated_check_thread(
                                    spawn_checking,
                                    async move |journey| {
                                        journey
                                            .check_for_dedicated_thread_upgrade(
                                                action_on_exit,
                                            )
                                            .await
                                    },
                                );
                                UpgradeCheckStatus::set(outcome.into_check_status());
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

                        StartOutcome::ContinueApplication
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
            let outcome = checking.check_for_immediate_upgrade().await;
            UpgradeCheckStatus::set(outcome.clone().into_check_status());

            match outcome {
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
                    let (completed, result) = upgrading.execute_immediate_upgrade().await;
                    UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(completed));
                    result
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

fn log_duplicate_start(config: &UpgradeConfig) -> StartOutcome {
    DEBUG_SELF_UPGRADE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "UpgradeJourney::start called more than once in this process; ignoring duplicate",
            crate_name = %config.get_crate_name(),
        );
    });
    StartOutcome::AlreadyStarted
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{DeferredUpgradeOutcome, ImmediateUpgradeOutcome, SingletonState,
                    StartClaimStatus, StartOutcome,
                    thread_runner::{run_blocking_logged, spawn_deferred_check_thread}},
            Checking, CooldownSkipped, Disabled, ImmediateCheckOutcome,
            UpgradeJourneyStateMarker};
use crate::{CooldownStatus, DEBUG_SELF_UPGRADE, ManifestCheckError, UpgradeConfig,
            UpgradeJourney, UpgradePolicy, UpgradeStrategy,
            format_already_up_to_date_msg, format_upgrade_failed_retry_msg,
            get_current_bin_name};
use r3bl_tui::TokioRuntimeOutcome;

/// Initial typestate of [`UpgradeJourney<NotStarted>`] before [`UpgradeJourney::start`]
/// is called.
///
/// # Thread Model
///
/// As part of the [Control Plane], depending on the active [`UpgradeStrategy`], state
/// transitions are driven either across a dedicated thread and the current thread, or
/// entirely on the current thread:
///
/// ## Deferred Upgrade on Exit
///
/// Under [`UpgradeStrategy::Deferred`] (`--upgrade-behavior`), the **Current Thread**
/// starts the check and finishes the upgrade at exit, while the **Dedicated Thread**
/// (`"self-upgrade"`) checks the manifest while the app runs and returns
/// [`DeferredCheckOutcome`] inside [`DeferredCheckHandle`]:
///
/// ```text
/// CURRENT THREAD                                  DEDICATED THREAD ("self-upgrade")
/// ──────────────                                  ─────────────────────────────────
/// 1. Calls journey.start()
///    • Claims SINGLETON -> SingletonState::Started(None)
///    • If 1h cooldown active:
///      returns DeferredUpgrade(CooldownSkipped)
///    • Otherwise: transitions to Checking
///      and spawns "self-upgrade" via
///      StdThreadTokioRunner::run_non_blocking,
///      storing DeferredCheckHandle in
///      SingletonState::Started(Some(handle)) ───► 2. Calls checking
///    │                                                 .check_for_deferred_upgrade()
///    ▼                                                 .await (2s timeout)
///    Returns DeferredUpgrade(                        and returns DeferredCheckOutcome:
///      DeferredCheckSpawned(Checking))                 UpToDate | UpgradeAvailable | CheckFailed
///    (App runs normal TUI / CLI work)                  │
///    │                                                 │
///    ▼                                                 │
/// 3. At exit, calls                                    │
///    handle_upgrade_on_exit()                          │
///    • Takes DeferredCheckHandle from SINGLETON        │
///      (drops lock) and joins dedicated thread ◄───────┘
///    • If UpgradeAvailable: transitions to Upgrading,
///      calls upgrading.execute_action_on_exit(),
///      and returns ExitOutcome::Completed(Completed)
/// ```
///
/// ## Immediate Upgrade
///
/// Under [`UpgradeStrategy::Immediate`] (`--upgrade`), no dedicated thread is spawned;
/// the **Current Thread** runs both the check and the upgrade inside
/// [`UpgradeJourney::start`] before returning [`StartOutcome::ImmediateUpgrade`]:
///
/// ```text
/// CURRENT THREAD (inside journey.start() -> Self::run_immediate_upgrade())
/// ────────────────────────────────────────────────────────────────────────
/// 1. Claims SINGLETON -> SingletonState::Started(None)
///    and transitions NotStarted -> Checking
///    │
/// 2. Calls checking.check_for_immediate_upgrade().await (5s timeout)
///    -> UpToDate | CheckFailed | Upgrading
///    │
/// 3. If Upgrading:
///    Calls upgrading.execute_immediate_upgrade().await
///    -> Completed(Upgraded | UpgradeFailed)
///    │
/// 4. Returns StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome) to caller,
///    and the binary exits immediately.
/// ```
///
/// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
/// [`DeferredCheckOutcome`]: crate::self_upgrade::DeferredCheckOutcome
/// [`SINGLETON`]: crate::self_upgrade::SINGLETON
/// [`StartOutcome::ImmediateUpgrade`]: crate::self_upgrade::StartOutcome::ImmediateUpgrade
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
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
    /// Atomically claims [`SingletonState`] in [`SINGLETON`] and dispatches based on
    /// [`UpgradeConfig::get_upgrade_policy()`]:
    ///
    /// 1. **Duplicate `.start()` calls**:
    ///    - Leaves [`SINGLETON`] untouched, spawns zero threads, logs via
    ///      [`DEBUG_SELF_UPGRADE`], and returns [`StartOutcome::AlreadyStarted`].
    ///
    /// 2. **Immediate upgrade ([`UpgradeStrategy::Immediate`], `--upgrade`)**:
    ///    - Checks the manifest (`5s` timeout, bypassing the `1h` cooldown) and runs
    ///      [`perform_upgrade`] on the current thread if a newer version exists,
    ///      returning [`StartOutcome::ImmediateUpgrade`].
    ///
    /// 3. **Deferred upgrade on exit ([`UpgradeStrategy::Deferred`],
    ///   `--upgrade-behavior`)**:
    ///    - Checks the `1h` cooldown timestamp file via
    ///      [`CooldownPolicy::check_status`][status]; if active, returns
    ///      [`DeferredUpgradeOutcome::CooldownSkipped`], or if expired, spawns the
    ///      dedicated `"self-upgrade"` thread via
    ///      [`StdThreadTokioRunner::run_non_blocking`] (storing its
    ///      [`DeferredCheckHandle`] in [`SINGLETON`] for [`handle_upgrade_on_exit`]) and
    ///      returns [`DeferredUpgradeOutcome::DeferredCheckSpawned`].
    ///
    /// 4. **Disabled ([`UpgradePolicy::Disabled`], `--upgrade-behavior disable`)**:
    ///    - Transitions to [`UpgradeJourney<Disabled>`] with zero threads and zero I/O,
    ///      returning [`StartOutcome::Disabled`].
    ///
    /// See [Thread Model] on [`NotStarted`] for the thread-by-thread state transition
    /// diagram, and [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
    /// [`self_upgrade`] module documentation.
    ///
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    /// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
    /// [`handle_upgrade_on_exit`]: crate::self_upgrade::handle_upgrade_on_exit
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`StdThreadTokioRunner::run_non_blocking`]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
    /// [`UpgradeJourney<Disabled>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
    /// [status]: crate::self_upgrade::CooldownPolicy::check_status
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    #[must_use]
    pub fn start(self) -> StartOutcome {
        if SingletonState::try_claim_start() == StartClaimStatus::AlreadyStarted {
            return log_duplicate_start(&self.config);
        }

        match self.config.get_upgrade_policy() {
            UpgradePolicy::Disabled => StartOutcome::Disabled(self.into_disabled()),
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate) => {
                let checking = self.into_checking(UpgradeStrategy::Immediate);
                StartOutcome::ImmediateUpgrade(Self::run_immediate_upgrade(checking))
            }
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(action_on_exit)) => {
                match self
                    .config
                    .get_cooldown_policy()
                    .check_status(self.config.get_crate_name())
                {
                    CooldownStatus::Active => StartOutcome::DeferredUpgrade(
                        DeferredUpgradeOutcome::CooldownSkipped(
                            self.into_cooldown_skipped(),
                        ),
                    ),
                    CooldownStatus::ExpiredOrMissing => {
                        let checking =
                            self.into_checking(UpgradeStrategy::Deferred(action_on_exit));
                        StartOutcome::DeferredUpgrade(spawn_deferred_check_thread(
                            checking,
                            action_on_exit,
                        ))
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
    ) -> ImmediateUpgradeOutcome {
        let config = checking.get_config().clone();
        let fallback_checking = checking.clone();

        match run_blocking_logged(&config, async move || -> ImmediateUpgradeOutcome {
            match checking.check_for_immediate_upgrade().await {
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
                    ImmediateUpgradeOutcome::CheckFailed(failed)
                }
                ImmediateCheckOutcome::UpToDate(up_to_date) => {
                    up_to_date.get_config().get_user_feedback_channel().println(
                        format_already_up_to_date_msg(
                            up_to_date.get_config().get_crate_name(),
                            &up_to_date.get_config().get_current_version().to_string(),
                        ),
                    );
                    ImmediateUpgradeOutcome::UpToDate(up_to_date)
                }
                ImmediateCheckOutcome::Upgrading(upgrading) => {
                    let completed = upgrading.execute_immediate_upgrade().await;
                    ImmediateUpgradeOutcome::Completed(completed)
                }
            }
        }) {
            TokioRuntimeOutcome::Completed(outcome) => outcome,
            TokioRuntimeOutcome::FailedToStart(err)
            | TokioRuntimeOutcome::Panicked(err) => {
                let error_str = format!("{err:#}");
                let bin_name = get_current_bin_name();
                config
                    .get_user_feedback_channel()
                    .eprintln(format_upgrade_failed_retry_msg(&bin_name, &error_str));

                ImmediateUpgradeOutcome::CheckFailed(fallback_checking.into_check_failed(
                    ManifestCheckError::RuntimeOrThreadInitFailed { reason: error_str },
                ))
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

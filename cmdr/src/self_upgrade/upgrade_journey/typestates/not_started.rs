// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{DeferredCheckHandle, DeferredUpgradeOutcome,
                    ImmediateUpgradeOutcome, StartOutcome,
                    per_process_start_latch::{PerProcessStartLatch, StartClaimStatus},
                    thread_runner::ThreadRunner},
            CheckFailedError, Checking, Disabled, ImmediateCheckOutcome, Skipped,
            StateMarker};
use crate::{DEBUG_SELF_UPGRADE, GateStatus, UpgradeActionOnExit, UpgradeConfig,
            UpgradeJourney, UpgradePolicy, UpgradeStrategy,
            format_already_up_to_date_msg, format_upgrade_failed_retry_msg,
            get_current_bin_name};

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
///    • Claims PerProcessStartLatch (PER_PROCESS_START_LATCH: false -> true)
///    • If 1h cooldown active:
///      returns DeferredUpgrade(Skipped)
///    • Otherwise: transitions to Checking
///      and spawns "self-upgrade" via
///      StdThreadTokioRunner::run_non_blocking,
///      returning DeferredUpgrade(
///      DeferredCheckSpawned(handle)) ────────────► 2. Calls checking
///    │                                                 .check_for_deferred_upgrade()
///    ▼                                                 .await (2s timeout)
///    App owns DeferredUpgradeOutcome                   and returns DeferredCheckOutcome:
///    (normal TUI / CLI work runs)                      UpToDate | UpgradeAvailable | CheckFailed
///    │                                                 │
///    │                                                 │
///    ▼                                                 │
/// 3. At exit, consumes DeferredUpgradeOutcome via      │
///    deferred.handle_upgrade_on_exit()                 │
///    • Joins owned DeferredCheckHandle ◄───────────────┘
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
/// 1. Claims PerProcessStartLatch (PER_PROCESS_START_LATCH: false -> true)
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
/// [`StartOutcome::ImmediateUpgrade`]:
///     crate::self_upgrade::StartOutcome::ImmediateUpgrade
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotStarted;
impl StateMarker for NotStarted {}

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
    /// Atomically claims [`PerProcessStartLatch`] and dispatches based on
    /// [`UpgradeConfig::get_upgrade_policy()`] as follows:
    ///
    /// 1. **Duplicate `.start()` calls**:
    ///    - Leaves [`PerProcessStartLatch`] claimed, spawns zero threads, logs via
    ///      [`DEBUG_SELF_UPGRADE`], and returns [`StartOutcome::AlreadyStarted`].
    ///
    /// 2. **Immediate upgrade ([`UpgradeStrategy::Immediate`], `--upgrade`)**:
    ///    - Checks the manifest (`5s` timeout, bypassing the `1h` cooldown) and runs
    ///      [`perform_upgrade`] on the current thread if a newer version exists,
    ///      returning [`StartOutcome::ImmediateUpgrade`].
    ///
    /// 3. **Deferred upgrade on exit ([`UpgradeStrategy::Deferred`],
    ///    `--upgrade-behavior`)**:
    ///    - Checks the `1h` cooldown timestamp file via
    ///      [`RemoteVersionCheckGate::try_check_gate_status`][status] (failing open and
    ///      logging under [`DEBUG_SELF_UPGRADE`] on unexpected filesystem or clock
    ///      errors); if [`GateStatus::ClosedForCooldown`], returns
    ///      [`DeferredUpgradeOutcome::Skipped`], or if [`GateStatus::Open`], spawns the
    ///      dedicated `"self-upgrade"` thread via
    ///      [`StdThreadTokioRunner::run_non_blocking`][3] and returns its owned
    ///      [`DeferredCheckHandle`] inside
    ///      [`DeferredUpgradeOutcome::DeferredCheckSpawned`] for
    ///      [`DeferredUpgradeOutcome::handle_upgrade_on_exit`][1].
    ///
    /// 4. **Disabled ([`UpgradePolicy::Disabled`], `--upgrade-behavior disable`)**:
    ///    - Transitions to [`UpgradeJourney<Disabled>`] with zero threads and zero I/O,
    ///      returning [`StartOutcome::Disabled`].
    ///
    /// See [Thread Model] on [`NotStarted`] for the thread-by-thread state transition
    /// diagram, and [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
    /// [`self_upgrade`] module documentation.
    ///
    /// [1]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
    /// [3]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    /// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`UpgradeJourney<Disabled>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
    /// [status]: crate::self_upgrade::RemoteVersionCheckGate::try_check_gate_status
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    pub fn start(self) -> StartOutcome {
        // Early return if another instance is already running.
        if PerProcessStartLatch::claim() == StartClaimStatus::AlreadyStarted {
            return log_duplicate_start(&self.config);
        }

        let upgrade_policy = self.config.get_upgrade_policy();
        match upgrade_policy {
            // Upgrade Disabled.
            UpgradePolicy::Disabled => StartOutcome::Disabled(self.into_disabled()),

            // Enabled - Immediate Upgrade.
            UpgradePolicy::Enabled(strategy @ UpgradeStrategy::Immediate) => {
                let checking = self.into_checking(strategy);
                StartOutcome::ImmediateUpgrade(Self::run_immediate_upgrade(checking))
            }

            // Enabled - Deferred Upgrade.
            UpgradePolicy::Enabled(
                strategy @ UpgradeStrategy::Deferred(action_on_exit),
            ) => match self.check_gate_status_fail_open() {
                GateStatus::ClosedForCooldown => StartOutcome::DeferredUpgrade(
                    DeferredUpgradeOutcome::Skipped(self.into_skipped()),
                ),
                GateStatus::Open => {
                    let checking = self.into_checking(strategy);
                    StartOutcome::DeferredUpgrade(Self::run_deferred_upgrade(
                        checking,
                        action_on_exit,
                    ))
                }
            },
        }
    }

    /// Evaluates [`RemoteVersionCheckGate::try_check_gate_status`][1] with a fail-open
    /// Control Plane policy: any [`GateStatusError`][2] (missing timestamp file on first
    /// run, filesystem error, or clock skew) is logged under [`DEBUG_SELF_UPGRADE`] and
    /// treated as [`GateStatus::Open`] so upgrade checks are never silently suppressed.
    ///
    /// [1]: crate::self_upgrade::RemoteVersionCheckGate::try_check_gate_status
    /// [2]: crate::self_upgrade::GateStatusError
    fn check_gate_status_fail_open(&self) -> GateStatus {
        match self
            .config
            .get_remote_version_check_gate()
            .try_check_gate_status()
        {
            Ok(status) => status,
            Err(err) => {
                DEBUG_SELF_UPGRADE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Could not check cooldown gate status; failing open",
                        crate_name = %self.config.get_crate_name(),
                        error = ?err
                    );
                });
                GateStatus::Open
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
    pub(in crate::self_upgrade) fn into_skipped(self) -> UpgradeJourney<Skipped> {
        UpgradeJourney {
            config: self.config,
            state: Skipped,
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

        match ThreadRunner::run_blocking(
            &config,
            async move || -> ImmediateUpgradeOutcome {
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
                                &up_to_date
                                    .get_config()
                                    .get_current_version()
                                    .to_string(),
                            ),
                        );
                        ImmediateUpgradeOutcome::UpToDate(up_to_date)
                    }
                    ImmediateCheckOutcome::Upgrading(upgrading) => {
                        let completed = upgrading.execute_immediate_upgrade().await;
                        ImmediateUpgradeOutcome::Completed(completed)
                    }
                }
            },
        ) {
            Ok(outcome) => outcome,
            Err(thread_err) => {
                let error_str = thread_err.to_string();
                let bin_name = get_current_bin_name();
                config
                    .get_user_feedback_channel()
                    .eprintln(format_upgrade_failed_retry_msg(&bin_name, &error_str));

                ImmediateUpgradeOutcome::CheckFailed(
                    fallback_checking
                        .into_check_failed(CheckFailedError::from(thread_err)),
                )
            }
        }
    }

    fn run_deferred_upgrade(
        checking: UpgradeJourney<Checking>,
        action_on_exit: UpgradeActionOnExit,
    ) -> DeferredUpgradeOutcome {
        let spawn_checking = checking.clone();
        match ThreadRunner::run_non_blocking(checking.get_config(), async move || {
            spawn_checking
                .check_for_deferred_upgrade(action_on_exit)
                .await
        }) {
            Ok(join_handle) => {
                DeferredUpgradeOutcome::DeferredCheckSpawned(DeferredCheckHandle {
                    checking,
                    join_handle,
                })
            }
            Err(thread_err) => DeferredUpgradeOutcome::DeferredCheckSpawnFailed(
                checking.into_check_failed(CheckFailedError::from(thread_err)),
            ),
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

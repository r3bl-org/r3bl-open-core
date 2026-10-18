// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Checking, Completed, CooldownSkipped, Disabled, UpToDate,
            UpgradeAvailable, UpgradeJourney, Upgrading};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex,
               scoped_mutex};
use std::{sync::LazyLock, thread::JoinHandle};

/// Internal state protected by `SINGLETON`, pairing the [`UpgradeCheckStatus`] snapshot
/// with the non-`Clone` [`JoinHandle`] of the dedicated version-check thread (when one
/// was spawned by [`UpgradeJourney::start`]).
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Default)]
struct UpgradeSingletonState {
    status: UpgradeCheckStatus,
    thread_check_handle: Option<JoinHandle<()>>,
}

/// Process-wide singleton instance of [`UpgradeSingletonState`], lazily initialized on
/// first access and protected by a [`ScopedMutex`] with [`PanicOnSpecificLockNesting`].
static SINGLETON: LazyLock<
    ScopedMutex<UpgradeSingletonState, { PanicOnSpecificLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(SPECIFIC, UpgradeSingletonState::default()));

/// Process-wide lifecycle status singleton for the [`self_upgrade`] engine.
///
/// Starts at [`UpgradeCheckStatus::NotStarted`] and tracks the active
/// [`UpgradeJourney<State>`] typestate across all [`UpgradePolicy`] modes:
/// - **[`UpgradeStrategy::Immediate`] (`--upgrade`)**: [`NotStarted`] -> [`Checking`] ->
///   [`UpToDate`], [`CheckFailed`], or [`Upgrading`] -> [`Completed`].
/// - **[`UpgradeStrategy::DedicatedThread`] (`--upgrade-behavior`)**: [`NotStarted`] ->
///   [`CooldownSkipped`] or [`Checking`] -> [`UpToDate`], [`CheckFailed`], or
///   [`UpgradeAvailable`] -> [`Upgrading`] -> [`Completed`].
/// - **[`UpgradePolicy::Disabled`] (`--upgrade-behavior disable`)**: [`NotStarted`] ->
///   [`Disabled`].
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation for the complete lifecycle diagram.
///
/// # Singleton & Exit Join Coordination
///
/// The active status and optional dedicated check thread `JoinHandle<()>` are stored
/// together inside a module-private `SINGLETON` static
/// (`LazyLock<ScopedMutex<UpgradeSingletonState, ...>>`).
///
/// At application shutdown, [`UpgradeCheckStatus::handle_upgrade_on_exit`]:
/// 1. Atomically takes `thread_check_handle.take()` and **releases `SINGLETON`** before
///    calling `handle.join()`, preventing lock-order deadlock and ensuring short-lived
///    CLI invocations wait up to [`DEDICATED_THREAD_MANIFEST_TIMEOUT`] (`2s`) when the
///    `1h` `mtime` cooldown was expired.
/// 2. Atomically transitions [`UpgradeCheckStatus::UpgradeAvailable`] into
///    [`UpgradeCheckStatus::Upgrading`] inside `SINGLETON.write` before releasing the
///    lock to execute the exit action, guaranteeing one-shot idempotent exit handling.
///
/// # Deadlock Prevention Policy: [`PanicOnSpecificLockNesting`] ([`SPECIFIC`])
///
/// `SINGLETON` is strictly a leaf lock: its `read` and `write` closures only perform
/// memory moves/clones and never acquire other locks, perform I/O, or join threads. We
/// use [`SPECIFIC`] rather than `ANY` because caller threads may already hold
/// component-level locks (such as [`OutputDevice`] or [`Readline`]) when querying the
/// upgrade status during UI drawing or application shutdown.
///
/// [`CheckFailed`]: Self::CheckFailed
/// [`Checking`]: Self::Checking
/// [`Completed`]: Self::Completed
/// [`CooldownSkipped`]: Self::CooldownSkipped
/// [`DEDICATED_THREAD_MANIFEST_TIMEOUT`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
/// [`Disabled`]: Self::Disabled
/// [`NotStarted`]: Self::NotStarted
/// [`OutputDevice`]: r3bl_tui::OutputDevice
/// [`PanicOnSpecificLockNesting`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`Readline`]: r3bl_tui::Readline
/// [`ScopedMutex`]: r3bl_tui::ScopedMutex
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`SPECIFIC`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`UpgradeAvailable`]: Self::UpgradeAvailable
/// [`UpgradeCheckStatus::handle_upgrade_on_exit`]: Self::handle_upgrade_on_exit
/// [`UpgradeCheckStatus::NotStarted`]: Self::NotStarted
/// [`UpgradeCheckStatus::UpgradeAvailable`]: Self::UpgradeAvailable
/// [`UpgradeCheckStatus::Upgrading`]: Self::Upgrading
/// [`UpgradeJourney<State>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Upgrading`]: Self::Upgrading
/// [`UpToDate`]: Self::UpToDate
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpgradeCheckStatus {
    /// [`UpgradeJourney::start`] has not yet been called in this process.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[default]
    NotStarted,

    /// Upgrade checks were explicitly bypassed via [`UpgradePolicy::Disabled`]. Carries
    /// [`UpgradeJourney<Disabled>`].
    ///
    /// [`Disabled`]: crate::self_upgrade::Disabled
    /// [`UpgradeJourney<Disabled>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    Disabled(UpgradeJourney<Disabled>),

    /// Dedicated-thread check was skipped because the per-crate `1h` `mtime` cooldown
    /// timestamp file is still fresh. Carries [`UpgradeJourney<CooldownSkipped>`].
    ///
    /// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
    /// [`UpgradeJourney<CooldownSkipped>`]: crate::self_upgrade::UpgradeJourney
    CooldownSkipped(UpgradeJourney<CooldownSkipped>),

    /// Manifest version check is currently in flight. Carries
    /// [`UpgradeJourney<Checking>`].
    ///
    /// [`Checking`]: crate::self_upgrade::Checking
    /// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
    Checking(UpgradeJourney<Checking>),

    /// Local binary version is greater than or equal to the remote
    /// [`R3BL_VERSIONS_JSON_URL`] version. Carries [`UpgradeJourney<UpToDate>`].
    ///
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
    /// [`UpgradeJourney<UpToDate>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    UpToDate(UpgradeJourney<UpToDate>),

    /// Dedicated-thread check found a strictly newer version in
    /// [`R3BL_VERSIONS_JSON_URL`] awaiting exit handling. Carries
    /// [`UpgradeJourney<UpgradeAvailable>`].
    ///
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
    /// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
    /// [`UpgradeJourney<UpgradeAvailable>`]: crate::self_upgrade::UpgradeJourney
    UpgradeAvailable(UpgradeJourney<UpgradeAvailable>),

    /// Network, parse, or thread initialization error occurred during the manifest
    /// check. Carries [`UpgradeJourney<CheckFailed>`].
    ///
    /// [`CheckFailed`]: crate::self_upgrade::CheckFailed
    /// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Immediate upgrade (`--upgrade`) or exit upgrade action (`Auto`,
    /// `PromptToUpgrade`, or `NotifyOnly`) is actively executing with `SINGLETON`
    /// released. Carries [`UpgradeJourney<Upgrading>`].
    ///
    /// [`UpgradeJourney<Upgrading>`]: crate::self_upgrade::UpgradeJourney
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    Upgrading(UpgradeJourney<Upgrading>),

    /// Upgrade or exit action finished with a terminal [`UpgradeResolution`]. Carries
    /// [`UpgradeJourney<Completed>`].
    ///
    /// [`Completed`]: crate::self_upgrade::Completed
    /// [`UpgradeJourney<Completed>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpgradeResolution`]: crate::self_upgrade::UpgradeResolution
    Completed(UpgradeJourney<Completed>),
}

impl UpgradeCheckStatus {
    /// Returns a cloned snapshot of the current [`UpgradeCheckStatus`] from the
    /// process-wide `SINGLETON`.
    #[must_use]
    pub fn get() -> Self { SINGLETON.read(|state| state.status.clone()) }

    /// Returns `true` if `SINGLETON` is still at [`Self::NotStarted`].
    #[must_use]
    pub(super) fn is_not_started() -> bool {
        SINGLETON.read(|state| matches!(state.status, Self::NotStarted))
    }

    /// Atomically claims `SINGLETON` if and only if its current status is
    /// [`Self::NotStarted`], replacing it with `initial_status` and returning `true`.
    ///
    /// Returns `false` without mutating `SINGLETON` if [`UpgradeJourney::start`] was
    /// already called earlier in this process.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[must_use]
    pub(super) fn try_claim_not_started(initial_status: Self) -> bool {
        SINGLETON.write(|state| {
            if matches!(state.status, Self::NotStarted) {
                state.status = initial_status;
                true
            } else {
                false
            }
        })
    }

    /// Writes `status` into `SINGLETON` without altering `thread_check_handle`.
    pub(super) fn set(status: Self) {
        SINGLETON.write(|state| {
            state.status = status;
        });
    }

    /// Stores the [`JoinHandle`] of the dedicated version-check thread without altering
    /// `status` (so a fast check thread that already wrote its terminal status before
    /// this call is never overwritten).
    pub(super) fn set_thread_check_handle(handle: JoinHandle<()>) {
        SINGLETON.write(|state| {
            state.thread_check_handle = Some(handle);
        });
    }

    /// Joins any in-flight dedicated version-check thread and executes the configured
    /// exit upgrade action if [`Self::get`] resolves to [`Self::UpgradeAvailable`].
    ///
    /// Works in both async and sync contexts:
    /// 1. Takes `thread_check_handle.take()` inside `SINGLETON.write` and drops the lock
    ///    **before** calling `handle.join()`, preventing deadlock when the check thread
    ///    writes its terminal status to `SINGLETON`.
    /// 2. Atomically transitions [`Self::UpgradeAvailable`] into [`Self::Upgrading`]
    ///    inside `SINGLETON.write` before executing the exit action, ensuring idempotent
    ///    one-shot behavior if called more than once.
    /// 3. Stores [`Self::Completed`] once the exit action finishes.
    pub fn handle_upgrade_on_exit() {
        // Step 1: Loud lock release before joining the dedicated check thread.
        let maybe_handle = SINGLETON.write(|state| state.thread_check_handle.take());
        if let Some(handle) = maybe_handle
            && let Err(panic_payload) = handle.join()
        {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade check thread panicked outside catch_unwind",
                    panic = ?panic_payload
                );
            });
        }

        // Step 2: Atomically claim `UpgradeAvailable -> Upgrading` inside leaf lock.
        let maybe_upgrading = SINGLETON.write(|state| {
            if let Self::UpgradeAvailable(ref available) = state.status {
                let action_on_exit = available.state().action_on_exit;
                let upgrading = available.clone().into_upgrading();
                state.status = Self::Upgrading(upgrading.clone());
                Some((upgrading, action_on_exit))
            } else {
                None
            }
        });

        // Step 3: Execute exit action outside lock and record terminal `Completed`.
        if let Some((upgrading, action_on_exit)) = maybe_upgrading {
            let completed = upgrading.execute_action_on_exit(action_on_exit);
            Self::set(Self::Completed(completed));
        }
    }

    /// Drains any in-flight dedicated check thread and resets `SINGLETON` back to
    /// [`Self::NotStarted`] with `thread_check_handle: None`, recovering cleanly even if
    /// a prior test poisoned the mutex.
    #[cfg(test)]
    pub(crate) fn reset_for_testing() {
        let maybe_handle =
            SINGLETON.lock_raw_poison_safe(|state| state.thread_check_handle.take());
        if let Some(handle) = maybe_handle {
            drop(handle.join());
        }
        SINGLETON.lock_raw_poison_safe(|state| {
            *state = UpgradeSingletonState::default();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{CooldownPolicy, CrateDistribution, CrateIdentity,
                        CrateMetadata, CrateReleaseDate, CrateVersion, NotStarted,
                        UpgradeActionOnExit, UpgradeConfig, UpgradePolicy,
                        UpgradeResolution, UpgradeStrategy, UserFeedbackChannel},
                *};
    use r3bl_tui::assert_eq2;
    use serial_test::serial;
    use std::time::Duration;

    fn make_test_metadata(version_str: &str) -> CrateMetadata {
        CrateMetadata {
            version: CrateVersion::try_from(version_str).unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: CrateReleaseDate::now(),
        }
    }

    fn make_test_journey(
        version_str: &str,
        policy: UpgradePolicy,
    ) -> UpgradeJourney<NotStarted> {
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from(version_str).unwrap()),
            UserFeedbackChannel::Quiet,
            policy,
            None,
            None,
            CooldownPolicy::Disabled,
        );
        UpgradeJourney::new(config)
    }

    #[test]
    #[serial]
    fn test_upgrade_check_status_all_nine_variants_and_single_claim() {
        UpgradeCheckStatus::reset_for_testing();

        // 1. NotStarted + atomic claim behavior.
        assert!(UpgradeCheckStatus::is_not_started());
        assert_eq2!(UpgradeCheckStatus::get(), UpgradeCheckStatus::NotStarted);

        // 2. Disabled claims NotStarted; second claim fails without mutating SINGLETON.
        let disabled_journey =
            make_test_journey("0.1.0", UpgradePolicy::Disabled).into_disabled();
        assert!(UpgradeCheckStatus::try_claim_not_started(
            UpgradeCheckStatus::Disabled(disabled_journey.clone())
        ));
        assert!(!UpgradeCheckStatus::is_not_started());
        assert!(!UpgradeCheckStatus::try_claim_not_started(
            UpgradeCheckStatus::NotStarted
        ));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Disabled(disabled_journey)
        );

        // 3. CooldownSkipped.
        let skipped_journey =
            make_test_journey("0.1.0", UpgradePolicy::default()).into_cooldown_skipped();
        UpgradeCheckStatus::set(UpgradeCheckStatus::CooldownSkipped(
            skipped_journey.clone(),
        ));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::CooldownSkipped(skipped_journey)
        );

        // 4. Checking.
        let checking_journey = make_test_journey("0.1.0", UpgradePolicy::default())
            .into_checking(UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto));
        UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(checking_journey.clone()));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Checking(checking_journey.clone())
        );

        // 5. UpToDate.
        let meta_up_to_date = make_test_metadata("0.1.0");
        let up_to_date_journey =
            checking_journey.clone().into_up_to_date(meta_up_to_date);
        UpgradeCheckStatus::set(UpgradeCheckStatus::UpToDate(up_to_date_journey.clone()));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpToDate(up_to_date_journey)
        );

        // 6. UpgradeAvailable.
        let meta_upgrade = make_test_metadata("0.2.0");
        let available_journey = checking_journey
            .clone()
            .into_upgrade_available(UpgradeActionOnExit::NotifyOnly, meta_upgrade);
        assert_eq2!(
            available_journey.state().action_on_exit,
            UpgradeActionOnExit::NotifyOnly
        );
        UpgradeCheckStatus::set(UpgradeCheckStatus::UpgradeAvailable(
            available_journey.clone(),
        ));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpgradeAvailable(available_journey.clone())
        );

        // 7. CheckFailed.
        let failed_journey = checking_journey
            .into_check_failed("connection refused: 127.0.0.1".to_string());
        UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(failed_journey.clone()));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::CheckFailed(failed_journey)
        );

        // 8. Upgrading.
        let upgrading_journey = available_journey.into_upgrading();
        UpgradeCheckStatus::set(UpgradeCheckStatus::Upgrading(upgrading_journey.clone()));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Upgrading(upgrading_journey.clone())
        );

        // 9. Completed.
        let completed_journey =
            upgrading_journey.into_completed(UpgradeResolution::NotifiedOnly);
        UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(completed_journey.clone()));
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Completed(completed_journey)
        );
    }

    #[test]
    #[serial]
    fn test_reset_for_testing_drains_inflight_thread_before_reset() {
        UpgradeCheckStatus::reset_for_testing();

        let checking = make_test_journey("0.1.0", UpgradePolicy::default())
            .into_checking(UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto));
        UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(checking.clone()));

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(25));
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                checking.into_check_failed("late write".to_string()),
            ));
        });
        UpgradeCheckStatus::set_thread_check_handle(handle);

        // `reset_for_testing` must join `handle` first, then reset to `NotStarted`.
        UpgradeCheckStatus::reset_for_testing();
        assert_eq2!(UpgradeCheckStatus::get(), UpgradeCheckStatus::NotStarted);
    }

    #[test]
    #[serial]
    fn test_handle_upgrade_on_exit_upgrading_guard_and_idempotency() {
        UpgradeCheckStatus::reset_for_testing();

        // 1. If already `Upgrading`, `handle_upgrade_on_exit` is a safe one-shot no-op.
        let meta = make_test_metadata("0.2.0");
        let upgrading = make_test_journey("0.1.0", UpgradePolicy::default())
            .into_checking(UpgradeStrategy::Immediate)
            .into_upgrading(meta.clone());
        UpgradeCheckStatus::set(UpgradeCheckStatus::Upgrading(upgrading.clone()));
        UpgradeCheckStatus::handle_upgrade_on_exit();
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Upgrading(upgrading)
        );

        // 2. `UpgradeAvailable(NotifyOnly)` transitions to `Completed(NotifiedOnly)`
        // once, and subsequent calls remain `Completed(NotifiedOnly)`.
        let available = make_test_journey(
            "0.1.0",
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::NotifyOnly,
            )),
        )
        .into_checking(UpgradeStrategy::DedicatedThread(
            UpgradeActionOnExit::NotifyOnly,
        ))
        .into_upgrade_available(UpgradeActionOnExit::NotifyOnly, meta.clone());
        let expected_completed = available
            .clone()
            .into_upgrading()
            .into_completed(UpgradeResolution::NotifiedOnly);

        UpgradeCheckStatus::set(UpgradeCheckStatus::UpgradeAvailable(available));
        UpgradeCheckStatus::handle_upgrade_on_exit();
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Completed(expected_completed.clone())
        );

        UpgradeCheckStatus::handle_upgrade_on_exit();
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Completed(expected_completed)
        );
    }
}

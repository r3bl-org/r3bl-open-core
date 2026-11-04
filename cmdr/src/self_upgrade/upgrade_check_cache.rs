// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Checking, Completed, CooldownSkipped, Disabled, UpToDate,
            UpgradeAvailable, UpgradeJourney, Upgrading};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex,
               scoped_mutex};
use std::{sync::LazyLock, thread::JoinHandle};

/// Process-wide lifecycle status for the [`self_upgrade`] engine, stored in
/// [`UpgradeSingletonState::status`] inside [`SINGLETON`]. This type is part of the
/// [Control Plane].
///
/// Starts at [`UpgradeCheckStatus::NotStarted`] and tracks the active
/// [`UpgradeJourney<State>`] typestate across all [`UpgradePolicy`] modes:
/// - **Immediate upgrade ([`UpgradeStrategy::Immediate`], `--upgrade`)**: [`NotStarted`]
///   -> [`Checking`] -> [`UpToDate`], [`CheckFailed`], or [`Upgrading`] -> [`Completed`].
/// - **Deferred upgrade on exit ([`UpgradeStrategy::DedicatedThread`],
///   `--upgrade-behavior`)**: [`NotStarted`] -> [`CooldownSkipped`] or [`Checking`] ->
///   [`UpToDate`], [`CheckFailed`], or [`UpgradeAvailable`] -> [`Upgrading`] ->
///   [`Completed`].
/// - **Disabled ([`UpgradePolicy::Disabled`], `--upgrade-behavior disable`)**:
///   [`NotStarted`] -> [`Disabled`].
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation for the complete lifecycle diagram,
/// [`UpgradeJourney::start`] for startup transitions, and
/// [`Self::handle_upgrade_on_exit`] for shutdown join and exit handling.
///
/// # Deadlock Prevention Policy: [`PanicOnSpecificLockNesting`] ([`SPECIFIC`])
///
/// [`SINGLETON`] is strictly a leaf lock: its `read` and `write` closures only perform
/// memory moves/clones and never acquire other locks, perform I/O, or join threads. We
/// use [`SPECIFIC`] rather than `ANY` because caller threads may already hold
/// component-level locks (such as [`OutputDevice`] or [`Readline`]) when querying the
/// upgrade status during UI drawing or application shutdown.
///
/// [`CheckFailed`]: Self::CheckFailed
/// [`Checking`]: Self::Checking
/// [`Completed`]: Self::Completed
/// [`CooldownSkipped`]: Self::CooldownSkipped
/// [`Disabled`]: Self::Disabled
/// [`NotStarted`]: Self::NotStarted
/// [`OutputDevice`]: r3bl_tui::OutputDevice
/// [`PanicOnSpecificLockNesting`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`Readline`]: r3bl_tui::Readline
/// [`ScopedMutex`]: r3bl_tui::ScopedMutex
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`SPECIFIC`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`UpgradeAvailable`]: Self::UpgradeAvailable
/// [`UpgradeCheckStatus::NotStarted`]: Self::NotStarted
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney<State>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Upgrading`]: Self::Upgrading
/// [`UpToDate`]: Self::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
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
    /// [`R3BL_VERSIONS_JSONC_URL`] version. Carries [`UpgradeJourney<UpToDate>`].
    ///
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    /// [`UpgradeJourney<UpToDate>`]: crate::self_upgrade::UpgradeJourney
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    UpToDate(UpgradeJourney<UpToDate>),

    /// Dedicated-thread check found a strictly newer version in
    /// [`R3BL_VERSIONS_JSONC_URL`] awaiting exit handling. Carries
    /// [`UpgradeJourney<UpgradeAvailable>`].
    ///
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
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
    /// `PromptToUpgrade`, or `NotifyOnly`) is actively executing with [`SINGLETON`]
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
    /// Writes `upgrade_singleton_state` into [`SINGLETON`]
    /// [`UpgradeSingletonState::status`] field without altering
    /// [`UpgradeSingletonState::thread_check_handle`] field.
    ///
    /// Called only by the two [Control Plane] entry points to commit state transitions
    /// after [`Self::try_claim_not_started`]:
    /// - **During a deferred upgrade on exit ([`UpgradeStrategy::DedicatedThread`][1])**:
    ///   [`UpgradeJourney::start`], in [`NotStarted`], spawns a dedicated check thread
    ///   which calls [`Self::set`] once the manifest check finishes. The main (current)
    ///   thread continues running the application and at some later time calls
    ///   [`Self::handle_upgrade_on_exit`] at shutdown (which calls [`Self::set`]).
    /// - **During an immediate upgrade ([`UpgradeStrategy::Immediate`][2],
    ///   `--upgrade`)**: No dedicated thread is spawned; [`UpgradeJourney::start`] runs
    ///   the entire check and installation on the current thread, calling [`Self::set`]
    ///   after each step before the binary exits.
    ///
    /// See [Thread Model] on [`NotStarted`] for the full thread-by-thread state
    /// transition diagram.
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    /// [2]: crate::self_upgrade::UpgradeStrategy::Immediate
    /// [`NotStarted`]: crate::self_upgrade::NotStarted
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    pub(in crate::self_upgrade) fn set(upgrade_singleton_state: Self) {
        SINGLETON.write(|state| {
            state.status = upgrade_singleton_state;
        });
    }

    /// Returns a cloned snapshot of the current [`UpgradeCheckStatus`] from the
    /// process-wide [`SINGLETON`].
    #[must_use]
    pub fn get() -> Self {
        SINGLETON.read(|upgrade_singleton_state| upgrade_singleton_state.status.clone())
    }

    /// Atomically claims [`SINGLETON`] if and only if its current status is
    /// [`Self::NotStarted`], replacing it with `initial_status` and returning `true`.
    ///
    /// # Preventing [`TOCTOU`] Race Conditions at Startup
    ///
    /// Checking [`Self::get`] and then calling [`Self::set`] in two separate lock
    /// acquisitions would create a Time-of-Check to Time-of-Use ([`TOCTOU`]) race if
    /// [`UpgradeJourney::start`] were called concurrently. Performing both the
    /// [`Self::NotStarted`] check and the assignment inside a single `SINGLETON.write`
    /// lock acquisition guarantees that at most one caller claims the engine, returning
    /// `false` to any duplicate caller without spawning extra threads or network calls.
    ///
    /// [`Self::NotStarted`]: Self::NotStarted
    /// [`TOCTOU`]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[must_use]
    pub(in crate::self_upgrade) fn try_claim_not_started(initial_status: Self) -> bool {
        SINGLETON.write(|state| {
            if matches!(state.status, Self::NotStarted) {
                state.status = initial_status;
                true
            } else {
                false
            }
        })
    }

    /// Stores the [`JoinHandle`] of the dedicated version-check thread into
    /// [`UpgradeSingletonState::thread_check_handle`] without altering
    /// [`UpgradeSingletonState::status`] (so a fast check thread that already wrote its
    /// terminal status before this call is never overwritten).
    ///
    /// Called on the current thread by [`UpgradeJourney::start`] during a deferred
    /// upgrade on exit ([`UpgradeStrategy::DedicatedThread`][1]).
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    pub(in crate::self_upgrade) fn set_thread_check_handle(handle: JoinHandle<()>) {
        SINGLETON.write(|state| {
            state.thread_check_handle = Some(handle);
        });
    }

    /// This method is called by `main()` at shutdown on the main thread to complete a
    /// deferred upgrade on exit ([`UpgradeStrategy::DedicatedThread`][1]). This is part
    /// of the [Control Plane].
    ///
    /// If [`UpgradeJourney::start`] spawned a dedicated check thread, the main thread
    /// first waits for that thread to finish, and then runs the configured
    /// [`UpgradeActionOnExit`] on the main thread if an upgrade is available
    /// ([`Self::UpgradeAvailable`]):
    /// 1. Takes [`UpgradeSingletonState::thread_check_handle`] inside `SINGLETON.write`
    ///    and drops the lock **before** calling `handle.join()`, preventing deadlock when
    ///    the check thread writes its terminal status to [`SINGLETON`].
    /// 2. Atomically transitions [`Self::UpgradeAvailable`] into [`Self::Upgrading`]
    ///    inside a single `SINGLETON.write` lock acquisition before releasing the lock,
    ///    preventing a [`TOCTOU`] race and guaranteeing one-shot idempotent behavior if
    ///    called more than once.
    /// 3. Delegates to `UpgradeJourney::<Upgrading>::execute_action_on_exit` outside the
    ///    lock (which invokes the [Execution Plane] to prompt or install the upgrade) and
    ///    commits the returned [`UpgradeJourney<Completed>`] via `Self::set`.
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    /// [`TOCTOU`]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
    /// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [`UpgradeJourney<Completed>`]: crate::self_upgrade::UpgradeJourney
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
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
                let action_on_exit = available.get_state().action_on_exit;
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

    /// Drains any in-flight dedicated check thread and resets [`SINGLETON`] back to
    /// [`UpgradeSingletonState::new`] ([`Self::NotStarted`] with
    /// [`UpgradeSingletonState::thread_check_handle`] set to `None`).
    #[cfg(test)]
    pub(crate) fn reset_for_testing() {
        let maybe_handle = SINGLETON.write(|state| state.thread_check_handle.take());
        if let Some(handle) = maybe_handle {
            drop(handle.join());
        }
        SINGLETON.write(|state| {
            *state = UpgradeSingletonState::new();
        });
    }
}

/// Pairs the [`UpgradeCheckStatus`] with the [`JoinHandle`] of the dedicated check thread
/// (when one is spawned by [`UpgradeJourney::start`]). [`SINGLETON`] makes this struct a
/// global process singleton.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug)]
pub struct UpgradeSingletonState {
    /// Current [`UpgradeCheckStatus`] of the [`self_upgrade`] engine.
    ///
    /// [`self_upgrade`]: mod@crate::self_upgrade
    pub status: UpgradeCheckStatus,

    /// [`JoinHandle`] of the dedicated check thread spawned by [`UpgradeJourney::start`]
    /// during a deferred upgrade on exit ([`UpgradeStrategy::DedicatedThread`][1]), or
    /// `None` if no thread was spawned or it has already been joined by
    /// [`UpgradeCheckStatus::handle_upgrade_on_exit`].
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    pub thread_check_handle: Option<JoinHandle<()>>,
}

impl UpgradeSingletonState {
    const fn new() -> Self {
        Self {
            status: UpgradeCheckStatus::NotStarted,
            thread_check_handle: None,
        }
    }
}

/// Process-wide singleton instance of [`UpgradeSingletonState`], lazily initialized on
/// first access and protected by a [`ScopedMutex`] with [`PanicOnSpecificLockNesting`].
pub static SINGLETON: LazyLock<
    ScopedMutex<UpgradeSingletonState, { PanicOnSpecificLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(SPECIFIC, UpgradeSingletonState::new()));

#[cfg(test)]
mod tests {
    use super::{super::{CooldownPolicy, CrateDistribution, CrateMetadata,
                        CrateReleaseDate, CrateVersion, EndpointUrl, JsoncScalar,
                        ManifestCheckError, NotStarted, UpgradeActionOnExit,
                        UpgradeConfig, UpgradePolicy, UpgradeResolution,
                        UpgradeStrategy, UserFeedbackChannel},
                *};
    use r3bl_tui::assert_eq2;
    use serial_test::serial;
    use std::time::Duration;

    fn make_test_metadata(version_str: &str) -> CrateMetadata {
        CrateMetadata {
            version: CrateVersion::try_parse(version_str).unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: CrateReleaseDate::now(),
        }
    }

    fn make_test_journey(
        version_str: &str,
        policy: UpgradePolicy,
    ) -> UpgradeJourney<NotStarted> {
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse(version_str).unwrap(),
            UserFeedbackChannel::Quiet,
            policy,
            EndpointUrl::default_manifest_url(),
            CooldownPolicy::Disabled,
        );
        UpgradeJourney::new(config)
    }

    #[test]
    #[serial]
    fn test_upgrade_check_status_all_nine_variants_and_single_claim() {
        UpgradeCheckStatus::reset_for_testing();

        // 1. NotStarted + atomic claim behavior.
        assert_eq2!(UpgradeCheckStatus::get(), UpgradeCheckStatus::NotStarted);

        // 2. Disabled claims NotStarted; second claim fails without mutating SINGLETON.
        let disabled_journey =
            make_test_journey("0.1.0", UpgradePolicy::Disabled).into_disabled();
        assert!(UpgradeCheckStatus::try_claim_not_started(
            UpgradeCheckStatus::Disabled(disabled_journey.clone())
        ));
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
        let available_journey = checking_journey.clone().into_upgrade_available(
            UpgradeActionOnExit::NotifyOnly,
            meta_upgrade,
            EndpointUrl::try_parse("https://example.com/releases/download").unwrap(),
        );
        assert_eq2!(
            available_journey.get_state().action_on_exit,
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
        let failed_journey =
            checking_journey.into_check_failed(ManifestCheckError::FetchOrParseFailed {
                diagnostic: "connection refused: 127.0.0.1".to_string(),
            });
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
                checking.into_check_failed(ManifestCheckError::FetchOrParseFailed {
                    diagnostic: "late write".to_string(),
                }),
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
            .into_upgrading(
                meta.clone(),
                EndpointUrl::try_parse("https://example.com/releases/download").unwrap(),
            );
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
        .into_upgrade_available(
            UpgradeActionOnExit::NotifyOnly,
            meta.clone(),
            EndpointUrl::try_parse("https://example.com/releases/download").unwrap(),
        );
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

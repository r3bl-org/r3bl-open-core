// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Completed, DeferredCheckOutcome, UpToDate, UpgradeJourney};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex,
               scoped_mutex};
use std::{sync::LazyLock, thread::JoinHandle};

/// Process-wide singleton instance of [`UpgradeSingletonState`], lazily initialized on
/// first access and protected by a [`ScopedMutex`] with [`PanicOnSpecificLockNesting`].
/// This static is part of the [Control Plane] (see [Thread Model]).
///
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
pub static SINGLETON: LazyLock<
    ScopedMutex<UpgradeSingletonState, { PanicOnSpecificLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(SPECIFIC, UpgradeSingletonState::new()));

/// Process-wide state stored inside [`SINGLETON`], pairing the single-start witness
/// ([`SingletonStartStatus`]) with the [`JoinHandle<DeferredCheckOutcome>`][jh] of the
/// dedicated thread (when spawned by [`UpgradeJourney::start`]). This type is part of the
/// [Control Plane] (see [Thread Model]).
///
/// # Deadlock Prevention Policy: [`PanicOnSpecificLockNesting`] ([`SPECIFIC`])
///
/// [`SINGLETON`] is strictly a leaf lock: its `read` and `write` closures only perform
/// memory moves and never acquire other locks, perform I/O, or join threads. We use
/// [`SPECIFIC`] rather than `ANY` because caller threads may already hold component-level
/// locks (such as [`OutputDevice`] or [`Readline`]) during UI drawing or application
/// shutdown.
///
/// [`OutputDevice`]: r3bl_tui::OutputDevice
/// [`PanicOnSpecificLockNesting`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`Readline`]: r3bl_tui::Readline
/// [`ScopedMutex`]: r3bl_tui::ScopedMutex
/// [`SPECIFIC`]: r3bl_tui::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [jh]: std::thread::JoinHandle
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug, Default)]
pub struct UpgradeSingletonState {
    /// Single-start witness tracking whether
    /// [`UpgradeJourney::<NotStarted>::start`][start] has claimed [`SINGLETON`] in this
    /// process.
    ///
    /// [start]: crate::self_upgrade::UpgradeJourney::start
    pub start_status: SingletonStartStatus,

    /// [`JoinHandle`] of the dedicated thread spawned by [`UpgradeJourney::start`]
    /// during a deferred upgrade on exit ([`UpgradeStrategy::Deferred`][1]),
    /// returning [`DeferredCheckOutcome`] when joined by [`handle_upgrade_on_exit`], or
    /// `None` if no thread was spawned or it has already been joined.
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    pub deferred_check_handle: Option<JoinHandle<DeferredCheckOutcome>>,
}

impl UpgradeSingletonState {
    const fn new() -> Self {
        Self {
            start_status: SingletonStartStatus::NotStarted,
            deferred_check_handle: None,
        }
    }

    /// Atomically transitions [`Self::start_status`] to
    /// [`SingletonStartStatus::AlreadyStarted`] and returns the prior
    /// [`SingletonStartStatus`].
    ///
    /// # Preventing [`TOCTOU`] Race Conditions at Startup
    ///
    /// Performing [`std::mem::replace`] inside a single `SINGLETON.write` lock
    /// acquisition guarantees that at most one caller observes
    /// [`SingletonStartStatus::NotStarted`], while any duplicate caller observes
    /// [`SingletonStartStatus::AlreadyStarted`] without spawning extra threads or network
    /// calls.
    ///
    /// [`TOCTOU`]: https://en.wikipedia.org/wiki/Time-of-check-to-time-of-use
    #[must_use]
    pub(in crate::self_upgrade) fn try_claim_start() -> SingletonStartStatus {
        SINGLETON.write(|state| {
            std::mem::replace(
                &mut state.start_status,
                SingletonStartStatus::AlreadyStarted,
            )
        })
    }

    /// Stores the [`JoinHandle<DeferredCheckOutcome>`][jh] of the dedicated thread into
    /// [`Self::deferred_check_handle`].
    ///
    /// Called on the current thread by [`UpgradeJourney::start`] (in the [Control Plane])
    /// during a deferred upgrade on exit ([`UpgradeStrategy::Deferred`][1]; see
    /// [Thread Model]).
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [jh]: std::thread::JoinHandle
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    pub(in crate::self_upgrade) fn set_deferred_check_handle(
        handle: JoinHandle<DeferredCheckOutcome>,
    ) {
        SINGLETON.write(|state| {
            state.deferred_check_handle = Some(handle);
        });
    }

    /// Drains any in-flight dedicated thread and resets [`SINGLETON`] back to
    /// [`Self::new`] (`start_status: NotStarted`, `deferred_check_handle: None`).
    #[cfg(test)]
    pub(crate) fn reset_for_testing() {
        let maybe_handle = SINGLETON.write(|state| state.deferred_check_handle.take());
        if let Some(handle) = maybe_handle {
            drop(handle.join());
        }
        SINGLETON.write(|state| {
            *state = Self::new();
        });
    }
}

/// Single-start witness stored in [`UpgradeSingletonState::start_status`] and returned by
/// [`UpgradeSingletonState::try_claim_start`].
///
/// Tracks whether [`UpgradeJourney::start`] has already claimed [`SINGLETON`] in this
/// process.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SingletonStartStatus {
    /// [`UpgradeJourney::start`] has not yet claimed [`SINGLETON`] in this process.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[default]
    NotStarted,

    /// [`UpgradeJourney::start`] has already claimed [`SINGLETON`] in this process; any
    /// subsequent `.start()` call must return [`StartOutcome::AlreadyStarted`].
    ///
    /// [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}

/// Outcome returned by [`handle_upgrade_on_exit`] when completing a deferred upgrade on
/// exit ([`UpgradeStrategy::Deferred`]).
///
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitOutcome {
    /// No deferred check was running (either [`UpgradeJourney::start`] was not called,
    /// policy was [`UpgradePolicy::Disabled`] / [`UpgradeStrategy::Immediate`] /
    /// [`CooldownSkipped`], OS thread spawn failed, or [`handle_upgrade_on_exit`] was
    /// already called earlier).
    ///
    /// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    NoDeferredCheckRunning,

    /// Dedicated thread finished and found `remote <= current` (`1h` cooldown timestamp
    /// file was touched).
    UpToDate(UpgradeJourney<UpToDate>),

    /// Dedicated thread failed (network error, parse error, runtime init error, or
    /// panic).
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Dedicated thread found `remote > current` ([`UpgradeAvailable`]), transitioned
    /// through [`Upgrading`], and finished [`UpgradeActionOnExit`] with a terminal
    /// [`UpgradeResolution`] inside [`Completed`].
    ///
    /// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
    /// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
    /// [`UpgradeResolution`]: crate::self_upgrade::UpgradeResolution
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    Completed(UpgradeJourney<Completed>),
}

/// Called by `main()` at shutdown on the main thread to complete a deferred upgrade on
/// exit ([`UpgradeStrategy::Deferred`][1]). This is part of the [Control Plane].
///
/// If [`UpgradeJourney::start`] spawned a dedicated thread, the main thread waits for
/// that thread to finish and receive its [`DeferredCheckOutcome`], then runs the
/// configured [`UpgradeActionOnExit`] on the main thread if an upgrade is available
/// ([`DeferredCheckOutcome::UpgradeAvailable`]):
/// 1. Takes [`UpgradeSingletonState::deferred_check_handle`] inside `SINGLETON.write` and
///    drops the lock **before** calling `handle.join()`, guaranteeing one-shot idempotent
///    behavior if called more than once.
/// 2. If `handle.join()` returns [`DeferredCheckOutcome::UpgradeAvailable`], transitions
///    into [`Upgrading`] and delegates to
///    `UpgradeJourney::<Upgrading>::execute_action_on_exit` (which invokes the [Execution
///    Plane] to prompt or install the upgrade), returning [`ExitOutcome::Completed`].
///
/// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
pub fn handle_upgrade_on_exit() -> ExitOutcome {
    // Step 1: Loud lock release — take `deferred_check_handle` inside `SINGLETON.write`
    // so the lock is dropped before `handle.join()` blocks on the dedicated thread.
    let Some(handle) = SINGLETON.write(|state| state.deferred_check_handle.take()) else {
        return ExitOutcome::NoDeferredCheckRunning;
    };

    let outcome = match handle.join() {
        Ok(outcome) => outcome,
        Err(panic_payload) => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade thread panicked outside catch_unwind",
                    panic = ?panic_payload
                );
            });
            return ExitOutcome::NoDeferredCheckRunning;
        }
    };

    // Step 2: Transition the `DeferredCheckOutcome` returned directly by the thread.
    match outcome {
        DeferredCheckOutcome::UpToDate(up_to_date) => ExitOutcome::UpToDate(up_to_date),
        DeferredCheckOutcome::CheckFailed(failed) => ExitOutcome::CheckFailed(failed),
        DeferredCheckOutcome::UpgradeAvailable(available) => {
            let action_on_exit = available.get_state().action_on_exit;
            let upgrading = available.into_upgrading();
            let completed = upgrading.execute_action_on_exit(action_on_exit);
            ExitOutcome::Completed(completed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CooldownPolicy, CrateDistribution, CrateMetadata, CrateReleaseDate,
                CrateVersion, EndpointUrl, JsoncScalar, ManifestCheckError, NotStarted,
                UpgradeActionOnExit, UpgradeConfig, UpgradePolicy, UpgradeResolution,
                UpgradeStrategy, UserFeedbackChannel};
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
    fn test_try_claim_start_single_claim_and_reset() {
        UpgradeSingletonState::reset_for_testing();

        assert_eq2!(
            UpgradeSingletonState::try_claim_start(),
            SingletonStartStatus::NotStarted
        );
        assert_eq2!(
            UpgradeSingletonState::try_claim_start(),
            SingletonStartStatus::AlreadyStarted
        );

        UpgradeSingletonState::reset_for_testing();
        assert_eq2!(
            UpgradeSingletonState::try_claim_start(),
            SingletonStartStatus::NotStarted
        );
    }

    #[test]
    #[serial]
    fn test_reset_for_testing_drains_inflight_thread_before_reset() {
        UpgradeSingletonState::reset_for_testing();

        let checking = make_test_journey("0.1.0", UpgradePolicy::default())
            .into_checking(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto));

        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(25));
            DeferredCheckOutcome::CheckFailed(checking.into_check_failed(
                ManifestCheckError::FetchOrParseFailed {
                    diagnostic: "late outcome".to_string(),
                },
            ))
        });
        UpgradeSingletonState::set_deferred_check_handle(handle);

        // `reset_for_testing` must join `handle` first, then reset `SINGLETON`.
        UpgradeSingletonState::reset_for_testing();
        assert_eq2!(
            handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }

    #[test]
    #[serial]
    fn test_handle_upgrade_on_exit_one_shot_idempotency() {
        UpgradeSingletonState::reset_for_testing();

        // 1. With no deferred check handle, returns `NoDeferredCheckRunning`.
        assert_eq2!(
            handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );

        // 2. `UpgradeAvailable(NotifyOnly)` transitions to `Completed(NotifiedOnly)`
        // once, and subsequent calls return `NoDeferredCheckRunning`.
        let meta = make_test_metadata("0.2.0");
        let available = make_test_journey(
            "0.1.0",
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::NotifyOnly,
            )),
        )
        .into_checking(UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly))
        .into_upgrade_available(
            UpgradeActionOnExit::NotifyOnly,
            meta,
            EndpointUrl::try_parse("https://example.com/releases/download").unwrap(),
        );
        let expected_completed = available
            .clone()
            .into_upgrading()
            .into_completed(UpgradeResolution::NotifiedOnly);

        let handle =
            std::thread::spawn(move || DeferredCheckOutcome::UpgradeAvailable(available));
        UpgradeSingletonState::set_deferred_check_handle(handle);

        assert_eq2!(
            handle_upgrade_on_exit(),
            ExitOutcome::Completed(expected_completed)
        );
        assert_eq2!(
            handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }
}

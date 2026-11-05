// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Completed, DeferredCheckOutcome, UpToDate, UpgradeJourney};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex,
               TokioRuntimeOutcome, scoped_mutex};
use std::{sync::LazyLock, thread::JoinHandle};

/// Process-wide singleton instance of [`SingletonState`], lazily initialized on first
/// access and protected by a [`ScopedMutex`] with [`PanicOnSpecificLockNesting`]. This
/// static is part of the [Control Plane] (see [Thread Model]).
///
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
pub static SINGLETON: LazyLock<
    ScopedMutex<SingletonState, { PanicOnSpecificLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(SPECIFIC, SingletonState::NotStarted));

/// Process-wide state stored inside [`SINGLETON`], encoding the single-start guard and
/// the optional [`DeferredCheckHandle`] of the dedicated thread (when spawned by
/// [`UpgradeJourney::start`]) as a 2-variant enum so [`NotStarted`][Self::NotStarted] +
/// `Some(handle)` is unrepresentable. This type is part of the [Control Plane] (see
/// [Thread Model]).
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
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug, Default)]
pub enum SingletonState {
    /// [`UpgradeJourney::start`] has not yet claimed [`SINGLETON`] in this process.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[default]
    NotStarted,

    /// [`UpgradeJourney::start`] has claimed [`SINGLETON`] in this process, optionally
    /// holding the [`DeferredCheckHandle`] of the dedicated thread spawned during a
    /// deferred upgrade check ([`UpgradeStrategy::Deferred`]) until
    /// [`handle_upgrade_on_exit`] takes and joins it.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    Started(Option<DeferredCheckHandle>),
}

impl SingletonState {
    /// Atomically transitions [`SINGLETON`] from [`Self::NotStarted`] to
    /// [`Self::Started(None)`][Self::Started], returning [`StartClaimStatus::Claimed`] to
    /// the first caller and [`StartClaimStatus::AlreadyStarted`] to any duplicate caller.
    ///
    /// # Preventing [`TOCTOU`] Race Conditions at Startup
    ///
    /// Performing both the [`Self::NotStarted`] check and the transition inside a single
    /// `SINGLETON.write` lock acquisition guarantees that at most one caller claims the
    /// engine without spawning extra threads or network calls.
    ///
    /// [`TOCTOU`]: https://en.wikipedia.org/wiki/Time-of-check-to-time-of-use
    #[must_use]
    pub fn try_claim_start() -> StartClaimStatus {
        SINGLETON.write(|state| match state {
            Self::NotStarted => {
                *state = Self::Started(None);
                StartClaimStatus::Claimed
            }
            Self::Started(_) => StartClaimStatus::AlreadyStarted,
        })
    }

    /// Stores the [`DeferredCheckHandle`] of the dedicated thread into
    /// [`Self::Started(Some(handle))`][Self::Started].
    ///
    /// Called on the current thread by `spawn_deferred_check_thread` (in the [Control
    /// Plane]) during a deferred upgrade on exit ([`UpgradeStrategy::Deferred`][1]; see
    /// [Thread Model]).
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    pub fn set_deferred_check_handle(handle: DeferredCheckHandle) {
        SINGLETON.write(|state| {
            *state = Self::Started(Some(handle));
        });
    }

    /// Takes the in-flight [`DeferredCheckHandle`] out of [`Self::Started`], leaving
    /// [`Self::Started(None)`][Self::Started] in place so [`SINGLETON`] remains claimed.
    #[must_use]
    pub fn take_deferred_check_handle() -> Option<DeferredCheckHandle> {
        SINGLETON.write(|state| match state {
            Self::NotStarted => None,
            Self::Started(slot) => slot.take(),
        })
    }

    /// Drains any in-flight dedicated thread and resets [`SINGLETON`] back to
    /// [`Self::NotStarted`].
    #[cfg(test)]
    pub(crate) fn reset_for_testing() {
        let maybe_handle = Self::take_deferred_check_handle();
        if let Some(handle) = maybe_handle {
            drop(handle.join());
        }
        SINGLETON.write(|state| {
            *state = Self::NotStarted;
        });
    }
}

/// [`JoinHandle`] of the dedicated thread spawned via
/// [`StdThreadTokioRunner::run_non_blocking`] during a deferred upgrade check
/// ([`UpgradeStrategy::Deferred`]).
///
/// [`StdThreadTokioRunner::run_non_blocking`]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
pub type DeferredCheckHandle = JoinHandle<TokioRuntimeOutcome<DeferredCheckOutcome>>;

/// Witness returned by [`SingletonState::try_claim_start`] indicating whether
/// [`UpgradeJourney::start`] claimed [`SINGLETON`] or observed a prior start.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartClaimStatus {
    /// [`SINGLETON`] was [`SingletonState::NotStarted`] and has now transitioned to
    /// [`SingletonState::Started(None)`][SingletonState::Started].
    Claimed,

    /// [`SINGLETON`] was already [`SingletonState::Started`]; caller must return
    /// [`StartOutcome::AlreadyStarted`].
    ///
    /// [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
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
    /// [`CooldownSkipped`], OS thread spawn or Tokio runtime initialization failed, or
    /// [`handle_upgrade_on_exit`] was already called earlier).
    ///
    /// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    NoDeferredCheckRunning,

    /// Dedicated thread finished and found `remote <= current` (`1h` cooldown timestamp
    /// file was touched).
    UpToDate(UpgradeJourney<UpToDate>),

    /// Dedicated thread failed during the manifest check (network or parse error).
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
/// 1. Takes the [`DeferredCheckHandle`] via
///    [`SingletonState::take_deferred_check_handle`] (releasing `SINGLETON.write`
///    **before** calling `handle.join()`), guaranteeing one-shot idempotent behavior if
///    called more than once.
/// 2. If `handle.join()` returns [`TokioRuntimeOutcome::Completed`] with
///    [`DeferredCheckOutcome::UpgradeAvailable`], transitions into [`Upgrading`] and
///    delegates to `UpgradeJourney::<Upgrading>::execute_action_on_exit` (which invokes
///    the [Execution Plane] to prompt or install the upgrade), returning
///    [`ExitOutcome::Completed`].
///
/// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[must_use]
pub fn handle_upgrade_on_exit() -> ExitOutcome {
    // Step 1: Loud lock release — take `DeferredCheckHandle` inside `SINGLETON.write`
    // so the lock is dropped before `handle.join()` blocks on the dedicated thread.
    let Some(handle) = SingletonState::take_deferred_check_handle() else {
        return ExitOutcome::NoDeferredCheckRunning;
    };

    let outcome = match handle.join() {
        Ok(TokioRuntimeOutcome::Completed(outcome)) => outcome,
        Ok(
            TokioRuntimeOutcome::FailedToStart(report)
            | TokioRuntimeOutcome::Panicked(report),
        ) => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade thread runtime failed or panicked",
                    error = ?report
                );
            });
            return ExitOutcome::NoDeferredCheckRunning;
        }
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
                UpgradeStrategy, UserFeedbackChannel, dedicated_thread};
    use r3bl_tui::{StdThreadTokioRunner, assert_eq2};
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
        SingletonState::reset_for_testing();

        assert_eq2!(SingletonState::try_claim_start(), StartClaimStatus::Claimed);
        assert_eq2!(
            SingletonState::try_claim_start(),
            StartClaimStatus::AlreadyStarted
        );

        SingletonState::reset_for_testing();
        assert_eq2!(SingletonState::try_claim_start(), StartClaimStatus::Claimed);
    }

    #[test]
    #[serial]
    fn test_reset_for_testing_drains_inflight_thread_before_reset() {
        SingletonState::reset_for_testing();

        let checking = make_test_journey("0.1.0", UpgradePolicy::default())
            .into_checking(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto));

        let handle = StdThreadTokioRunner::run_non_blocking(
            dedicated_thread::DEDICATED_THREAD_NAME,
            async move || {
                tokio::time::sleep(Duration::from_millis(25)).await;
                DeferredCheckOutcome::CheckFailed(checking.into_check_failed(
                    ManifestCheckError::FetchOrParseFailed {
                        diagnostic: "late outcome".to_string(),
                    },
                ))
            },
        )
        .unwrap();
        SingletonState::set_deferred_check_handle(handle);

        // `reset_for_testing` must join `handle` first, then reset `SINGLETON`.
        SingletonState::reset_for_testing();
        assert_eq2!(
            handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }

    #[test]
    #[serial]
    fn test_handle_upgrade_on_exit_one_shot_idempotency() {
        SingletonState::reset_for_testing();

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

        let handle = StdThreadTokioRunner::run_non_blocking(
            dedicated_thread::DEDICATED_THREAD_NAME,
            async move || DeferredCheckOutcome::UpgradeAvailable(available),
        )
        .unwrap();
        SingletonState::set_deferred_check_handle(handle);

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

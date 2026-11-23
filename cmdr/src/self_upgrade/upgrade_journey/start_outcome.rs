// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, CheckFailedError, Checking, Completed, DeferredCheckOutcome,
            Disabled, ExitOutcome, Skipped, UpToDate, UpgradeJourney,
            thread_runner::ThreadRunner};
use r3bl_tui::StdThreadTokioJoinHandle;

/// Handle to an in-flight dedicated check thread spawned during a deferred upgrade on
/// exit ([`UpgradeStrategy::Deferred`]).
///
/// Owned linearly inside [`DeferredUpgradeOutcome::DeferredCheckSpawned`] and consumed
/// by [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]. Storing both the
/// [`UpgradeJourney<Checking>`] fallback and the [`StdThreadTokioJoinHandle`] inside this
/// opaque wrapper guarantees that:
/// 1. Duplicate `.start()` callers receiving [`StartOutcome::AlreadyStarted`] can never
///    steal the active check thread's handle.
/// 2. If the dedicated OS thread's Tokio runtime fails to initialize or panics,
///    [`DeferredUpgradeOutcome::handle_upgrade_on_exit`] transitions the fallback journey
///    into [`ExitOutcome::CheckFailed`] without losing the diagnostic payload.
///
/// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
/// [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
/// [`StdThreadTokioJoinHandle`]: r3bl_tui::StdThreadTokioJoinHandle
/// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
#[derive(Debug)]
pub struct DeferredCheckHandle {
    pub(in crate::self_upgrade) checking: UpgradeJourney<Checking>,
    pub(in crate::self_upgrade) join_handle:
        StdThreadTokioJoinHandle<DeferredCheckOutcome>,
}

impl DeferredCheckHandle {
    /// Returns a reference to the [`UpgradeJourney<Checking>`] associated with the
    /// in-flight dedicated check thread.
    ///
    /// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
    #[must_use]
    pub fn get_journey(&self) -> &UpgradeJourney<Checking> { &self.checking }
}

/// Outcome returned by [`UpgradeJourney::<NotStarted>::start`][start], telling the
/// binary's `main()` whether upgrade checks are disabled, `--upgrade` executed
/// immediately, a deferred upgrade was scheduled for exit, or `start()` was already
/// called earlier in this process.
///
/// At application exit, if [`Self::DeferredUpgrade`] was returned, call
/// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`] (consuming it by value) to join
/// any owned [`DeferredCheckHandle`] and complete the deferred upgrade lifecycle.
/// [`Self::Disabled`], [`Self::ImmediateUpgrade`], and [`Self::AlreadyStarted`] are
/// terminal at startup and require no exit handling.
///
/// [start]: crate::self_upgrade::UpgradeJourney::start
#[must_use = "match ImmediateUpgrade to exit early, or hold DeferredUpgrade to call .handle_upgrade_on_exit() at shutdown"]
#[derive(Debug)]
pub enum StartOutcome {
    /// `--upgrade-behavior disable` ([`UpgradePolicy::Disabled`]): upgrade checks are
    /// disabled and no exit handling is needed.
    ///
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    Disabled(UpgradeJourney<Disabled>),

    /// `--upgrade` ([`UpgradeStrategy::Immediate`]) executed; `main()` should exit
    /// immediately without running normal application logic.
    ///
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    ImmediateUpgrade(ImmediateUpgradeOutcome),

    /// Deferred upgrade on exit ([`UpgradeStrategy::Deferred`]); `main()` continues
    /// running the application and calls
    /// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`] at shutdown.
    ///
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    DeferredUpgrade(DeferredUpgradeOutcome),

    /// [`UpgradeJourney::start`] was already called earlier in this process; the
    /// duplicate call spawned zero threads and left [`PerProcessStartLatch`] claimed
    /// without a [`DeferredCheckHandle`].
    ///
    /// [`PerProcessStartLatch`]: crate::self_upgrade::upgrade_journey::per_process_start_latch::PerProcessStartLatch
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}

/// Outcome of an explicit `--upgrade` ([`UpgradeStrategy::Immediate`]) run inside
/// [`StartOutcome::ImmediateUpgrade`].
///
/// Uses a pure domain enum instead of `miette::Result<()>` so [`UserFeedbackChannel`]
/// remains the single output emitter (preventing `miette`'s `Termination` handler from
/// printing duplicate `Error:` diagnostics on [`stderr`] or leaking output when
/// [`UserFeedbackChannel::Quiet`] is configured).
///
/// [`stderr`]: std::io::stderr
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UserFeedbackChannel::Quiet`]: crate::self_upgrade::UserFeedbackChannel::Quiet
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
#[derive(Debug)]
pub enum ImmediateUpgradeOutcome {
    /// Local binary was already up to date (`remote <= current`).
    UpToDate(UpgradeJourney<UpToDate>),

    /// Manifest check or Tokio runtime initialization failed; user-facing retry feedback
    /// has already been emitted via [`UserFeedbackChannel`].
    ///
    /// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Upgrade ran and finished with [`UpgradeResolution::Upgraded`] or
    /// [`UpgradeResolution::UpgradeFailed`].
    ///
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
    Completed(UpgradeJourney<Completed>),
}

/// Startup state when [`UpgradeJourney::start`] returns
/// [`StartOutcome::DeferredUpgrade`].
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[must_use = "call .handle_upgrade_on_exit() at application shutdown"]
#[derive(Debug)]
pub enum DeferredUpgradeOutcome {
    /// Deferred check was skipped because the `1h` cooldown timestamp file is still
    /// active.
    Skipped(UpgradeJourney<Skipped>),

    /// Dedicated thread was spawned with [`UpgradeJourney<Checking>`] and its owned
    /// [`DeferredCheckHandle`] is stored here for [`Self::handle_upgrade_on_exit`].
    ///
    /// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
    DeferredCheckSpawned(DeferredCheckHandle),

    /// Dedicated OS thread failed to spawn; transitioned to
    /// [`UpgradeJourney<CheckFailed>`].
    ///
    /// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
    DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
}

impl DeferredUpgradeOutcome {
    /// Called by `main()` at shutdown on the main thread (consuming `self` by value) to
    /// complete a deferred upgrade on exit ([`UpgradeStrategy::Deferred`][1]). The TUI
    /// teardown must be complete before this is called. This is part of the [Control
    /// Plane].
    ///
    /// Consumes the linear [`DeferredUpgradeOutcome`] value so it cannot be invoked more
    /// than once or steal another caller's thread handle:
    /// 1. [`Self::Skipped`] holds no thread handle and returns [`ExitOutcome::Skipped`].
    /// 2. [`Self::DeferredCheckSpawnFailed`] preserves its OS thread spawn failure by
    ///    returning [`ExitOutcome::CheckFailed`].
    /// 3. [`Self::DeferredCheckSpawned`] joins its owned [`DeferredCheckHandle`]:
    ///    - If joining returns [`DeferredCheckOutcome::UpgradeAvailable`], transitions
    ///      into [`Upgrading`] and delegates to
    ///      `UpgradeJourney::<Upgrading>::execute_action_on_exit` (which invokes the
    ///      [Execution Plane] to prompt or install the upgrade), returning
    ///      [`ExitOutcome::Completed`].
    ///    - If joining fails with [`ThreadRunnerError::RuntimeInitFailed`] or
    ///      [`ThreadRunnerError::ThreadPanicked`], preserves the diagnostic payload
    ///      inside [`ExitOutcome::CheckFailed`]
    ///      ([`CheckFailedError::ThreadRunner`]).
    ///
    /// # Terminal State at Exit
    ///
    /// When executing logic that runs on exit using
    /// [`StdThreadTokioRunner::run_blocking`][2], after a TUI application (like `giti` or
    /// `edi`) is exiting, the main UI loop ([`TerminalWindow::main_event_loop`],
    /// [`ReadlineAsyncContext`], or [`choose`]) has already dropped its [RAII] guards and
    /// restored the terminal from [raw mode] back to [cooked mode]. We have designed it
    /// this way so that the following can work interoperably with full TUI apps:
    /// 1. [`choose`] can cleanly create its own temporary [raw mode] session for modal
    ///    prompts and restore [cooked mode] afterward.
    /// 2. [`Spinner`] (in [standalone mode] with `maybe_shared_writer: None`, and not
    ///    embedded inside an active [`ReadlineAsyncContext`]) can run in [cooked mode].
    ///
    /// [1]: crate::self_upgrade::UpgradeStrategy::Deferred
    /// [2]: r3bl_tui::StdThreadTokioRunner::run_blocking
    /// [`choose`]: r3bl_tui::choose
    /// [`ReadlineAsyncContext`]: r3bl_tui::ReadlineAsyncContext
    /// [`Spinner`]: r3bl_tui::Spinner
    /// [`TerminalWindow::main_event_loop`]: r3bl_tui::TerminalWindow::main_event_loop
    /// [`ThreadRunnerError::RuntimeInitFailed`]: crate::self_upgrade::ThreadRunnerError::RuntimeInitFailed
    /// [`ThreadRunnerError::ThreadPanicked`]: crate::self_upgrade::ThreadRunnerError::ThreadPanicked
    /// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [cooked mode]: mod@r3bl_tui::core::ansi#terminal-input-modes-raw-vs-cooked
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
    /// [raw mode]: mod@r3bl_tui::core::ansi#terminal-input-modes-raw-vs-cooked
    /// [standalone mode]: r3bl_tui::Spinner#standalone-mode
    #[must_use]
    pub fn handle_upgrade_on_exit(self) -> ExitOutcome {
        let DeferredCheckHandle {
            checking,
            join_handle,
        } = match self {
            Self::Skipped(skipped) => return ExitOutcome::Skipped(skipped),
            Self::DeferredCheckSpawnFailed(failed) => {
                return ExitOutcome::CheckFailed(failed);
            }
            Self::DeferredCheckSpawned(handle) => handle,
        };

        let outcome = match ThreadRunner::join(checking.get_config(), join_handle) {
            Ok(outcome) => outcome,
            Err(thread_err) => {
                return ExitOutcome::CheckFailed(
                    checking.into_check_failed(CheckFailedError::from(thread_err)),
                );
            }
        };

        // Transition the `DeferredCheckOutcome` returned directly by the thread.
        match outcome {
            DeferredCheckOutcome::UpToDate(up_to_date) => {
                ExitOutcome::UpToDate(up_to_date)
            }
            DeferredCheckOutcome::CheckFailed(failed) => ExitOutcome::CheckFailed(failed),
            DeferredCheckOutcome::UpgradeAvailable(available) => {
                let action_on_exit = available.get_state().action_on_exit;
                let upgrading = available.into_upgrading();
                let completed = upgrading.execute_action_on_exit(action_on_exit);
                ExitOutcome::Completed(completed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrateDistribution, CrateMetadata, CrateReleaseDate, CrateVersion,
                EndpointUrl, JsoncScalar, NotStarted, RemoteVersionCheckGate,
                ThreadRunnerError, UpgradeActionOnExit, UpgradeConfig, UpgradePolicy,
                UpgradeResolution, UpgradeStrategy, UserFeedbackChannel};
    use r3bl_tui::{StdThreadTokioRunner, assert_eq2};

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
            RemoteVersionCheckGate::Disabled,
        );
        UpgradeJourney::new(config)
    }

    #[test]
    fn test_handle_upgrade_on_exit_skipped_returns_exit_skipped() {
        let skipped = make_test_journey(
            "0.1.0",
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::NotifyOnly,
            )),
        )
        .into_skipped();
        let outcome =
            DeferredUpgradeOutcome::Skipped(skipped.clone()).handle_upgrade_on_exit();
        match outcome {
            ExitOutcome::Skipped(actual) => assert_eq2!(actual, skipped),
            other => panic!("Expected ExitOutcome::Skipped, got {other:?}"),
        }
    }

    #[test]
    fn test_handle_upgrade_on_exit_preserves_spawn_failure() {
        let strategy = UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto);
        let error = CheckFailedError::ThreadRunner(
            ThreadRunnerError::ThreadSpawnFailed {
                src_err: std::io::Error::other("Resource busy"),
            },
        );
        let failed = make_test_journey("0.1.0", UpgradePolicy::Enabled(strategy))
            .into_checking(strategy)
            .into_check_failed(error);

        let deferred = DeferredUpgradeOutcome::DeferredCheckSpawnFailed(failed);
        match deferred.handle_upgrade_on_exit() {
            ExitOutcome::CheckFailed(journey) => match journey.get_state().error {
                CheckFailedError::ThreadRunner(
                    ThreadRunnerError::ThreadSpawnFailed { ref src_err },
                ) => {
                    assert!(src_err.to_string().contains("Resource busy"));
                }
                ref other => {
                    panic!("Expected ThreadSpawnFailed, got {other:?}")
                }
            },
            other => panic!("Expected ExitOutcome::CheckFailed, got {other:?}"),
        }
    }

    #[test]
    fn test_handle_upgrade_on_exit_executes_owned_deferred_handle() {
        let meta = make_test_metadata("0.2.0");
        let strategy = UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly);
        let checking = make_test_journey("0.1.0", UpgradePolicy::Enabled(strategy))
            .into_checking(strategy);
        let available = checking.clone().into_upgrade_available(
            UpgradeActionOnExit::NotifyOnly,
            meta,
            EndpointUrl::try_parse("https://example.com/releases/download").unwrap(),
        );

        let join_handle = StdThreadTokioRunner::run_non_blocking(async move || {
            DeferredCheckOutcome::UpgradeAvailable(available)
        })
        .unwrap();

        let deferred_handle = DeferredCheckHandle {
            checking: checking.clone(),
            join_handle,
        };
        assert_eq2!(deferred_handle.get_journey(), &checking);

        let deferred = DeferredUpgradeOutcome::DeferredCheckSpawned(deferred_handle);
        match deferred.handle_upgrade_on_exit() {
            ExitOutcome::Completed(completed) => {
                assert_eq2!(
                    completed.get_state().metadata.version,
                    CrateVersion::try_parse("0.2.0").unwrap()
                );
                assert!(matches!(
                    completed.get_state().resolution,
                    UpgradeResolution::NotifiedOnly
                ));
            }
            other => panic!("Expected ExitOutcome::Completed, got {other:?}"),
        }
    }
}

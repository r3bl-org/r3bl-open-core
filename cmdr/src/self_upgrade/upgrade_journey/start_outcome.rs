// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Checking, Completed, CooldownSkipped, DeferredCheckOutcome,
            Disabled, ExitOutcome, SingletonState, UpToDate, UpgradeJourney};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::TokioRuntimeOutcome;

/// Outcome returned by [`UpgradeJourney::<NotStarted>::start`][start], telling the
/// binary's `main()` whether upgrade checks are disabled, `--upgrade` executed
/// immediately, a deferred upgrade was scheduled for exit, or `start()` was already
/// called earlier in this process.
///
/// At application exit, call [`Self::handle_upgrade_on_exit`] on this value to join any
/// in-flight dedicated thread and complete the deferred upgrade lifecycle.
///
/// [start]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartOutcome {
    /// `--upgrade-behavior disable` ([`UpgradePolicy::Disabled`]): upgrade checks are
    /// disabled; calling [`Self::handle_upgrade_on_exit`] at exit is a no-op returning
    /// [`ExitOutcome::NoDeferredCheckRunning`].
    ///
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    Disabled(UpgradeJourney<Disabled>),

    /// `--upgrade` ([`UpgradeStrategy::Immediate`]) executed; `main()` should exit
    /// immediately without running normal application logic.
    ///
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    ImmediateUpgrade(ImmediateUpgradeOutcome),

    /// Deferred upgrade on exit ([`UpgradeStrategy::Deferred`]); `main()` continues
    /// running the application and calls [`Self::handle_upgrade_on_exit`] at shutdown.
    ///
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    DeferredUpgrade(DeferredUpgradeOutcome),

    /// [`UpgradeJourney::start`] was already called earlier in this process; the
    /// duplicate call spawned zero threads and left [`SINGLETON`] unchanged.
    ///
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}

impl StartOutcome {
    /// Called by `main()` at shutdown on the main thread to complete a deferred upgrade
    /// on exit ([`UpgradeStrategy::Deferred`][1]). The TUI teardown must be complete
    /// before this is called. This is part of the [Control Plane].
    ///
    /// If [`UpgradeJourney::start`] spawned a dedicated thread, the main thread waits for
    /// that thread to finish and receive its [`DeferredCheckOutcome`], then runs the
    /// configured [`UpgradeActionOnExit`] on the main thread if an upgrade is available
    /// ([`DeferredCheckOutcome::UpgradeAvailable`]):
    /// 1. Takes the [`DeferredCheckHandle`] via
    ///    [`SingletonState::take_deferred_check_handle`] (releasing `SINGLETON.write`
    ///    **before** calling `handle.join()`), guaranteeing one-shot idempotent behavior
    ///    if called more than once.
    /// 2. If `handle.join()` returns [`TokioRuntimeOutcome::Completed`] with
    ///    [`DeferredCheckOutcome::UpgradeAvailable`], transitions into [`Upgrading`] and
    ///    delegates to `UpgradeJourney::<Upgrading>::execute_action_on_exit` (which
    ///    invokes the [Execution Plane] to prompt or install the upgrade), returning
    ///    [`ExitOutcome::Completed`].
    ///
    /// # Terminal State at Exit
    ///
    /// When executing logic that runs on exit using
    /// [`StdThreadTokioRunner::run_blocking`], after a TUI application (like `giti` or
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
    /// [`choose`]: r3bl_tui::choose
    /// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
    /// [`ReadlineAsyncContext`]: r3bl_tui::ReadlineAsyncContext
    /// [`Spinner`]: r3bl_tui::Spinner
    /// [`StdThreadTokioRunner::run_blocking`]:
    ///     r3bl_tui::StdThreadTokioRunner::run_blocking
    /// [`TerminalWindow::main_event_loop`]: r3bl_tui::TerminalWindow::main_event_loop
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
    pub fn handle_upgrade_on_exit(&self) -> ExitOutcome {
        // Step 1: Loud lock release — take `DeferredCheckHandle` inside `SINGLETON.write`
        // so the lock is dropped before `handle.join()` blocks on the dedicated thread.
        let Some(handle) = SingletonState::take_deferred_check_handle() else {
            return ExitOutcome::NoDeferredCheckRunning;
        };

        let outcome = match handle.join() {
            TokioRuntimeOutcome::Completed(outcome) => outcome,
            TokioRuntimeOutcome::FailedToStart(report)
            | TokioRuntimeOutcome::Panicked(report) => {
                DEBUG_SELF_UPGRADE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Dedicated self-upgrade thread runtime failed or panicked",
                        error = ?report
                    );
                });
                return ExitOutcome::NoDeferredCheckRunning;
            }
        };

        // Step 2: Transition the `DeferredCheckOutcome` returned directly by the thread.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferredUpgradeOutcome {
    /// Deferred check was skipped because the `1h` cooldown timestamp file is still
    /// active.
    CooldownSkipped(UpgradeJourney<CooldownSkipped>),

    /// Dedicated thread was spawned with [`UpgradeJourney<Checking>`] and its
    /// [`DeferredCheckHandle`] was stored in [`SINGLETON`] for
    /// [`StartOutcome::handle_upgrade_on_exit`].
    ///
    /// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`StartOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
    /// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
    DeferredCheckSpawned(UpgradeJourney<Checking>),

    /// Dedicated OS thread failed to spawn; transitioned to
    /// [`UpgradeJourney<CheckFailed>`].
    ///
    /// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
    DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
}

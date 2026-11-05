// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Checking, DeferredUpgradeOutcome, SingletonState, UpgradeJourney};
use crate::{DEBUG_SELF_UPGRADE, ManifestCheckError, UpgradeActionOnExit, UpgradeConfig,
            dedicated_thread};
use r3bl_tui::{StdThreadTokioRunner, TokioRuntimeOutcome};

/// Wrapper around [`StdThreadTokioRunner::run_blocking`] that logs runtime startup or
/// panic failures via [`DEBUG_SELF_UPGRADE`].
///
/// Part of the [Control Plane] threading and runtime orchestration (see [Thread Model]).
///
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
pub(in crate::self_upgrade) fn run_blocking_logged<
    T: Send,
    F: Send + AsyncFnOnce() -> T,
>(
    config: &UpgradeConfig,
    async_fn: F,
) -> TokioRuntimeOutcome<T> {
    let outcome = StdThreadTokioRunner::run_blocking(async_fn);
    if let TokioRuntimeOutcome::FailedToStart(ref report)
    | TokioRuntimeOutcome::Panicked(ref report) = outcome
    {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "run_blocking_logged: Tokio runtime execution failed",
                crate_name = %config.get_crate_name(),
                error = ?report
            );
        });
    }
    outcome
}

/// Spawns the dedicated `"self-upgrade"` thread via
/// [`StdThreadTokioRunner::run_non_blocking`] to run
/// [`UpgradeJourney::<Checking>::check_for_deferred_upgrade`], stores its
/// [`DeferredCheckHandle`] inside [`SINGLETON`], and returns [`DeferredUpgradeOutcome`].
///
/// Part of the [Control Plane] dedicated thread orchestration (see [Thread Model]).
///
/// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
/// [`SINGLETON`]: crate::self_upgrade::SINGLETON
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[must_use]
pub(in crate::self_upgrade) fn spawn_deferred_check_thread(
    checking: UpgradeJourney<Checking>,
    action_on_exit: UpgradeActionOnExit,
) -> DeferredUpgradeOutcome {
    let spawn_checking = checking.clone();
    match StdThreadTokioRunner::run_non_blocking(
        dedicated_thread::DEDICATED_THREAD_NAME,
        async move || {
            spawn_checking
                .check_for_deferred_upgrade(action_on_exit)
                .await
        },
    ) {
        Ok(handle) => {
            SingletonState::set_deferred_check_handle(handle);
            DeferredUpgradeOutcome::DeferredCheckSpawned(checking)
        }
        Err(io_err) => {
            let error = ManifestCheckError::RuntimeOrThreadInitFailed {
                reason: format!(
                    "Failed to spawn dedicated self-upgrade thread: {io_err:#}"
                ),
            };
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Could not spawn dedicated self-upgrade thread",
                    crate_name = %checking.get_config().get_crate_name(),
                    error = %error
                );
            });
            DeferredUpgradeOutcome::DeferredCheckSpawnFailed(
                checking.into_check_failed(error),
            )
        }
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SELF_UPGRADE, UpgradeConfig};
use r3bl_tui::{StdThreadTokioJoinHandle, StdThreadTokioRunner, TokioRuntimeOutcome};
use std::io;

/// Marker struct that wraps [`StdThreadTokioRunner`] with
/// logging ([`DEBUG_SELF_UPGRADE`]).
///
/// Part of the [Control Plane] threading and runtime orchestration (see [Thread Model]).
///
/// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
/// [`StdThreadTokioRunner`]: r3bl_tui::StdThreadTokioRunner
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug)]
pub struct ThreadRunner;

impl ThreadRunner {
    /// Wrapper around [`StdThreadTokioRunner::run_blocking`][1] that logs runtime startup
    /// or panic failures via [`DEBUG_SELF_UPGRADE`].
    ///
    /// [1]: r3bl_tui::StdThreadTokioRunner::run_blocking
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    pub(in crate::self_upgrade) fn run_blocking<T: Send, F: Send + AsyncFnOnce() -> T>(
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
                    message = "ThreadRunner::run_blocking: Tokio runtime execution failed",
                    crate_name = %config.get_crate_name(),
                    error = ?report
                );
            });
        }
        outcome
    }

    /// Wrapper around [`StdThreadTokioRunner::run_non_blocking`][2] that spawns the
    /// dedicated thread and logs thread spawn failures via [`DEBUG_SELF_UPGRADE`].
    ///
    /// [2]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    pub(in crate::self_upgrade) fn run_non_blocking<
        T: Send + 'static,
        F: Send + 'static + AsyncFnOnce() -> T,
    >(
        config: &UpgradeConfig,
        async_fn: F,
    ) -> io::Result<StdThreadTokioJoinHandle<T>> {
        let result = StdThreadTokioRunner::run_non_blocking(async_fn);
        if let Err(ref io_err) = result {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "ThreadRunner::run_non_blocking: Could not spawn dedicated self-upgrade thread",
                    crate_name = %config.get_crate_name(),
                    error = ?io_err
                );
            });
        }
        result
    }
}

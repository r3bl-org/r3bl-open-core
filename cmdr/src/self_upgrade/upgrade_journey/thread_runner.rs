// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SELF_UPGRADE, UpgradeConfig};
use r3bl_tui::{StdThreadTokioJoinHandle, StdThreadTokioRunner, TokioRuntimeOutcome};

/// Control Plane failure when spawning, initializing, or joining a dedicated or scoped
/// self-upgrade OS thread and its Tokio runtime ([`StdThreadTokioRunner`]).
///
/// [`StdThreadTokioRunner`]: r3bl_tui::StdThreadTokioRunner
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ThreadRunnerError {
    /// Dedicated OS thread failed to spawn ([`StdThreadTokioRunner::run_non_blocking`]).
    ///
    /// [`StdThreadTokioRunner::run_non_blocking`]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
    #[error("Failed to spawn dedicated self-upgrade OS thread: {src_err}")]
    ThreadSpawnFailed {
        /// Underlying OS thread spawn error.
        #[source]
        src_err: std::io::Error,
    },

    /// Scoped OS thread spawn or Tokio runtime initialization failed
    /// ([`TokioRuntimeOutcome::FailedToStart`]).
    ///
    /// [`TokioRuntimeOutcome::FailedToStart`]: r3bl_tui::TokioRuntimeOutcome::FailedToStart
    #[error("Failed to initialize Tokio runtime on self-upgrade OS thread: {src_err}")]
    RuntimeInitFailed {
        /// Underlying OS thread or Tokio runtime initialization error.
        #[source]
        src_err: std::io::Error,
    },

    /// Dedicated or scoped OS thread panicked during execution
    /// ([`TokioRuntimeOutcome::Panicked`]).
    ///
    /// [`TokioRuntimeOutcome::Panicked`]: r3bl_tui::TokioRuntimeOutcome::Panicked
    #[error("{thread_name} panicked: {panic_msg}")]
    ThreadPanicked {
        /// Diagnostic thread label (`SCOPED_THREAD_NAME` or `UNSCOPED_THREAD_NAME`).
        thread_name: &'static str,

        /// Extracted panic payload description.
        panic_msg: String,
    },
}

/// Marker struct that wraps [`StdThreadTokioRunner`] with structured
/// [`ThreadRunnerError`] conversion and logging ([`DEBUG_SELF_UPGRADE`]).
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
    /// or panic failures via [`DEBUG_SELF_UPGRADE`] and converts them into
    /// [`ThreadRunnerError`].
    ///
    /// # Errors
    ///
    /// Returns [`ThreadRunnerError::RuntimeInitFailed`] if the scoped OS thread or Tokio
    /// runtime failed to start, or [`ThreadRunnerError::ThreadPanicked`] if the thread
    /// panicked.
    ///
    /// [1]: r3bl_tui::StdThreadTokioRunner::run_blocking
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    pub fn run_blocking<T: Send, F: Send + AsyncFnOnce() -> T>(
        config: &UpgradeConfig,
        async_fn: F,
    ) -> Result<T, ThreadRunnerError> {
        map_runtime_outcome(
            StdThreadTokioRunner::run_blocking(async_fn),
            "ThreadRunner::run_blocking: Tokio runtime execution failed",
            config,
        )
    }

    /// Wrapper around [`StdThreadTokioRunner::run_non_blocking`][2] that spawns the
    /// dedicated thread, logs spawn failures via [`DEBUG_SELF_UPGRADE`], and converts
    /// them into [`ThreadRunnerError`].
    ///
    /// # Errors
    ///
    /// Returns [`ThreadRunnerError::ThreadSpawnFailed`] if the OS thread could not be
    /// spawned.
    ///
    /// [2]: r3bl_tui::StdThreadTokioRunner::run_non_blocking
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    pub fn run_non_blocking<
        T: Send + 'static,
        F: Send + 'static + AsyncFnOnce() -> T,
    >(
        config: &UpgradeConfig,
        async_fn: F,
    ) -> Result<StdThreadTokioJoinHandle<T>, ThreadRunnerError> {
        StdThreadTokioRunner::run_non_blocking(async_fn).map_err(|src_err| {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "ThreadRunner::run_non_blocking: Could not spawn dedicated self-upgrade thread",
                    crate_name = %config.get_crate_name(),
                    error = ?src_err
                );
            });
            ThreadRunnerError::ThreadSpawnFailed { src_err }
        })
    }

    /// Joins `join_handle`, logs runtime startup or panic failures via
    /// [`DEBUG_SELF_UPGRADE`], and converts them into [`ThreadRunnerError`].
    ///
    /// # Errors
    ///
    /// Returns [`ThreadRunnerError::RuntimeInitFailed`] if the dedicated Tokio runtime
    /// failed to initialize, or [`ThreadRunnerError::ThreadPanicked`] if the dedicated
    /// thread panicked.
    ///
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    pub fn join<T: Send + 'static>(
        config: &UpgradeConfig,
        join_handle: StdThreadTokioJoinHandle<T>,
    ) -> Result<T, ThreadRunnerError> {
        map_runtime_outcome(
            join_handle.join(),
            "ThreadRunner::join: Dedicated self-upgrade thread execution failed",
            config,
        )
    }
}

fn map_runtime_outcome<T>(
    outcome: TokioRuntimeOutcome<T>,
    log_message: &str,
    config: &UpgradeConfig,
) -> Result<T, ThreadRunnerError> {
    match outcome {
        TokioRuntimeOutcome::Completed(value) => Ok(value),
        TokioRuntimeOutcome::FailedToStart(src_err) => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = log_message,
                    crate_name = %config.get_crate_name(),
                    error = ?src_err
                );
            });
            Err(ThreadRunnerError::RuntimeInitFailed { src_err })
        }
        TokioRuntimeOutcome::Panicked {
            thread_name,
            panic_msg,
        } => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = log_message,
                    crate_name = %config.get_crate_name(),
                    thread_name = thread_name,
                    panic_msg = %panic_msg
                );
            });
            Err(ThreadRunnerError::ThreadPanicked {
                thread_name,
                panic_msg,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrateVersion, EndpointUrl, JsoncScalar, RemoteVersionCheckGate,
                UpgradePolicy, UserFeedbackChannel};
    use r3bl_tui::assert_eq2;

    fn make_test_config() -> UpgradeConfig {
        UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Disabled,
            EndpointUrl::default_manifest_url(),
            RemoteVersionCheckGate::Disabled,
        )
    }

    #[test]
    fn test_run_blocking_returns_ok_and_catches_panic() {
        let config = make_test_config();
        let res_ok = ThreadRunner::run_blocking(&config, async || 42_u32);
        assert_eq2!(res_ok.unwrap(), 42_u32);

        let res_panic: Result<(), ThreadRunnerError> =
            ThreadRunner::run_blocking(&config, async || {
                panic!("simulated blocking panic");
            });
        match res_panic {
            Err(ThreadRunnerError::ThreadPanicked { ref panic_msg, .. }) => {
                assert!(panic_msg.contains("simulated blocking panic"));
            }
            other => panic!("Expected Err(ThreadPanicked), got {other:?}"),
        }
    }

    #[test]
    fn test_run_non_blocking_and_join_returns_ok_and_catches_panic() {
        let config = make_test_config();
        let ok_handle = ThreadRunner::run_non_blocking(&config, async || 7_u32).unwrap();
        assert_eq2!(ThreadRunner::join(&config, ok_handle).unwrap(), 7_u32);

        let panic_handle = ThreadRunner::run_non_blocking(&config, async || -> u32 {
            panic!("simulated non-blocking panic");
        })
        .unwrap();
        match ThreadRunner::join(&config, panic_handle) {
            Err(ThreadRunnerError::ThreadPanicked { ref panic_msg, .. }) => {
                assert!(panic_msg.contains("simulated non-blocking panic"));
            }
            other => panic!("Expected Err(ThreadPanicked), got {other:?}"),
        }
    }
}

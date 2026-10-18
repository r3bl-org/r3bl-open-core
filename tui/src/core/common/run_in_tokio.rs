// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Creates a dedicated [scoped OS thread][st], starts a [`current_thread`][ct] Tokio
/// runtime on it, in order to execute `async_fn` to completion.
///
/// These are the reasons why we adopted this seemingly complex pattern:
/// 1. Running on a dedicated OS thread guarantees [`block_on`][bo] never panics with
///    `"Cannot start a runtime from within a runtime"` when called from inside an active
///    Tokio runtime.
/// 2. Using a dedicated scoped OS thread [`std::thread::scope`] allows `async_fn` to
///    borrow stack data without requiring `'static` or [`Arc`] cloning.
///
/// # Thread Bounds (`Send` vs. `!Send`)
///
/// - **`F` (`Send + AsyncFnOnce() -> T`)**: Must be [`Send`] because `async_fn` moves
///   **into** the spawned OS thread. However, the [`Future`] produced when `async_fn()`
///   is called inside the thread does **not** need to be [`Send`], because it is
///   created, polled, and dropped entirely on that single thread.
/// - **`T` (`Send`)**: Must be [`Send`] because [`TokioRuntimeOutcome::Completed(T)`][co]
///   moves **out of** the spawned thread back to the caller via `join_handle.join()`.
///
/// # Returns
///
/// - [`TokioRuntimeOutcome::Completed`]: The `current_thread` Tokio runtime initialized
///   and ran `async_fn` to completion, returning its output `T`.
/// - [`TokioRuntimeOutcome::FailedToStart`]: Building the `current_thread` Tokio runtime
///   failed before `async_fn` could run.
/// - [`TokioRuntimeOutcome::Panicked`]: The Tokio runtime started, but `async_fn`
///   panicked during execution.
///
/// [`Arc`]: std::sync::Arc
/// [bo]: tokio::runtime::Runtime::block_on
/// [co]: TokioRuntimeOutcome::Completed
/// [ct]: tokio::runtime::Builder::new_current_thread
/// [st]: std::thread::scope
pub fn run_in_tokio<T: Send, F: Send + AsyncFnOnce() -> T>(
    async_fn: F,
) -> TokioRuntimeOutcome<T> {
    std::thread::scope(|scoped_thread| {
        // Spawn a new thread and execute the async function in a new Tokio runtime.
        let join_handle = scoped_thread.spawn({
            || -> TokioRuntimeOutcome<T> {
                let res_runtime_create = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match res_runtime_create {
                    Ok(runtime) => {
                        let outcome = runtime.block_on(async_fn());
                        TokioRuntimeOutcome::Completed(outcome)
                    }
                    Err(err) => TokioRuntimeOutcome::failed_to_start(&err),
                }
            }
        });

        // Return the result from the join handle.
        match join_handle.join() {
            Ok(execution) => execution,
            Err(panic_payload) => TokioRuntimeOutcome::panicked(&panic_payload),
        }
    })
}

/// Outcome of executing an async closure on an isolated Tokio runtime via
/// [`run_in_tokio`].
#[derive(Debug)]
pub enum TokioRuntimeOutcome<T> {
    /// The Tokio runtime initialized and ran `async_fn` to completion, producing `T`.
    Completed(T),

    /// The `current_thread` Tokio runtime failed to initialize before `async_fn` ran.
    FailedToStart(miette::Report),

    /// The `current_thread` Tokio runtime started, but `async_fn` panicked while
    /// running on the scoped OS thread.
    Panicked(miette::Report),
}

impl<T> TokioRuntimeOutcome<T> {
    fn failed_to_start(err: &std::io::Error) -> Self {
        let report = miette::miette!("Failed to create tokio runtime: {err}");
        Self::FailedToStart(report)
    }

    fn panicked(panic_payload: &Box<dyn std::any::Any + Send + 'static>) -> Self {
        let panic_msg = match (
            panic_payload.downcast_ref::<&str>(),
            panic_payload.downcast_ref::<String>(),
        ) {
            (Some(&s), _) => s,
            (_, Some(s)) => s.as_str(),
            (None, None) => "unknown panic payload",
        };

        let report = miette::miette!("Scoped OS thread panicked: {panic_msg}");
        Self::Panicked(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_run_in_tokio_completed_from_sync_context() {
        let outcome = run_in_tokio(async || 42);
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(42)));
    }

    #[tokio::test]
    async fn test_run_in_tokio_completed_from_async_context() {
        let outcome = run_in_tokio(async || 42);
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(42)));
    }

    #[test]
    fn test_run_in_tokio_non_send_future() {
        let outcome = run_in_tokio(async || {
            // `Rc` is `!Send` and lives across the `.await` point inside the
            // single-threaded runtime.
            let non_send_rc = Rc::new(99);
            tokio::task::yield_now().await;
            *non_send_rc
        });
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(99)));
    }

    #[test]
    fn test_run_in_tokio_panicked() {
        let outcome = run_in_tokio(async || -> u32 {
            panic!("simulated panic inside run_in_tokio");
        });

        match outcome {
            TokioRuntimeOutcome::Panicked(report) => {
                assert!(
                    report
                        .to_string()
                        .contains("simulated panic inside run_in_tokio")
                );
            }
            _ => panic!("Expected TokioRuntimeOutcome::Panicked"),
        }
    }

    #[test]
    fn test_failed_to_start() {
        let io_err = std::io::Error::other("simulated runtime init failure");

        let outcome = TokioRuntimeOutcome::<()>::failed_to_start(&io_err);

        match outcome {
            TokioRuntimeOutcome::FailedToStart(report) => {
                assert!(
                    report
                        .to_string()
                        .contains("simulated runtime init failure")
                );
            }
            _ => panic!("Expected TokioRuntimeOutcome::FailedToStart"),
        }
    }
}

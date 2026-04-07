// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Control flow signal for loops and threads.
///
/// A unified type for indicating whether a loop or thread should continue processing or
/// stop. Used across:
/// - [Main event loop] (terminal window).
/// - [`MioPollWorker`] thread (input handling).
/// - [PTY input processing] loop.
///
/// # Type Parameters
///
/// - `E`: The error type to return if the loop stops due to an error. Defaults to `()`.
///
/// # Result Conversion
///
/// This type supports implicit conversion from [`Result<(), E>`] via [`.into()`],
/// allowing for a fluid functional style when working with locks and loops.
///
/// # Usage Guidance
///
/// To maintain high readability and low cognitive load, follow these conventions:
/// 1. **Errors**: Prefer `Err(...).into()` for idiomatic error propagation.
/// 2. **Closure Scopes**: Use `lock.write(|state| { ... Ok(()) }).into()` to cleanly
///    terminate a meaningful inner scope.
/// 3. **Early Returns**: Use [`Self::Continue`] or [`Self::Stop`] directly for early
///    returns in a state machine loop. This is semantically stronger than `Ok(()).into()`
///    or `Err(()).into()`, which are discouraged.
///
/// [`.into()`]: Into::into
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
/// [Main event loop]: crate::main_event_loop_impl
/// [PTY input processing]: crate::pty_session::pty_session_builder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Continuation<E = ()> {
    /// Continue to the next iteration.
    #[default]
    Continue,

    /// Stop processing and exit the loop/thread (Normal exit).
    Stop,

    /// Signal that OS resources are corrupted but the thread & process is viable. Request
    /// the controlling framework to tear down and recreate the current worker in-place
    /// (same thread, fresh worker, since OS resources need to be reallocated). This
    /// triggers the self-healing restart mechanism (not to be confused with the relaunch
    /// mechanism with new thread).
    ///
    /// Only consumed by the RRT framework's [`run_worker_loop()`]. Non-[`RRT`] callers
    /// should use [`Continue`] or [`Stop`].
    ///
    /// [`Continue`]: Self::Continue
    /// [`RRT`]: crate::RRT
    /// [`run_worker_loop()`]: crate::run_worker_loop
    /// [`Stop`]: Self::Stop
    Restart,

    /// Stop processing and return an error (Abnormal exit).
    ReturnError(E),
}

/// Convenience conversion from [`Result`] to [`Continuation`].
///
/// - `Ok(())` maps to [`Continuation::Continue`].
/// - `Err(e)` maps to [`Continuation::ReturnError(e)`].
///
/// [`Continuation::ReturnError(e)`]: Continuation::ReturnError
impl<E> From<Result<(), E>> for Continuation<E> {
    fn from(result: Result<(), E>) -> Self {
        match result {
            Ok(()) => Self::Continue,
            Err(e) => Self::ReturnError(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum ContainsResult {
    #[default]
    DoesNotContain,
    DoesContain,
}

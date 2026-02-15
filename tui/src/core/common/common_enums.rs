// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Control flow signal for loops and threads.
///
/// A unified type for indicating whether a loop or thread should continue processing or
/// stop. Used across:
/// - [Main event loop] (terminal window).
/// - [`MioPollWorker`] thread (input handling).
/// - [PTY input processing] loop.
///
/// [Main event loop]: crate::main_event_loop_impl
/// [PTY input processing]: crate::core::pty::pty_read_write
/// [`MioPollWorker`]: crate::terminal_lib_backends::MioPollWorker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Continuation {
    /// Continue to the next iteration.
    #[default]
    Continue,

    /// Stop processing and exit the loop/thread.
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
    /// [`RRT`]: crate::core::resilient_reactor_thread::RRT
    /// [`Stop`]: Self::Stop
    /// [`run_worker_loop()`]: crate::core::resilient_reactor_thread::run_worker_loop
    Restart,
}

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum ContainsResult {
    #[default]
    DoesNotContain,
    DoesContain,
}

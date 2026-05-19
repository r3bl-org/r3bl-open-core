// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words maxfiles taskthreads

//! Public API types for the Resilient Reactor Thread (RRT) pattern. See
//! [`SubscribeError`] and [`BroadcastSender`] for details.

use crate::resilient_reactor_thread::RRTEvent;
use tokio::sync::broadcast;

/// [Sender]-half of the [broadcast channel] that carries [`RRTEvent<E>`] to subscribers.
///
/// [broadcast channel]: tokio::sync::broadcast
/// [Sender]: tokio::sync::broadcast::Sender
pub type BroadcastSender<E> = broadcast::Sender<RRTEvent<E>>;

/// The outcome of a restart retry loop. Used to make the control flow in
/// [`run_worker_loop()`] more readable by avoiding ambiguous `bool` returns.
///
/// [`run_worker_loop()`]: super::run_worker_loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryLoopExhaustion {
    /// The restart budget was exhausted before a new worker could be created.
    Yes,
    /// A new worker was successfully created before the budget was exhausted.
    No,
}

/// The reason why an orderly stop of the dedicated thread was initiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The framework loop detected that all subscribers have dropped.
    ZeroReceivers,
    /// The worker implementation returned [`Continuation::Stop`] (e.g., due to EOF).
    ///
    /// [`Continuation::Stop`]: crate::Continuation::Stop
    WorkerRequested,
}

/// Errors from [`RRT::try_subscribe()`].
///
/// Each variant represents a distinct failure mode with a dedicated OS specific (where
/// appropriate) [diagnostic code] and actionable help text. The three failure modes are:
///
/// | Variant             | Cause                                                                           | Recoverable? |
/// | :------------------ | :------------------------------------------------------------------------------ | :----------- |
/// | [`MutexPoisoned`]   | A prior thread panicked while holding an internal RRT lock                      | No           |
/// | [`WorkerCreation`]  | [`RRTWorker::create_and_register_os_sources()`] failed (OS resource exhaustion) | Maybe        |
/// | [`ThreadSpawn`]     | [`std::thread::Builder::spawn()`] failed (thread limits)                        | Maybe        |
///
/// [`MutexPoisoned`]: Self::MutexPoisoned
/// [`RRT::try_subscribe()`]: super::RRT::try_subscribe
/// [`RRTWorker::create_and_register_os_sources()`]: crate::RRTWorker::create_and_register_os_sources
/// [`ThreadSpawn`]: Self::ThreadSpawn
/// [`WorkerCreation`]: Self::WorkerCreation
/// [diagnostic code]: miette::Diagnostic::code
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SubscribeError {
    /// Internal state mutex was poisoned by a prior thread panic.
    #[error("RRT internal state mutex poisoned")]
    #[diagnostic(
        code(r3bl_tui::rrt::mutex_poisoned),
        help(
            "A prior thread panicked while holding an RRT lock. \
             Consider restarting the application."
        )
    )]
    MutexPoisoned,

    /// [`RRTWorker::create_and_register_os_sources()`] failed to acquire OS resources.
    ///
    /// The inner [`miette::Report`] preserves the full error chain from the worker
    /// implementation (e.g., [`PollCreationError`], [`SoftwareInterruptCreationError`]). Access it
    /// via pattern matching.
    ///
    /// [`PollCreationError`]: crate::terminal_lib_backends::PollCreationError
    /// [`RRTWorker::create_and_register_os_sources()`]: crate::RRTWorker::create_and_register_os_sources
    /// [`SoftwareInterruptCreationError`]: crate::terminal_lib_backends::SoftwareInterruptCreationError
    #[error("Failed to create worker thread resources")]
    #[diagnostic(code(r3bl_tui::rrt::worker_creation))]
    #[cfg_attr(
        target_os = "linux",
        diagnostic(help(
            "Check OS resource limits - \
             use `ulimit -n` for file descriptors, \
             `cat /proc/sys/fs/file-max` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "macos",
        diagnostic(help(
            "Check OS resource limits - \
             use `ulimit -n` for file descriptors, \
             `launchctl limit maxfiles` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "windows",
        diagnostic(help(
            "Check OS resource limits - \
             Windows handle limits are typically high, \
             but check Task Manager for handle count"
        ))
    )]
    WorkerCreation(miette::Report),

    /// [`std::thread::Builder::spawn()`] failed.
    #[error("Failed to spawn RRT worker thread")]
    #[diagnostic(code(r3bl_tui::rrt::thread_spawn))]
    #[cfg_attr(
        target_os = "linux",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check `ulimit -u` for per-user limit, \
             `cat /proc/sys/kernel/threads-max` for system-wide limit"
        ))
    )]
    #[cfg_attr(
        target_os = "macos",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check `ulimit -u` for per-user limit, \
             `sysctl kern.num_taskthreads` for per-process limit"
        ))
    )]
    #[cfg_attr(
        target_os = "windows",
        diagnostic(help(
            "The system may have reached its thread limit - \
             check Task Manager for thread count, \
             or use `Get-Process` in PowerShell to inspect per-process threads"
        ))
    )]
    ThreadSpawn(#[source] std::io::Error),
}

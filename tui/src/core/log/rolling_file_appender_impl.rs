// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DeadlockPreventionPolicy::PanicOnAnyLockNesting, MkdirOptions, ScopedMutex,
            scoped_mutex, try_mkdir};
use std::{path::PathBuf, sync::LazyLock};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

/// Keeps the background writer thread alive for the lifetime of the process. If this
/// guard is dropped, the background thread exits and buffered logs are flushed.
///
/// Uses [`ScopedMutex<Option>`] so [`GlobalLogFileGuard`] can `.take()` it on drop to
/// flush.
///
/// # Architectural Rationale for [`PanicOnAnyLockNesting`] (ANY)
///
/// We use the most restrictive policy (`ANY`) here because:
/// 1. **Foundational Service Protection**: Logging is a foundational system service that
///    is initialized and shut down exactly once. Using `ANY` protects these critical
///    lifecycle events from interference, ensuring they happen before the thread begins
///    acquiring more granular `SPECIFIC` locks (like [`Readline`] or [`OutputDevice`]).
///    This enforces a loud and fast failure if any other system tries to nest locks
///    during logging setup or teardown.
/// 2. **Not in the Hot Path**: This mutex is **never** acquired for every log message.
///    The [`NonBlocking`] writer handles actual logging concurrently. This mutex only
///    protects the [`WorkerGuard`] during [`try_create()`] and [`drop()`].
/// 3. **Shutdown Resilience**: The [`Drop`] implementation uses [`lock_raw()`] to bypass
///    all ledger checks and poison-safety panics, ensuring logs are flushed even during a
///    [Double Panic Abort] scenario.
///
/// [`lock_raw()`]: crate::ScopedMutex::lock_raw
/// [`OutputDevice`]: crate::OutputDevice
/// [`Readline`]: crate::Readline
/// [Double Panic Abort]: crate#the-double-panic-abort-risk
pub static ROLLING_LOG_FILE_WRITER_GUARD: LazyLock<
    ScopedMutex<Option<WorkerGuard>, { PanicOnAnyLockNesting }>,
> = LazyLock::new(|| scoped_mutex!(ANY, None));

/// Creates a non-blocking file writer backed by a rolling file appender.
///
/// The returned [`NonBlocking`] writer sends log events to a dedicated background thread
/// that performs the actual disk I/O, so callers (including the [`mio-poller`] thread)
/// are never blocked by file writes.
///
/// The [`WorkerGuard`] is stored in a process-global [`Mutex`] to keep the background
/// thread alive. To flush buffered logs at shutdown, hold a [`GlobalLogFileGuard`] on
/// `main()`'s stack - its [`Drop`] impl takes the guard from the static and drops it,
/// triggering a flush.
///
/// # Errors
///
/// Returns an error if:
/// - The path has no parent directory.
/// - The path has no file name.
/// - Insufficient permissions to access the file or directory.
/// - This method has already been called once.
///
/// # Panics
///
/// Panics if the guard mutex is poisoned.
///
/// [`mio-poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
/// [`Mutex<Option>`]: std::sync::Mutex
/// [`Mutex`]: std::sync::Mutex
#[allow(clippy::unwrap_in_result, clippy::redundant_closure_for_method_calls)]
pub fn try_create(path_str: &str) -> miette::Result<NonBlocking> {
    // Can only init this once per process.
    if ROLLING_LOG_FILE_WRITER_GUARD.read(Option::is_some) {
        miette::bail!("Rolling file appender already created");
    }

    let path = PathBuf::from(&path_str);

    let parent = path.parent().ok_or_else(|| {
        miette::miette!(
            format!("Can't access current folder {}. It might not exist, or don't have required permissions.",
            path.display())
        )
    })?;

    if !parent.as_os_str().is_empty() {
        try_mkdir(parent, MkdirOptions::CreateIntermediateDirectories)?;
    }

    let file_stem = path.file_name().ok_or_else(|| {
        miette::miette!(format!(
        "Can't access file name {}. It might not exist, or don't have required permissions.",
        path.display()
    ))
    })?;

    let rolling_file_appender = tracing_appender::rolling::never(parent, file_stem);
    let (non_blocking_rolling_file_appender, guard) =
        tracing_appender::non_blocking(rolling_file_appender);

    // Save the guard so the background thread lives for the process lifetime.
    ROLLING_LOG_FILE_WRITER_GUARD
        .write(|slot| *slot = Some(guard));

    Ok(non_blocking_rolling_file_appender)
}

/// [`RAII`] sentinel that flushes the non-blocking log file writer on drop.
///
/// This struct is a zero-cost sentinel — constructing it does nothing, and its [`Drop`]
/// just reaches into the static. If no file appender was created (e.g.,
/// [`DisplayPreference::Stdout`]), the static is [`None`] and the drop is a no-op.
///
/// Hold this on `main()`'s stack to guarantee buffered log events are flushed when the
/// process exits normally. When dropped, it takes the [`WorkerGuard`] from the
/// process-global static and drops it, which signals the background writer thread to
/// flush and exit.
///
/// Returned by [`try_initialize_logging_global`] and
/// [`try_initialize_logging_thread_local`].
///
/// [`DisplayPreference::Stdout`]: crate::log::DisplayPreference::Stdout
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`try_initialize_logging_global`]: crate::log::try_initialize_logging_global
/// [`try_initialize_logging_thread_local`]:
///     crate::log::try_initialize_logging_thread_local
#[derive(Debug)]
pub struct GlobalLogFileGuard;

impl Drop for GlobalLogFileGuard {
    fn drop(&mut self) {
        ROLLING_LOG_FILE_WRITER_GUARD.lock_raw_poison_safe(|slot| {
            // WorkerGuard::drop() flushes the buffer.
            drop(slot.take());
        });
    }
}

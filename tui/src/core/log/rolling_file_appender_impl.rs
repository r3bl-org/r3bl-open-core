// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{MkdirOptions, try_mkdir};
use std::{path::PathBuf, sync::Mutex};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

/// Keeps the background writer thread alive for the lifetime of the process. If this
/// guard is dropped, the background thread exits and buffered logs are flushed.
///
/// Uses [`Mutex<Option>`] so [`GlobalLogFileGuard`] can `.take()` it on drop to flush.
static ROLLING_LOG_FILE_WRITER_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

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
#[allow(clippy::unwrap_in_result)]
pub fn try_create(path_str: &str) -> miette::Result<NonBlocking> {
    // Can only init this once per process.
    if ROLLING_LOG_FILE_WRITER_GUARD.lock().unwrap().is_some() {
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
    *ROLLING_LOG_FILE_WRITER_GUARD.lock().unwrap() = Some(guard);

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
        if let Ok(mut slot) = ROLLING_LOG_FILE_WRITER_GUARD.lock() {
            drop(slot.take()); // WorkerGuard::drop() flushes the buffer.
        }
    }
}

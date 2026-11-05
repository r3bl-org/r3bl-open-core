// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::thread::JoinHandle;

/// Marker struct that spawns a dedicated [`std::thread`] running its own single-threaded
/// Tokio runtime, [`current_thread`][ct], to execute a given [`async_fn`][af], in either
/// mode:
/// - [Blocking][`Self::run_blocking`]
///   - Uses a scoped OS thread ([`std::thread::scope`]) and joins it immediately before
///     returning [`TokioRuntimeOutcome<T>`].
/// - [Non-blocking][`Self::run_non_blocking`]
///   - Spawns a named OS thread ([`std::thread::Builder::spawn`]) and returns
///     [`StdThreadTokioJoinHandle<T>`][jh] immediately so the caller can continue running
///     and join it later.
///
/// # Use Cases
///
/// You might have the same logic that you need to run in a synchronous context as well as
/// an asynchronous context. This happens in callers of the `self_upgrade` module (in the
/// `r3bl-cmdr` crate), which are typically the `main()` function of a binary. Because
/// `main()` can be either synchronous (`fn main()`) or asynchronous (`#[tokio::main]
/// async fn main()`), this struct allows the exact same self-upgrade entry points
/// (`UpgradeJourney::start` and `StartOutcome::handle_upgrade_on_exit`) to be called
/// from both contexts without writing the logic twice or exposing [`async`] "function
/// coloring" at the API boundary.
///
/// # Why a Dedicated OS Thread + [`current_thread`][ct] Tokio Runtime?
///
/// Spawning an isolated [`std::thread`] with its own single-threaded
/// ([`current_thread`][ct]) Tokio runtime solves two core design problems when packaging
/// async logic behind a synchronous API:
///
/// 1. **Context-agnostic callers (no nested-runtime panics)**:
///    - If a synchronous API is called from inside an active `#[tokio::main]` runtime,
///      reusing the caller's runtime via [`Handle::block_on`] immediately panics with
///      `"Cannot start a runtime from within a runtime"`.
///    - Using [`tokio::task::block_in_place`] also panics whenever the caller happens to
///      run on a single-threaded runtime (such as `#[tokio::main(flavor =
///      "current_thread")]` or `#[tokio::test]`).
///    - Spawning a dedicated [`std::thread`] with its own isolated [`current_thread`][ct]
///      runtime guarantees [`Runtime::block_on`][bo] succeeds regardless of whether the
///      calling thread has a Tokio runtime or what flavor it uses.
///    - Because Tokio runtime contexts are strictly thread-local, spinning up and tearing
///      down this isolated [`current_thread`][ct] runtime has zero side effects on the
///      caller's main `#[tokio::main]` runtime.
/// 2. **Blocking vs. Non-blocking API boundary (instead of sync vs. async)**:
///    - Rather than splitting the API by caller context (`sync` vs. `async`), the API
///      splits by **when the work completes relative to the caller**:
///      - [`Self::run_blocking`] waits for completion immediately and uses
///        [`std::thread::scope`] so `async_fn` can borrow stack references without
///        `'static` bounds or [`Arc`] cloning.
///      - [`Self::run_non_blocking`] runs in the background while the CLI/TUI application
///        executes and returns a [`StdThreadTokioJoinHandle`][jh] to be joined at
///        application exit.
///
/// # Why [`AsyncFnOnce`][af] Instead of a Sync Closure?
///
/// The closure passed to [`Self::run_blocking`] and [`Self::run_non_blocking`] is an
/// [`AsyncFnOnce`][af] (rather than a synchronous [`FnOnce`]) because the underlying
/// operations orchestrated by callers like `self_upgrade` depend on `r3bl_tui` and Tokio
/// building blocks that require an active Tokio reactor:
/// 1. **[`Spinner::try_start`]** and **[`Spinner::await_shutdown`]** (spawns a background
///    animation task via [`tokio::spawn`] driven by [`tokio::time::interval`]).
/// 2. **[`choose`]** in `prompt_user_to_upgrade` (drives an async [`EventStream`] loop
///    via [`enter_event_loop_async`]).
/// 3. **[`PtySessionBuilder::start_async`]** in `run_cargo_install_with_progress` and
///    `run_rustup_update` (spawns [`tokio::task::spawn_blocking`] bridges over
///    [`tokio::sync::mpsc`] channels and races output against [`tokio::signal::ctrl_c`]
///    via [`tokio::select!`]).
/// 4. **[`reqwest::Client`]** + **[`tokio::time::timeout`]** + **[`tokio::select!`]** in
///    `download.rs` and `fetch_manifest.rs` (streams HTTP chunks and races against
///    [`tokio::signal::ctrl_c`] for cooperative cancellation).
///
/// > **Note on Terminal State at Application Exit (TUI Apps like `edi` and `giti`)**:
/// > When running exit work via [`Self::run_blocking`] after a TUI application finishes,
/// > the main UI loop ([`TerminalWindow::main_event_loop`], [`ReadlineAsyncContext`], or
/// > [`choose`]) has already dropped its RAII guards and restored the terminal from
/// > [raw mode] back to [cooked mode]. This is required so that:
/// > - [`choose`] can cleanly bracket its own temporary [raw mode] session for modal
/// > prompts and restore [cooked mode] afterward.
/// > - Standalone [`Spinner`] output and [`tokio::signal::ctrl_c`] work normally in
/// > [cooked mode] (where the OS terminal driver's `ISIG` flag translates `Ctrl+C`
/// > into [`SIGINT`] and `OPOST`/`ONLCR` formats newlines).
///
/// [`Arc`]: std::sync::Arc
/// [`async`]: https://doc.rust-lang.org/std/keyword.async.html
/// [`choose`]: crate::choose
/// [`enter_event_loop_async`]: crate::enter_event_loop_async
/// [`EventStream`]: crossterm::event::EventStream
/// [`Handle::block_on`]: tokio::runtime::Handle::block_on
/// [`PtySessionBuilder::start_async`]: crate::core::pty::PtySessionBuilder::start_async
/// [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
/// [`reqwest::Client`]: reqwest::Client
/// [`SIGINT`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGINT
/// [`Spinner::await_shutdown`]: crate::Spinner::await_shutdown
/// [`Spinner::try_start`]: crate::Spinner::try_start
/// [`Spinner`]: crate::Spinner
/// [`TerminalWindow::main_event_loop`]: crate::tui::TerminalWindow::main_event_loop
/// [`tokio::select!`]: tokio::select
/// [`tokio::signal::ctrl_c`]: tokio::signal::ctrl_c
/// [`tokio::spawn`]: tokio::spawn
/// [`tokio::sync::mpsc`]: tokio::sync::mpsc
/// [`tokio::task::block_in_place`]: tokio::task::block_in_place
/// [`tokio::task::spawn_blocking`]: tokio::task::spawn_blocking
/// [`tokio::time::interval`]: tokio::time::interval
/// [`tokio::time::timeout`]: tokio::time::timeout
/// [af]: std::ops::AsyncFnOnce
/// [bo]: tokio::runtime::Runtime::block_on
/// [cooked mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
/// [ct]: tokio::runtime::Builder::new_current_thread
/// [jh]: StdThreadTokioJoinHandle
/// [raw mode]: mod@crate::terminal_raw_mode#raw-mode-vs-cooked-mode
#[derive(Debug)]
pub struct StdThreadTokioRunner;

impl StdThreadTokioRunner {
    /// Creates a dedicated [scoped OS thread][st], starts a [`current_thread`][ct] Tokio
    /// runtime on it, and blocks until `async_fn` executes to completion.
    ///
    /// # Thread Bounds (`Send` vs. `!Send`)
    ///
    /// - **`F` (`Send + AsyncFnOnce() -> T`)**: Must be [`Send`] because `async_fn` moves
    ///   **into** the spawned OS thread. However, the [`Future`] produced when
    ///   `async_fn()` is called inside the thread does **not** need to be [`Send`],
    ///   because it is created, polled, and dropped entirely on that single thread.
    /// - **`T` (`Send`)**: Must be [`Send`] because
    ///   [`TokioRuntimeOutcome::Completed(T)`][co] moves **out of** the spawned thread
    ///   back to the caller when the scoped thread joins.
    ///
    /// # Returns
    ///
    /// - [`TokioRuntimeOutcome::Completed`]: The `current_thread` Tokio runtime
    ///   initialized and ran `async_fn` to completion, returning its output `T`.
    /// - [`TokioRuntimeOutcome::FailedToStart`]: Building the `current_thread` Tokio
    ///   runtime failed before `async_fn` could run.
    /// - [`TokioRuntimeOutcome::Panicked`]: The Tokio runtime started, but `async_fn`
    ///   panicked during execution.
    ///
    /// [co]: TokioRuntimeOutcome::Completed
    /// [ct]: tokio::runtime::Builder::new_current_thread
    /// [st]: std::thread::scope
    pub fn run_blocking<T: Send, F: Send + AsyncFnOnce() -> T>(
        async_fn: F,
    ) -> TokioRuntimeOutcome<T> {
        std::thread::scope(|scoped_thread| {
            // Spawn a new thread and execute the async function in a new Tokio runtime.
            let join_handle =
                scoped_thread.spawn(|| Self::execute_in_current_thread_runtime(async_fn));

            // Return the result from the join handle.
            match join_handle.join() {
                Ok(execution) => execution,
                Err(panic_payload) => {
                    TokioRuntimeOutcome::panicked(SCOPED_THREAD_NAME, &panic_payload)
                }
            }
        })
    }

    /// Spawns a named OS thread, starts a [`current_thread`][ct] Tokio runtime on it, and
    /// executes `async_fn` to completion in the background, returning a
    /// [`StdThreadTokioJoinHandle<T>`][jh] immediately without blocking the caller.
    ///
    /// Unlike [`Self::run_blocking`] (which uses [`std::thread::scope`] and immediately
    /// joins the scoped thread), `run_non_blocking` returns immediately so the
    /// caller can continue running and join the [`StdThreadTokioJoinHandle`][jh] later
    /// (for example, at application exit).
    ///
    /// # Thread Bounds (`'static` + `Send` vs. `!Send`)
    ///
    /// - **`F` (`Send + 'static + AsyncFnOnce() -> T`)**: Must be [`Send`] + `'static`
    ///   because the spawned thread may outlive the caller's stack frame. However, the
    ///   [`Future`] produced when `async_fn()` is called inside the thread does **not**
    ///   need to be [`Send`], because it is created, polled, and dropped entirely on that
    ///   single thread.
    /// - **`T` (`Send + 'static`)**: Must be [`Send`] + `'static` because
    ///   [`TokioRuntimeOutcome::Completed(T)`][co] moves **out of** the spawned thread
    ///   back to the caller when [`StdThreadTokioJoinHandle::join`] is eventually called.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the operating system fails to spawn the named OS
    /// thread via [`std::thread::Builder::spawn`].
    ///
    /// [co]: TokioRuntimeOutcome::Completed
    /// [ct]: tokio::runtime::Builder::new_current_thread
    /// [jh]: StdThreadTokioJoinHandle
    pub fn run_non_blocking<T, F>(
        thread_name: impl Into<String>,
        async_fn: F,
    ) -> std::io::Result<StdThreadTokioJoinHandle<T>>
    where
        T: Send + 'static,
        F: Send + 'static + AsyncFnOnce() -> T,
    {
        std::thread::Builder::new()
            .name(thread_name.into())
            .spawn(move || Self::execute_in_current_thread_runtime(async_fn))
            .map(StdThreadTokioJoinHandle)
    }

    /// Builds a [`current_thread`][ct] Tokio runtime with all drivers enabled (`I/O`,
    /// time, and signal) and drives `async_fn()` to completion via
    /// [`Runtime::block_on`][bo].
    ///
    /// Called inside the dedicated OS thread spawned by [`Self::run_blocking`] or
    /// [`Self::run_non_blocking`]. Any panic thrown while polling `async_fn()` unwinds
    /// out of [`Runtime::block_on`][bo] and is caught by the enclosing OS thread's
    /// `join()` handler, which converts it into [`TokioRuntimeOutcome::Panicked`].
    ///
    /// [bo]: tokio::runtime::Runtime::block_on
    /// [ct]: tokio::runtime::Builder::new_current_thread
    fn execute_in_current_thread_runtime<T, F: AsyncFnOnce() -> T>(
        async_fn: F,
    ) -> TokioRuntimeOutcome<T> {
        let res_runtime_create = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        match res_runtime_create {
            Ok(runtime) => {
                let outcome = runtime.block_on(async_fn());
                TokioRuntimeOutcome::Completed(outcome)
            }
            Err(err) => TokioRuntimeOutcome::failed_to_start(err),
        }
    }
}

/// Handle returned by [`StdThreadTokioRunner::run_non_blocking`].
///
/// Wraps [`std::thread::JoinHandle`] so [`Self::join`] maps thread panics directly into
/// [`TokioRuntimeOutcome::Panicked`], matching [`StdThreadTokioRunner::run_blocking`].
#[must_use]
#[derive(Debug)]
pub struct StdThreadTokioJoinHandle<T>(JoinHandle<TokioRuntimeOutcome<T>>);

impl<T> StdThreadTokioJoinHandle<T> {
    /// Waits for the spawned OS thread to finish and returns its [`TokioRuntimeOutcome`].
    pub fn join(self) -> TokioRuntimeOutcome<T> {
        let thread_name = self
            .0
            .thread()
            .name()
            .unwrap_or(UNSCOPED_THREAD_NAME)
            .to_string();
        match self.0.join() {
            Ok(execution) => execution,
            Err(panic_payload) => {
                TokioRuntimeOutcome::panicked(&thread_name, &panic_payload)
            }
        }
    }
}

/// Outcome of executing an async closure on a [`std::thread`]'s Tokio runtime via
/// [`StdThreadTokioRunner::run_blocking`] or [`StdThreadTokioRunner::run_non_blocking`].
#[must_use]
#[derive(Debug)]
pub enum TokioRuntimeOutcome<T> {
    /// The Tokio runtime initialized and ran `async_fn` to completion, producing `T`.
    Completed(T),

    /// The `current_thread` Tokio runtime failed to initialize before `async_fn` ran.
    FailedToStart(miette::Report),

    /// The `current_thread` Tokio runtime started, but `async_fn` panicked while
    /// running on the OS thread.
    Panicked(miette::Report),
}

impl<T> TokioRuntimeOutcome<T> {
    fn failed_to_start(err: std::io::Error) -> Self {
        let report =
            miette::Report::from_err(err).wrap_err("Failed to create tokio runtime");
        Self::FailedToStart(report)
    }

    fn panicked(
        thread_name: &str,
        panic_payload: &Box<dyn std::any::Any + Send + 'static>,
    ) -> Self {
        let panic_msg = match (
            panic_payload.downcast_ref::<&str>(),
            panic_payload.downcast_ref::<String>(),
        ) {
            (Some(&s), _) => s,
            (_, Some(s)) => s.as_str(),
            (None, None) => "unknown panic payload",
        };

        let report = miette::miette!("{thread_name} panicked: {panic_msg}");
        Self::Panicked(report)
    }
}

const SCOPED_THREAD_NAME: &str = "Scoped OS thread";
const UNSCOPED_THREAD_NAME: &str = "Unscoped OS thread";

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_run_blocking_completed_from_sync_context() {
        let outcome = StdThreadTokioRunner::run_blocking(async || 42);
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(42)));
    }

    #[tokio::test]
    async fn test_run_blocking_completed_from_async_context() {
        let outcome = StdThreadTokioRunner::run_blocking(async || 42);
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(42)));
    }

    #[test]
    fn test_run_blocking_non_send_future() {
        let outcome = StdThreadTokioRunner::run_blocking(async || {
            // `Rc` is `!Send` and lives across the `.await` point inside the
            // single-threaded runtime.
            let non_send_rc = Rc::new(99);
            tokio::task::yield_now().await;
            *non_send_rc
        });
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(99)));
    }

    #[test]
    fn test_run_blocking_panicked() {
        let outcome = StdThreadTokioRunner::run_blocking(async || -> u32 {
            panic!("simulated panic inside run_blocking");
        });

        match outcome {
            TokioRuntimeOutcome::Panicked(report) => {
                let msg = report.to_string();
                assert!(msg.contains(&format!("{SCOPED_THREAD_NAME} panicked")));
                assert!(msg.contains("simulated panic inside run_blocking"));
            }
            _ => panic!("Expected TokioRuntimeOutcome::Panicked"),
        }
    }

    #[test]
    fn test_run_non_blocking_completed() {
        let handle =
            StdThreadTokioRunner::run_non_blocking("test-run-non-blocking", async || {
                tokio::task::yield_now().await;
                42
            })
            .unwrap();

        let outcome = handle.join();
        assert!(matches!(outcome, TokioRuntimeOutcome::Completed(42)));
    }

    #[test]
    fn test_run_non_blocking_panicked() {
        let handle = StdThreadTokioRunner::run_non_blocking(
            "test-non-blocking-panic",
            async || -> u32 {
                panic!("simulated panic inside run_non_blocking");
            },
        )
        .unwrap();

        let outcome = handle.join();
        match outcome {
            TokioRuntimeOutcome::Panicked(report) => {
                let msg = report.to_string();
                assert!(msg.contains("test-non-blocking-panic panicked"));
                assert!(msg.contains("simulated panic inside run_non_blocking"));
            }
            _ => panic!("Expected TokioRuntimeOutcome::Panicked"),
        }
    }

    #[test]
    fn test_failed_to_start() {
        let io_err = std::io::Error::other("simulated runtime init failure");

        let outcome = TokioRuntimeOutcome::<()>::failed_to_start(io_err);

        match outcome {
            TokioRuntimeOutcome::FailedToStart(report) => {
                let msg = format!("{report:#}");
                assert!(msg.contains("Failed to create tokio runtime"));
                assert!(msg.contains("simulated runtime init failure"));
            }
            _ => panic!("Expected TokioRuntimeOutcome::FailedToStart"),
        }
    }
}

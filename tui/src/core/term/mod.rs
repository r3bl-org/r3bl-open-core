// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words isatty winsize tcgetwinsize

//! # Terminal Interactivity and Size Detection
//!
//! This module provides a centralized, backend-aware API for detecting terminal
//! interactivity and size. It is the single source of truth for [`TTY`] detection across
//! the crate, ensuring consistent behavior across different terminal backends
//! ([`Crossterm`] and [`DirectToAnsi`]) and OSes.
//!
//! ## Why this module exists
//!
//! Different parts of this codebase needs to know whether the [`TUI`] or
//! [`readline_async`] app is running in a real terminal:
//!
//! - **[`Spinner`]** needs to know if [`stdout`] is a [`TTY`] before writing animations.
//!   Writing spinner frames to a pipe or file produces garbage.
//! - **[`ReadlineAsyncContext`]** needs both [`stdin`] (to read keystrokes) and
//!   [`stdout`] (to render the prompt) to be [`TTY`]s.
//! - **[Color detection]** queries [`stdout`] / [`stderr`] to decide whether to emit
//!   [`ANSI`] color codes. Without this, running examples via `cargo run` on Windows
//!   produced colorless output because `cargo` redirects streams and they were falsely
//!   reported as non-[`TTY`]s. See [Windows `cargo run` workaround] below.
//! - **[Raw mode]** needs a terminal file descriptor to call [`tcsetattr`]. If [`stdin`]
//!   is redirected, it falls back to [`/dev/tty`].
//! - **Tests** need to distinguish a real terminal from CI environments (where streams
//!   are pipes) to adjust assertions like expected color depth.
//!
//! Before this module existed, each component performed its own [`isatty`] check with
//! different logic, leading to inconsistencies (e.g., the app thinking it was interactive
//! while color detection disagreed).
//!
//! ## Interactivity levels
//!
//! The crate distinguishes between three levels of interactivity:
//!
//! 1. **Input Interactivity** ([`is_input_interactive()`]): Can we read keystrokes and
//!    mouse events from [`stdin`]?
//! 2. **Output Interactivity** ([`is_output_interactive()`]): Can we render the TUI to
//!    [`stdout`]? This only checks [`stdout`], as the TUI is rendered there. Redirection
//!    of [`stderr`] does not disable the TUI.
//! 3. **Full Interactivity** ([`is_fully_interactive()`]): Are all three streams
//!    ([`stdin`], [`stdout`], [`stderr`]) connected to a [`TTY`]? This is the strictest
//!    check, used primarily by tests to verify a "clean" terminal environment for
//!    assertions like color depth.
//!
//! ## Windows `cargo run` workaround
//!
//! On Windows, `cargo run` redirects standard streams, causing them to be falsely
//! reported as non-[`TTY`]s. In the past, this meant running examples in this crate using
//! the following command would produce output without colors or interactive features.
//! ```bash
//! cargo run --example pty_mux_example
//! ```
//!
//! To fix this, the low-level [`is_tty_stdin()`], [`is_tty_stdout()`], and
//! [`is_tty_stderr()`] helpers assume streams are interactive when they detect execution
//! under `cargo` (via the `CARGO` or `CARGO_PKG_NAME` environment variables). This
//! ensures all downstream consumers ([Color detection], [Raw mode], [`Spinner`]) benefit
//! from the workaround.
//!
//! Here are some links to describe what happens in Windows leading to the strange `cargo
//! run` behavior:
//! - [`GetConsoleMode`] - the Win32 API that [`IsTerminal`] uses under the hood. Returns
//!   an error for redirected handles, which is how Rust detects non-[`TTY`] streams.
//! - [Console Handles] - how Windows manages stdin/stdout/stderr console handles and what
//!   happens when they are redirected.
//! - [`IsTerminal`] - Rust's cross-platform trait that wraps [`isatty`] (Unix) and
//!   [`GetConsoleMode`] (Windows).
//!
//! ## [`stderr`] redirection disclaimer
//!
//! This crate is designed to **never write directly to [`stderr`]** as it would clobber
//! the [`TUI`] or [`readline_async`] app's output.
//!
//! All output goes through [`SharedWriter`] (for [`readline_async`] apps) or
//! [`OffscreenBuffer`] (for [`TUI`] apps), which route content to [`stdout`] in a
//! terminal-safe way.
//!
//! Logging is handled internally via [`TracingConfig`] - [`install_thread_local()`] or
//! [`install_global()`], which writes to files or in-memory buffers rather than
//! [`stderr`].
//!
//! Let's say the user redirects [`stderr`] by running a [`TUI`] or [`readline_async`] app
//! binary called `my-tui-app`:
//! ```bash
//! my-tui-app 2>errors.log
//! ```
//!
//! Then the [redirected stream] will be empty and the `errors.log` file will not contain
//! anything. If the user expects there to be something in this file, then this looks like
//! something may have gone wrong. They aren't aware that this library will never pollute
//! [`stderr`], which is why we have [`TracingConfig`].
//!
//! The [`emit_stderr_redirection_disclaimer()`] function exists to make this explicit. It
//! writes a single informational line to [`stderr`] explaining that application logs are
//! handled internally and only unexpected panics will appear in the [redirected stream].
//! This is called automatically by [`TUI`] and [`readline_async`]. And
//!
//! [`/dev/tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//! [`DirectToAnsi`]: mod@crate::direct_to_ansi
//! [`GetConsoleMode`]: https://learn.microsoft.com/en-us/windows/console/getconsolemode
//! [`install_global()`]: crate::TracingConfig::install_global
//! [`install_thread_local()`]: crate::TracingConfig::install_thread_local
//! [`isatty`]: https://man7.org/linux/man-pages/man3/isatty.3.html
//! [`IsTerminal`]: std::io::IsTerminal
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`readline_async`]: mod@crate::readline_async
//! [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
//! [`SharedWriter`]: crate::SharedWriter
//! [`Spinner`]: crate::Spinner
//! [`stderr`]: std::io::stderr
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`tcsetattr`]: https://man7.org/linux/man-pages/man3/tcsetattr.3.html
//! [`TracingConfig`]: crate::TracingConfig
//! [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop
//! [Color detection]: crate::examine_env_vars_to_determine_color_support
//! [Console Handles]: https://learn.microsoft.com/en-us/windows/console/console-handles
//! [Raw mode]: crate::terminal_raw_mode
//! [redirected stream]:
//!     https://en.wikipedia.org/wiki/Redirection_(computing)#Redirecting_to_and_from_the_standard_file_handles
//! [Windows `cargo run` workaround]: #windows-cargo-run-workaround

// Attach.
pub mod term_api;
pub mod term_api_impl;

// Re-export.
pub use term_api::*;
pub use term_api_impl::*;

// Integration tests.
#[cfg(any(test, doc))]
pub mod term_integration_tests;

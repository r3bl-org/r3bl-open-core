// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words terminalasynctry spinnertry

//! Readline async and choose modules
//!
//! This module provides readline async functionality, choice selection UI, and spinners
//! for building interactive terminal applications.
//!
//! # Introduction
//!
//! The [`readline_async`] module lets your CLI program be asynchronous and interactive
//! without blocking the main thread. Your spawned tasks can use it to concurrently write
//! to the display output, pause and resume it. You can also display of colorful animated
//! spinners ⌛🌈 for long running tasks. With it, you can create beautiful, powerful, and
//! interactive REPLs (read execute print loops) with ease.
//!
//! 1. Because [`read_line()`] is blocking. And there is no way to terminate an OS thread
//!    that is blocking in Rust. To do this, you have to [`request_shutdown`] the process
//!    (who's thread is blocked in [`read_line()`]).
//!
//!     - There is no way to get [`read_line()`] unblocked once it is blocked.
//!     - You can use [`process::request_shutdown()`] or [`panic!()`] to kill the entire
//!       process. This is not appealing.
//!     - Even if that task is wrapped in a [`thread::spawn()` or
//!       `thread::spawn_blocking()`], it isn't possible to cancel or abort that thread,
//!       without cooperatively asking it to [`request_shutdown`]. To see what this type
//!       of code looks like, take a look at [this].
//!
//! 2. Another problem is that when a thread is blocked in [`read_line()`], and you have
//!    to display output to [`stdout`] concurrently, this poses some challenges.
//!
//!     - This is because the caret is moved by [`read_line()`] and it blocks.
//!     - When another thread / task writes to [`stdout`] concurrently, it assumes that the
//!       caret is at row `0` of a new line.
//!     - This results in output that doesn't look good since it clobbers the
//!       [`read_line()`] output, which assumes that no other output will be produced,
//!       while is blocking for user input, resulting in a bad user experience.
//!
//! Here is a video of the [`readline_async`] and [`spinner`] examples in this crate, in
//! action:
//!
//! ![`readline_async_video`](https://github.com/r3bl-org/r3bl-open-core/tree/main/docs/video/r3bl_terminal_async_clip_ffmpeg.gif?raw=true)
//!
//! # Features
//!
//! 1. Read user input from the terminal line by line, while your program concurrently
//!    writes lines to the same terminal.
//!    - One [`Readline`] instance can be used to spawn many async [`stdout`] writers,
//!      [`crate::SharedWriter`], that can write to the terminal concurrently.
//!    - For most users the [`ReadlineAsyncContext`] struct is the simplest way to use
//!      this module. You rarely have to access the underlying [`Readline`] or
//!      [`crate::SharedWriter`] directly. But you can if you need to.
//!    - [`crate::SharedWriter`] can be cloned and is thread-safe. However, there is only
//!      one instance of [`Readline`] per [`ReadlineAsyncContext`] instance.
//!
//! 2. Generate a spinner (indeterminate progress indicator). This spinner works
//!    concurrently with the rest of your program. When the [`Spinner`] is active, it
//!    automatically pauses output from all the [`crate::SharedWriter`] instances that are
//!    associated with one [`Readline`] instance. Typically a spawned task clones its own
//!    [`crate::SharedWriter`] to generate its output. This is useful when you want to
//!    show a spinner while waiting for a long-running task to complete. Please look at
//!    the example to see this in action, by running:
//!    ```bash
//!    cargo run --example readline_async
//!    ```
//!    Then type `starttask1`, press Enter. Then type `spinner`, press Enter.
//!
//! 3. Use [`tokio`] tracing with support for concurrently writing to [`stdout`]. If you
//!    choose to log to [`stdout`] then the concurrent version [`crate::SharedWriter`]
//!    from this crate will be used. This ensures that the concurrent output is supported
//!    even for your tracing logs to [`stdout`].
//!
//! 4. You can also plug in your own terminal, like [`stdout`], or [`stderr`], or any
//!    other terminal that implements [`crate::SendRawTerminal`] trait for more details.
//!
//! This module can detect when your terminal is not in interactive mode. E.g.: when you
//! pipe the output of your program to another program. In this case, the
//! [`readline_async`] feature is disabled. Both the [`ReadlineAsyncContext`] and
//! [`Spinner`] support this functionality. So if you run the examples in this crate, and
//! pipe something into them, they won't do anything.
//!
//! Here's an example:
//!
//! ```bash
//! # This will work.
//! cargo run --examples readline_async
//!
//! # This won't do anything. Just exits with no error.
//! echo "hello" | cargo run --examples readline_async
//! ```
//!
//! ## Pause and resume support
//!
//! The pause and resume functionality is implemented using:
//! - [`LineState::is_paused`] - Used to check if the line state is paused and affects
//!   rendering and input.
//! - [`LineState::set_paused`] - Use to set the paused state via the
//!   [`crate::SharedWriter`] below. This can't be called directly (outside the crate
//!   itself).
//! - [`crate::SharedWriter::line_state_control_channel_sender`] - Mechanism used to
//!   manipulate the paused state.
//!
//! The [`Readline::try_new`] or [`ReadlineAsyncContext::try_new`] create a
//! [`line_state_control_channel`] to send and receive [`crate::LineStateControlSignal`]:
//!
//! 1. The sender end of this channel is moved to the [`crate::SharedWriter`]. So any
//!    [`crate::SharedWriter`] can be used to send [`crate::LineStateControlSignal`]s to
//!    the channel, which will be processed in the task started, just for this, in
//!    [`Readline::try_new`]. This is the primary mechanism to switch between pause and
//!    resume. Some helper functions are provided in [`ReadlineAsyncContext::pause`] and
//!    [`ReadlineAsyncContext::resume`], though you can just send the signals directly to
//!    the channel's sender via the
//!    [`crate::SharedWriter::line_state_control_channel_sender`].
//! 2. The receiver end of this [`tokio::sync::mpsc::channel`] is moved to the task that
//!    is spawned by [`Readline::try_new`]. This is where the actual work is done when
//!    signals are sent via the sender (described above).
//!
//! While the [Readline] is suspended, no input is possible, and only Ctrl+C and Ctrl+D
//! are allowed to make it through, the rest of the keypresses are ignored.
//!
//! See [Readline] module docs for more implementation details on this.
//!
//! ## Input Editing Behavior
//!
//! While entering text, the user can edit and navigate through the current input line
//! with the following key bindings:
//!
//! - Works on all OSes (Linux, Windows, macOS) on most modern terminal emulators.
//! - Full Unicode Support (Including Grapheme Clusters).
//! - Multiline Editing.
//! - In-memory History.
//! - Left, Right: Move cursor left/right.
//! - Up, Down: Scroll through input history.
//! - Ctrl+W: Erase the input from the cursor to the previous whitespace.
//! - Ctrl+U: Erase the input before the cursor.
//! - Ctrl+L: Clear the screen.
//! - Ctrl+Left / Ctrl+Right: Move to previous/next whitespace.
//! - Home: Jump to the start of the line.
//!     - When the `"emacs"` feature (on by default) is enabled, Ctrl+A has the same
//!       effect.
//! - End: Jump to the end of the line.
//!     - When the `"emacs"` feature (on by default) is enabled, Ctrl+E has the same
//!       effect.
//! - Ctrl+C, Ctrl+D: Send an [`Eof`] event.
//! - Ctrl+C: Send an `Interrupt` event.
//!
//! # Examples
//!
//! See the `tui/examples` directory for comprehensive examples:
//! - `readline_async` - Async readline with concurrent output
//! - `spinner` - Animated progress indicators
//! - `shell_async` - Interactive shell implementation
//! - `choose` - Choice selection UI
//!
//! # How to use this module
//!
//! ## [`ReadlineAsyncContext::try_new()`], which is the main entry point for most use cases
//!
//! 1. To read user input, call [`ReadlineAsyncContext::read_line()`].
//! 2. You can call [`ReadlineAsyncContext::clone_shared_writer()`] to get a
//!    [`crate::SharedWriter`] instance that you can use to write to [`stdout`]
//!    concurrently, using [`std::write!`] or [`std::writeln!`].
//! 3. If you use [`std::writeln!`] then there's no need to
//!    [`ReadlineAsyncContext::flush()`] because the `\n` will flush the buffer. When
//!    there's no `\n` in the buffer, or you are using [`std::write!`] then you might need
//!    to call [`ReadlineAsyncContext::flush()`].
//! 4. You can use the [`crate::rla_println`!] and [`crate::rla_println_prefixed`!]
//!    methods to easily write concurrent output to the [`stdout`]
//!    ([`crate::SharedWriter`]).
//! 5. You can also get access to the underlying [`Readline`] via the
//!    [`ReadlineAsyncContext::readline`] field. Details on this struct are listed below.
//!    For most use cases you won't need to do this.
//!
//! ## [`Readline`] overview (please see the docs for this struct for details)
//!
//! - Structure for reading lines of input from a terminal while lines are output to the
//!   terminal concurrently. It uses dependency injection, allowing you to supply
//!   resources that can be used to:
//!   1. Read input from the user, via an async input device [`InputDevice`].
//!   2. Generate output to the raw terminal via output device [`OutputDevice`].
//!
//! - Terminal input is retrieved by calling [`Readline::readline()`], which returns each
//!   complete line of input once the user presses Enter.
//!
//! - Each [`Readline`] instance is associated with one or more [`crate::SharedWriter`]
//!   instances. Lines written to an associated [`crate::SharedWriter`] are output to the
//!   raw terminal.
//!
//! - Call [`Readline::try_new()`] to create a [`Readline`] instance and associated
//!   [`crate::SharedWriter`].
//!
//! - Call [`Readline::readline()`] (most likely in a loop) to receive a line of input
//!   from the terminal.  The user entering the line can edit their input using the key
//!   bindings listed under "Input Editing" below.
//!
//! - After receiving a line from the user, if you wish to add it to the history (so that
//!   the user can retrieve it while editing a later line), call
//!   [`Readline::add_history_entry()`].
//!
//! - Lines written to the associated [`crate::SharedWriter`] while `readline()` is in
//!   progress will be output to the screen above the input line.
//!
//! - When done, call [`crate::manage_shared_writer_output::flush_internal()`] to ensure
//!   that all lines written to the [`crate::SharedWriter`] are output.
//!
//! ## [`Spinner::try_start()`]
//!
//! This displays an indeterminate spinner while waiting for a long-running task to
//! complete. The intention with displaying this spinner is to give the user an indication
//! that the program is still running and hasn't hung up or become unresponsive. When
//! other tasks produce output concurrently, this spinner's output will not be clobbered.
//! Neither will the spinner output clobber the output from other tasks. It suspends the
//! output from all the [`crate::SharedWriter`] instances that are associated with one
//! [`Readline`] instance. Both the `readline_async.rs` and `spinner.rs` examples shows
//! this:
//! ```bash
//! cargo run --example readline_async` and `cargo run --example spinner
//! ```
//!
//! [`Spinner`]s also has cancellation support. Once a spinner is started, Ctrl+C and
//! Ctrl+D are directed to the spinner, to cancel it. Spinners can also be checked for
//! completion or cancellation by long running tasks, to ensure that they
//! [`request_shutdown`] as a response to user cancellation. Take a look at the
//! `examples/readline_async.rs` file to get an understanding of how to use this API.
//!
//! The third change is that [`ReadlineAsyncContext::try_new()`] now accepts prompts that
//! can have [`ANSI`] escape sequences in them. Here's an example of this.
//!
//! ```
//! # use r3bl_tui::readline_async::ReadlineAsyncContext;
//! # use r3bl_tui::{fg_magenta, CliTextInline, ok};
//! # pub async fn sample() -> Result<(), Box<dyn std::error::Error>> {
//!     let prompt = {
//!         let user = "naz";
//!         let prompt_seg_1 = fg_magenta("╭").bg_dark_gray().to_string();
//!         let prompt_seg_2 = fg_magenta(&format!("┤{user}├")).bg_dark_gray().to_string();
//!         let prompt_seg_3 = fg_magenta("╮").bg_dark_gray().to_string();
//!         Some(format!("{}{}{} ", prompt_seg_1, prompt_seg_2, prompt_seg_3))
//!     };
//!     let maybe_rl_ctx = ReadlineAsyncContext::try_new(prompt, None).await?;
//!     let Some(mut rl_ctx) = maybe_rl_ctx else {
//!         return Err(miette::miette!("Failed to create terminal").into());
//!     };
//!     ok!()
//! # }
//! ```
//!
//! # Video tutorials
//!
//! - [Async readline and spinner playlist]
//! - [Linux TTY programming playlist]
//!
//! # Origin
//!
//! This module is forked from [rustyline-async]. However it has mostly been rewritten and
//! re-architected. Here are some changes made to the code:
//!
//! - Rearchitect the entire module from the ground up to operate in a totally different
//!   manner than the original. All the underlying mental models are different, and
//!   simpler. The main event loop is redone. And a task is used to monitor the line
//!   channel for communication between multiple [`crate::SharedWriter`]s and the
//!   [`Readline`], to properly support pause and resume, and other control functions.
//! - Drop support for all async runtimes other than [`tokio`]. Rewrite all the code for
//!   this.
//! - Drop crates like `pin-project`, `thingbuf` in favor of [`tokio`]. Rewrite all the
//!   code for this.
//! - Drop `simplelog` and `log` dependencies. Add support for [`tracing`]. Rewrite all
//!   the code for this, and add [`tracing_setup.rs`].
//! - Remove all examples and create new ones to mimic a real world CLI application.
//! - Add `spinner_impl`, `readline_impl`, and `public_api` modules.
//! - Add tests.
//!
//! ## References for blocking and thread cancellation in Rust
//!
//! - [Docs: tokio's `stdin`]
//! - [Discussion: Stopping a thread in Rust]
//! - [Discussion: Support for `Thread::cancel()`]
//! - [Discussion: stdin, stdout redirection for spawned processes]
//!
//! ## Educational references for Linux [`TTY`] and async Rust
//!
//! - [Linux TTY and async Rust - Article on developerlife.com]
//! - [Linux TTY and async Rust - Playlist on developerlife.com YT channel]
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`Eof`]: ReadlineEvent::Eof
//! [`InputDevice`]: crate::InputDevice
//! [`line_state_control_channel`]: field@crate::SharedWriter::line_state_control_channel_sender
//! [`OutputDevice`]: crate::OutputDevice
//! [`panic!()`]: https://doc.rust-lang.org/std/panic/index.html
//! [`process::request_shutdown()`]: https://doc.rust-lang.org/std/process/fn.exit.html
//! [`read_line()`]: std::io::Stdin::read_line
//! [`readline_async`]: mod@crate::readline_async
//! [`ReadlineAsyncContext::readline`]: field@ReadlineAsyncContext::readline
//! [`request_shutdown`]: ReadlineAsyncContext::request_shutdown
//! [`spinner`]: mod@crate::readline_async::spinner
//! [`stderr`]: std::io::stderr
//! [`stdout`]: std::io::stdout
//! [`thread::spawn()` or `thread::spawn_blocking()`]: https://tokio.rs/tokio/tutorial/spawning
//! [`tokio::sync::mpsc::channel`]: tokio::sync::mpsc::channel
//! [`tokio`]: tokio
//! [`tracing_setup.rs`]: crate::TracingConfig
//! [`tracing`]: tracing
//! [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
//! [`tty`]: https://man7.org/linux/man-pages/man4/tty.4.html
//! [Async readline and spinner playlist]: https://www.youtube.com/watch?v=3vQJguti02I&list=PLofhE49PEwmwelPkhfiqdFQ9IXnmGdnSE
//! [Discussion: stdin, stdout redirection for spawned processes]: https://stackoverflow.com/questions/34611742/how-do-i-read-the-output-of-a-child-process-without-blocking-in-rust
//! [Discussion: Stopping a thread in Rust]: https://users.rust-lang.org/t/stopping-a-thread/6328/7
//! [Discussion: Support for `Thread::cancel()`]: https://internals.rust-lang.org/t/thread-cancel-support/3056/16
//! [Docs: tokio's `stdin`]: https://docs.rs/tokio/latest/tokio/io/struct.Stdin.html
//! [Linux TTY and async Rust - Article on developerlife.com]: https://developerlife.com/2024/08/20/tty-linux-async-rust/
//! [Linux TTY and async Rust - Playlist on developerlife.com YT channel]: https://www.youtube.com/watch?v=bolScvh4x7I&list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3
//! [Linux TTY programming playlist]: https://www.youtube.com/playlist?list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3
//! [rustyline-async]: https://github.com/zyansheep/rustyline-async
//! [this]: https://github.com/nazmulidris/rust-scratch/blob/fcd730c4b17ed0b09ff2c1a7ac4dd5b4a0c66e49/tcp-api-server/src/client_task.rs#L275

// XMARK: Prevent rustfmt from reformatting entire file.
#![rustfmt::skip]

// Attach sources.
// #[macro_use] propagates macros textually (order matters).
#[macro_use] pub mod readline_async_api;
#[macro_use] pub mod choose_impl;
#[macro_use] pub mod readline_async_impl;
pub mod choose_api;
pub mod spinner;
pub mod spinner_impl;

// Re-export the public API.
pub use choose_api::*;
pub use choose_impl::*;
pub use readline_async_api::*;
pub use readline_async_impl::*;
pub use spinner::*;
pub use spinner_impl::*;

// r3bl-open-core crates.
use crate::{InlineString, StdMutex};

// External crates.
use smallvec::SmallVec;
use std::sync::Arc;

// Type aliases.
pub type SafeLineState = Arc<StdMutex<LineState>>;
pub type SafeHistory = Arc<StdMutex<History>>;

pub type SafeBool = Arc<StdMutex<bool>>;
pub type SafeInlineString = Arc<StdMutex<InlineString>>;

/// This is a buffer of [`crate::DEFAULT_STRING_STORAGE_SIZE`] 80 rows x
/// [`crate::DEFAULT_PAUSE_BUFFER_SIZE`] 128 columns (chars). This buffer collects output
/// while the async terminal is paused.
pub type PauseBuffer = SmallVec<[InlineString; DEFAULT_PAUSE_BUFFER_SIZE]>;
pub const DEFAULT_PAUSE_BUFFER_SIZE: usize = 128;
pub type SafePauseBuffer = Arc<StdMutex<PauseBuffer>>;

// Constants.
pub const HISTORY_SIZE_MAX: usize = 1_000;

/// Channel buffer capacity for the readline async loop.
///
/// This enum forces callers to explicitly choose a channel capacity based on their use case,
/// making the memory/performance trade-offs visible at the call site.
///
/// # Memory Analysis
///
/// Each [`LineStateControlSignal`] message occupies approximately **64 bytes**:
/// - `InlineString`: ~32 bytes (16-byte inline storage + metadata)
/// - Enum discriminant and largest variant: ~40 bytes
/// - Tokio channel node overhead: ~24 bytes
///
/// # Capacity Reference Table
///
/// ```text
/// ┌────────────┬─────────────────┬──────────────┬─────────────────────────────┐
/// │  Variant   │  Capacity       │  Memory      │  Use Case                   │
/// ├────────────┼─────────────────┼──────────────┼─────────────────────────────┤
/// │  Minimal   │   10,000 msgs   │    0.61 MB   │  Simple CLIs, <10K outputs  │
/// │  Moderate  │   20,000 msgs   │    1.22 MB   │  Medium burst traffic       │
/// │  Large     │   50,000 msgs   │    3.05 MB   │  Large codebases (<50K)     │
/// │  VeryLarge │  100,000 msgs   │    6.10 MB   │  Very large projects        │
/// │  Extreme   │  200,000 msgs   │   12.20 MB   │  Huge monorepos             │
/// │  Overkill  │  500,000 msgs   │   30.50 MB   │  Pathological cases         │
/// └────────────┴─────────────────┴──────────────┴─────────────────────────────┘
/// ```
///
/// # Real-World Burst Scenarios
///
/// Filesystem traversal (directory tree walking) generates one message per directory:
/// - **r3bl-open-core**: ~13,666 directories → `Moderate` (20K)
/// - **Linux kernel**: ~80,000 directories → `Extreme` (200K)
/// - **Chromium**: ~150,000 directories → `Extreme` (200K)
///
/// # Why This Matters
///
/// The underlying [`SharedWriter`] uses **non-blocking `try_send()`** which fails
/// immediately when the channel is full, even if the receiver is actively processing
/// messages. If you choose too small a capacity for burst traffic scenarios, you'll get
/// "Receiver has closed" errors even though the receiver is still running.
///
/// Choose conservatively: the memory cost is negligible compared to the cost of runtime
/// failures and poor user experience.
///
/// [`LineStateControlSignal`]: crate::LineStateControlSignal
/// [`SharedWriter`]: crate::SharedWriter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelCapacity {
    /// 10,000 messages (~0.61 MB) - For simple CLIs with light output.
    Minimal,
    /// 20,000 messages (~1.22 MB) - For moderate burst traffic.
    Moderate,
    /// 50,000 messages (~3.05 MB) - For large codebases.
    Large,
    /// 100,000 messages (~6.10 MB) - For very large projects (recommended default).
    VeryLarge,
    /// 200,000 messages (~12.20 MB) - For huge monorepos.
    Extreme,
    /// 500,000 messages (~30.50 MB) - For pathological edge cases.
    Overkill,
}

impl ChannelCapacity {
    /// Returns the actual channel capacity as a [`usize`].
    #[must_use]
    pub const fn capacity(self) -> usize {
        match self {
            Self::Minimal => 10_000,
            Self::Moderate => 20_000,
            Self::Large => 50_000,
            Self::VeryLarge => 100_000,
            Self::Extreme => 200_000,
            Self::Overkill => 500_000,
        }
    }
}

impl Default for ChannelCapacity {
    /// Defaults to [`VeryLarge`] (100K messages, ~6 MB) as a safe choice for most applications.
    ///
    /// [`VeryLarge`]: ChannelCapacity::VeryLarge
    fn default() -> Self { Self::VeryLarge }
}


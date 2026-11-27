// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized TUI module coordinator and static `DEBUG_*` / `INFO_*` logging toggles for
//! `r3bl_tui`.
//!
//! When file logging is enabled, you can use `run.fish log` or `tail -f
//! /tmp/r3bl_tui/log.txt` to watch the logs.

#![rustfmt::skip]

/// Enable or disable generating log output for telemetry data. This has higher precedence
/// than [`DEBUG_TUI`]. The telemetry logs are not debug level, but info level.
pub const INFO_DISPLAY_LOG_TELEMETRY: bool = true;

/// Controls debug logging for the [`tui`] module.
///
/// [`tui`]: mod@crate::tui
pub const DEBUG_TUI: bool = true;

/// Enable or disable bounds checking debug logging. This logs bounds check conversions,
/// such as when a saturating cast to `u16` overflows or underflows.
pub const DEBUG_BOUNDS_CHECK: bool = false;

/// Enable or disable MD parser debug logging. This makes the parser very slow when
/// enabled.
pub const DEBUG_MD_PARSER: bool = false;
/// This is for running tests on the MD parser. No need to enable logging for this to
/// work.
pub const DEBUG_MD_PARSER_STDOUT: bool = false;

/// Enable or disable syntax highlighting debug logging.
pub const DEBUG_SYN_HI: bool = false;

/// Enable or disable select, copy, paste debug logging.
pub const DEBUG_CLIPBOARD: bool = false;

/// Enable or disable compositor debug logging.
pub const DEBUG_COMPOSITOR: bool = false;

/// Enable or disable debug logging for this `terminal_lib_backends` module. Related flag:
/// [`DEBUG_RENDER_PIPELINE_EXPANDED`].
pub const DEBUG_RENDER_PIPELINE: bool = false;

/// This flag has no effect unless [`DEBUG_RENDER_PIPELINE`] is set to `true`.
pub const DEBUG_RENDER_PIPELINE_EXPANDED: bool = false;

/// Controls input event debugging [`InputDevice`], and execution of render ops
/// [`queue_terminal_command!`] debugging output.
///
/// [`InputDevice`]: crate::InputDevice
/// [`queue_terminal_command!`]: crate::queue_terminal_command
pub const DEBUG_TERMINAL_BACKEND: bool = false;

/// Controls debug logging for the [`DirectToAnsi`] backend (input parsing and
/// output rendering/flush operations).
///
/// [`DirectToAnsi`]: crate::terminal_lib_backends::direct_to_ansi
pub const DEBUG_DIRECT_TO_ANSI: bool = false;

/// Controls input event debugging for the [`mio-poller`] thread.
///
/// [`mio-poller`]: crate::terminal_lib_backends::direct_to_ansi::input::mio_poller
pub const DEBUG_MIO_POLLER: bool = false;

/// Controls debug logging for the [PTY multiplexer], including adaptive rendering.
///
/// [PTY multiplexer]: crate::PTYMux
pub const DEBUG_PTY_MUX: bool = true;

/// Controls high-volume debug logging for the [`ProcessManager`].
///
/// Logs continuous data polling events, byte output yielding, and terminal resizing
/// within the active [`PTY`] sessions. Because of the extreme volume of bytes being
/// parsed from the [`PTY`] event loop, this is separated from [`DEBUG_PTY_MUX`].
///
/// [`ProcessManager`]: crate::ProcessManager
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub const DEBUG_PTY_PROCESS_MANAGER: bool = false;

/// Controls debug logging for the [`VT-100` PTY output parser].
///
/// Logs unhandled [`ANSI`] escape sequences, unsupported private modes, and parsing
/// errors directly from the [`AnsiToOfsBufPerformer`]. Generates extremely
/// high-volume output when running interactive TUIs, so defaults to `false`.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
/// [`VT-100` PTY output parser]: crate::core::ansi::vt_100_pty_output_parser
pub const DEBUG_VT100_PARSER: bool = false;

/// Controls debug logging for the [Resilient Reactor Thread].
///
/// [Resilient Reactor Thread]: crate::core::resilient_reactor_thread
pub const DEBUG_RESILIENT_REACTOR_THREAD: bool = false;

/// Controls selective warning and error logging for the [`env_source`] module.
///
/// When enabled, logs subshell process spawn failures and non-zero subshell exit
/// statuses with captured [`stderr`].
///
/// [`env_source`]: mod@crate::core::script::env_source
/// [`stderr`]: std::io::stderr
pub const DEBUG_ENV_SOURCE: bool = false;

/// Controls debug logging for the [`script`] module.
///
/// [`script`]: mod@crate::core::script
pub const DEBUG_SCRIPT: bool = true;

/// Controls debug logging for [`try_choose`].
///
/// [`try_choose`]: crate::try_choose
pub const DEBUG_CHOOSE: bool = false;

/// [`Unicode`] replacement character used when a [`grapheme`] cluster cannot be converted
/// to a single char. This character (�) is the standard fallback for
/// invalid/undisplayable characters.
///
/// [`grapheme`]: https://unicode.org/reports/tr29/
/// [`Unicode`]: https://en.wikipedia.org/wiki/Unicode
pub const UNICODE_REPLACEMENT_CHAR: char = '�';

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod rsx;
#[macro_use]
pub mod terminal_lib_backends;

// Attach.
pub mod animator;
pub mod cmd_line_args;
pub mod dialog;
pub mod editor;
pub mod global_constants;
pub mod layout;
pub mod md_parser;
pub mod syntax_highlighting;
pub mod terminal_window;

// Re-export.
pub use animator::*;
pub use cmd_line_args::*;
pub use dialog::*;
pub use editor::*;
pub use global_constants::*;
pub use layout::*;
pub use md_parser::*;
pub use syntax_highlighting::*;
pub use terminal_lib_backends::*;
pub use terminal_window::*;

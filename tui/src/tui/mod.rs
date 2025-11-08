// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Enable or disable generating log output for telemetry data. This has higher precedence
/// than [`DEBUG_TUI_MOD`]. The telemetry logs are not debug level, but info level.
pub const DISPLAY_LOG_TELEMETRY: bool = true;

/// This is the global `DEBUG` const. It is possible to create local (module scoped)
/// `DEBUG` const. However, you would have to use that symbol explicitly in the relevant
/// module, eg:
/// - `use $crate::terminal_lib_backends::DEBUG_TUI...;`
///
/// If set to `true` it enables file logging for entire module.
pub const DEBUG_TUI_MOD: bool = true;

/// Enable or disable MD parser debug logging. This makes the parser very slow when
/// enabled.
pub const DEBUG_MD_PARSER: bool = false;
/// This is for running tests on the MD parser. No need to enable logging for this to
/// work.
pub const DEBUG_MD_PARSER_STDOUT: bool = false;

/// Enable or disable syntax highlighting debug logging.
pub const DEBUG_TUI_SYN_HI: bool = false;

/// Enable or disable select, copy, paste debug logging.
pub const DEBUG_TUI_COPY_PASTE: bool = false;

/// Enable or disable compositor debug logging.
pub const DEBUG_TUI_COMPOSITOR: bool = false;

/// Enable or disable debug logging for this `terminal_lib_backends` module. Related flag:
/// [`DEBUG_TUI_SHOW_PIPELINE_EXPANDED`].
pub const DEBUG_TUI_SHOW_PIPELINE: bool = false;

/// This flag has no effect unless [`DEBUG_TUI_SHOW_PIPELINE`] is set to `true`.
pub const DEBUG_TUI_SHOW_PIPELINE_EXPANDED: bool = false;

/// Controls input event debugging [`crate::InputDeviceExt`], and execution of render ops
/// [`crate::queue_terminal_command`!] debugging output.
pub const DEBUG_TUI_SHOW_TERMINAL_BACKEND: bool = false;

/// Unicode replacement character used when a grapheme cluster cannot be converted to a
/// single char. This character (�) is the standard fallback for invalid/undisplayable
/// characters.
pub const UNICODE_REPLACEMENT_CHAR: char = '�';

// Attach sources.
pub mod animator;
pub mod cmd_line_args;
pub mod dialog;
pub mod editor;
pub mod global_constants;
pub mod layout;
pub mod list;
pub mod md_parser;
pub mod rsx;
pub mod syntax_highlighting;
pub mod terminal_lib_backends;
pub mod terminal_window;

// Re-export.
pub use animator::*;
pub use cmd_line_args::*;
pub use dialog::*;
pub use editor::*;
pub use global_constants::*;
pub use layout::*;
pub use list::*;
pub use md_parser::*;
pub use rsx::*;
pub use syntax_highlighting::*;
pub use terminal_lib_backends::*;
pub use terminal_window::*;

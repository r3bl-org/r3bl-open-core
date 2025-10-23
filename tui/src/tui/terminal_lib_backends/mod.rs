// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Background information on terminals PTY, TTY, VT100, ANSI, ASCII
//!
//! crossterm:
//! - docs: <https://docs.rs/crossterm/latest/crossterm/index.html>
//! - Raw mode: <https://docs.rs/crossterm/0.23.2/crossterm/terminal/index.html#raw-mode>
//! - Event Poll vs block: <https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#115-event-polling>
//! - Async event read eg: <https://github.com/crossterm-rs/crossterm/blob/master/examples/event-stream-tokio.rs>
//! - Async event read src: <https://github.com/crossterm-rs/crossterm/blob/master/src/event/stream.rs#L23>
//!
//! terminal:
//! - Video: <https://youtu.be/Q-te_UBzzjo?t=849>
//! - Raw mode: <https://en.wikipedia.org/wiki/POSIX_terminal_interface#Non-canonical_mode_processing>
//! - Canonical mode: <https://en.wikipedia.org/wiki/POSIX_terminal_interface#Canonical_mode_processing>
//! - Control characters & escape codes:
//!   - ANSI escape codes: <https://en.wikipedia.org/wiki/ANSI_escape_code>
//!     - Windows support: <https://en.wikipedia.org/wiki/ANSI_escape_code#DOS,_OS/2,_and_Windows>
//!     - Colors: <https://en.wikipedia.org/wiki/ANSI_escape_code#Colors>
//!   - ASCII control chars: <https://www.asciitable.com/>
//!   - VT100 Control codes: <https://vt100.net/docs/vt100-ug/chapter3.html#ED>
//! - ANSI (8-bit) vs ASCII (7-bit): <http://www.differencebetween.net/technology/web-applications/difference-between-ansi-and-ascii/>
//! - Windows Terminal (bash): <https://www.makeuseof.com/windows-terminal-vs-powershell/>
//!
//! Examples of TUI / CLI editor:
//! - reedline
//!   - repo: <https://github.com/nushell/reedline>
//!   - live stream videos: <https://www.youtube.com/playlist?list=PLP2yfE2-FXdQw0I6O4YdIX_mzBeF5TDdv>
//! - kilo
//!   - blog: <http://antirez.com/news/108>
//!   - repo (C code): <https://github.com/antirez/kilo>
//! - Sodium:
//!   - repo: <https://github.com/redox-os/sodium>

/// Terminal library backend selection for the TUI system.
///
/// R3BL TUI supports multiple terminal manipulation libraries, allowing users to choose
/// the backend that best fits their needs. Currently supported backends include:
///
/// - **Crossterm**: Cross-platform terminal library (default and recommended)
/// - **Termion**: Unix-specific terminal library with minimal overhead
///
/// # Example
///
/// ```rust
/// use r3bl_tui::TerminalLibBackend;
///
/// let backend = TerminalLibBackend::Crossterm;
/// match backend {
///     TerminalLibBackend::Crossterm => println!("Using Crossterm backend"),
///     TerminalLibBackend::Termion => println!("Using Termion backend"),
/// }
/// ```
#[derive(Debug)]
pub enum TerminalLibBackend {
    /// Crossterm backend - cross-platform terminal library supporting Windows, macOS, and
    /// Linux. This is the default and recommended backend for most applications.
    Crossterm,
    /// Termion backend - Unix-specific terminal library with minimal overhead.
    /// Only available on Unix-like systems (Linux, macOS).
    Termion,
}

/// The default terminal library backend used by R3BL TUI.
///
/// This constant defines which terminal backend is used throughout the TUI system.
/// Currently set to [`TerminalLibBackend::Crossterm`] for maximum compatibility.
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;

// Attach source files.
pub mod compositor_render_ops_to_ofs_buf;
pub mod crossterm_backend;
pub mod direct_ansi;
pub mod enhanced_keys;
pub mod input_device_ext;
pub mod input_event;
pub mod key_press;
pub mod modifier_keys_mask;
pub mod mouse_input;
pub mod offscreen_buffer;
pub mod offscreen_buffer_pool;
pub mod paint;
pub mod raw_mode;
pub mod render_op;
pub mod render_pipeline;
pub mod render_tui_styled_texts;
pub mod termion_backend;
pub mod z_order;

// Re-export.
pub use compositor_render_ops_to_ofs_buf::*;
pub use crossterm_backend::*;
pub use direct_ansi::*;
pub use enhanced_keys::*;
pub use input_device_ext::*;
pub use input_event::*;
pub use key_press::*;
pub use modifier_keys_mask::*;
pub use mouse_input::*;
pub use offscreen_buffer::*;
pub use offscreen_buffer_pool::*;
pub use paint::*;
pub use raw_mode::*;
pub use render_op::*;
pub use render_pipeline::*;
pub use render_tui_styled_texts::*;
pub use z_order::*;

// Tests.
mod test_input_event;
mod test_keypress;
mod test_mouse_input;
mod test_render_pipeline;

// Benchmarks.
#[cfg(test)]
mod pixel_char_bench;
#[cfg(test)]
mod render_op_bench;

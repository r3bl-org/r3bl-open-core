/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

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

pub enum TerminalLibBackend {
    Crossterm,
    Termion,
}

pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;

// Attach source files.
pub mod crossterm_backend;
pub mod crossterm_color_converter;
pub mod enhanced_keys;
pub mod input_device_ext;
pub mod input_event;
pub mod keypress;
pub mod modifier_keys_mask;
pub mod mouse_input;
pub mod offscreen_buffer;
pub mod offscreen_buffer_pool;
pub mod paint;
pub mod raw_mode;
pub mod render_op;
pub mod render_pipeline;
pub mod render_pipeline_to_offscreen_buffer;
pub mod render_tui_styled_texts;
pub mod termion_backend;
pub mod z_order;

// Re-export.
pub use crossterm_backend::*;
pub use crossterm_color_converter::*;
pub use enhanced_keys::*;
pub use input_device_ext::*;
pub use input_event::*;
pub use keypress::*;
pub use modifier_keys_mask::*;
pub use mouse_input::*;
pub use offscreen_buffer::*;
pub use offscreen_buffer_pool::*;
pub use paint::*;
pub use raw_mode::*;
pub use render_op::*;
pub use render_pipeline::*;
pub use render_pipeline_to_offscreen_buffer::*;
pub use render_tui_styled_texts::*;
pub use z_order::*;

// Tests.
mod test_input_event;
mod test_keypress;
mod test_mouse_input;
mod test_render_pipeline;

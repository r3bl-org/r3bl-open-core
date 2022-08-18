/*
 *   Copyright (c) 2022 R3BL LLC
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

// Attach source files.
pub mod color_converter;
pub mod async_event_stream_ext;
pub mod tw_command;
pub mod tw_input_event;
pub mod keypress;
pub mod tw_raw_mode;
pub mod mouse_input;

// Re-export.
pub use color_converter::*;
pub use async_event_stream_ext::*;
pub use tw_command::*;
pub use tw_input_event::*;
pub use keypress::*;
pub use tw_raw_mode::*;
pub use mouse_input::*;
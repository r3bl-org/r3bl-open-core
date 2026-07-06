// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides tmux-like functionality for multiplexing terminal sessions, with
//! universal compatibility for ALL programs: [`TUI`] apps, [`readline_async`] apps, and
//! command-line tools.
//!
//! # Key Features
//!
//! - **Per-process virtual terminals**: Each process maintains its own
//!   [`OfsBuf`]
//! - **Universal compatibility**: Works with bash, TUI apps, CLI tools, and more
//! - **Instant switching**: No delays or hacks needed - just display different buffers
//! - **Dynamic keyboard-driven process switching**: F1 through F9 (based on process
//!   count)
//! - **Status bar with process information**: Live status indicators for each process
//! - **[`OSC`] sequence integration**: Dynamic terminal title updates
//! - **Resource management**: Clean cleanup of [`PTY`] sessions and raw mode
//!
//! # Architecture
//!
//! The module is designed around a **per-process virtual terminal** architecture where
//! each process maintains its own complete terminal state through an [`OfsBuf`].
//! This enables true terminal multiplexing similar to tmux, but with enhanced support for
//! truecolor and TUI apps that frequently re-render their UI, with instant switching and
//! universal compatibility.
//!
//! # Key Components:
//!
//! - [`PTYMux`]: Main orchestrator that manages the event loop and coordinates components
//! - [`ProcessManager`]: Handles [`PTY`] lifecycle management and maintains per-process
//!   virtual terminals
//! - [`input_router`]: Routes keyboard input and handles dynamic shortcuts
//! - [`OutputRenderer`]: Renders the active process's buffer with status bar compositing
//!
//! ### Virtual Terminal Architecture (The "Virtual Tab" Mental Model)
//!
//! To understand how [`PTYMux`] works, it helps to understand the hierarchy:
//! 1. **The Virtual Terminal Emulator App** ([`PTYMux`]): The overarching application
//!    that manages everything, like the virtual or headless equivalent of [`Wezterm`].
//! 2. **The Virtual Tab** ([`Process`]): A completely self-contained, headless tab. Just
//!    like running `htop` in a tab in [`Wezterm`].
//! 3. **The Engine & Canvas** ([`OfsBufVT100`]): The actual headless **Virtual Terminal
//!    Emulator** living inside the tab. It parses the bytes from the OS subprocess (like
//!    `bash`) and paints them onto its own invisible 2D grid in real-time.
//!
//! Because each tab maintains its own canvas in the background, all processes run and
//! render simultaneously. When the user switches tabs, [`PTYMux`] doesn't need to ask the
//! underlying program to redraw itself. It simply tells the [`OutputRenderer`] to stop
//! copying pixels from Tab A's canvas and start copying from Tab B's canvas. The switch
//! is instant because Tab B's canvas has been kept perfectly up-to-date in the
//! background.
//!
//! # Usage Example
//!
//! ```no_run
//! use r3bl_tui::{TuiAvailability, IntoErr, core::pty_mux::PTYMux, ok};
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     let multiplexer = match PTYMux::builder()
//!         .add_process("bash", "bash", vec![])
//!         .add_process("editor", "nvim", vec![])
//!         .add_process("monitor", "htop", vec![])
//!         .build()
//!     {
//!         TuiAvailability::Available(mux) => mux,
//!         it => return it.into_err(),
//!     };
//!
//!     multiplexer.run().await?;  // F1/F2/F3 to switch, Ctrl+Q to quit
//!     ok!()
//! }
//! ```
//!
//! # Underlying protocol parser
//!
//! - [`vt_100_pty_output_parser`]: The [`ANSI`] parser module that processes escape
//!   sequences from child processes. The [`ProcessManager`] uses this via
//!   [`OfsBufVT100::apply_ansi_bytes`]
//! - [`core::ansi`]: Parent module containing all [`ANSI`]/[`VT-100`] protocol handling
//!
//! [`ANSI Parser`]: crate::AnsiToOfsBufPerformer
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`core::ansi`]: mod@crate::core::ansi
//! [`OfsBuf`]: crate::OfsBuf
//! [`OfsBufVT100::apply_ansi_bytes`]: crate::OfsBufVT100::apply_ansi_bytes
//! [`OfsBufVT100`]: crate::OfsBufVT100
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY Session`]: crate::PtySession
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
//! [`WezTerm`]: https://wezfurlong.org/wezterm/

// Attach.
mod adaptive_render_budget;
mod constants;
#[cfg(any(test, doc))]
pub mod input_router;
#[cfg(not(any(test, doc)))]
mod input_router;
mod mux;
mod output_renderer;
mod process_manager;
mod scrollback_amount;

// Public re-exports (flat API)
pub use adaptive_render_budget::*;
pub use constants::*;
pub use input_router::*;
pub use mux::*;
pub use output_renderer::*;
pub use process_manager::*;
pub use scrollback_amount::*;

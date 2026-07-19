// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides tmux-like functionality for multiplexing terminal sessions, with
//! universal compatibility for ALL programs: [`TUI`] apps, [`readline_async`] apps, and
//! command-line tools.
//!
//! # Key Features
//!
//! - **Per-process virtual terminals**: Each process maintains its own headless
//!   [`OfsBuf`] 2D canvas.
//! - **2D Viewport Panning (Up/Down & Left/Right)**: Novel 2D panning for normal-mode CLI
//!   tools (`cat`, `grep`, `head`, `tail`), allowing users to pan left/right across long
//!   lines.
//! - **`terminfo` Bypassing & SSH Robustness**: Output renderer bypasses OS `terminfo`
//!   databases, providing deterministic execution over SSH without remote `.terminfo`
//!   deployment.
//! - **Universal compatibility**: Works out-of-the-box with `bash`, TUI apps (`nvim`,
//!   `htop`), and CLI tools.
//! - **Instant switching**: Switch active terminal sessions instantly with zero render
//!   delay.
//! - **Status bar with process indicators**: Live status displays for active and
//!   background processes.
//! - **[`OSC`] sequence integration**: Dynamic terminal title and progress updates.
//! - **Resource management**: Clean cleanup of [`PTY`] sessions and raw terminal mode.
//!
//! # Architecture
//!
//! The module is designed around a **per-process virtual terminal** architecture where
//! each process maintains its own complete terminal state through an [`OfsBuf`]. This
//! enables true terminal multiplexing similar to tmux, but with enhanced support for
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
//! # Virtual Terminal Architecture
//!
//! To understand how [`PTYMux`] works, it helps to understand the hierarchy:
//! 1. **The Virtual Terminal Emulator App** ([`PTYMux`]): The overarching application
//!    that manages everything, like the virtual or headless equivalent of [`WezTerm`].
//! 2. **The Virtual Terminal** ([`Process`]): A completely self-contained, headless
//!    terminal. Just like running `htop` or `bash` in a pane in [`WezTerm`], this layer
//!    manages user-facing UX state (like vertical scrollback history).
//! 3. **The [`VT-100`] Engine & [`Canvas`]** ([`OfsBufVT100`]): The actual headless
//!    "screen" living inside the terminal. It blindly parses bytes from the OS subprocess
//!    (like `htop` or `bash`) and paints them onto its own invisible 2D grid (in memory)
//!    in real-time.
//!
//! Because each terminal maintains its own "canvas" in the background, all processes run
//! and render simultaneously. When the user switches terminals, [`PTYMux`] doesn't need
//! to ask the underlying program to redraw itself. It simply tells the [`OutputRenderer`]
//! to stop copying pixels from Terminal A's canvas and start copying from Terminal B's
//! canvas. The switch is instant because Terminal B's canvas has been kept perfectly
//! up-to-date in the background.
//!
//! # Viewport Mechanics (Scroll vs Pan)
//!
//! Because of this architecture, moving the viewport is divided into two distinct
//! operations to protect the integrity of the [`VT-100`] parser's state machine:
//!
//! 1. **Scroll (Vertical)**: Interacting with historical output.
//!    - **Triggers**: Mouse Wheel Up/Down, Keyboard PageUp/PageDown.
//!    - **Mechanics**: Vertical scrolling is explicitly handled *externally* to the
//!      canvas. The [`VT-100`] engine's internal row index must remain anchored to the
//!      bottom of the buffer so it can continue appending new output. Instead, vertical
//!      scrolling is a purely visual overlay ([`Process::maybe_scroll_offset`]) managed
//!      by the Virtual Terminal ([`Process`]) during rendering.
//!    - **Alternate Screen**: Disabled (forwarded to the process if mouse tracking is
//!      enabled).
//!
//! 2. **Pan (Horizontal)**: Shifting the viewport over the 2D grid.
//!    - **Triggers**: Native Mouse Scroll Left/Right, or `Shift` + Mouse Scroll Up/Down
//!      (currently no keyboard bindings).
//!    - **Mechanics**: Because terminals do not have "horizontal history" ribbons,
//!      panning directly mutates the internal column index of the [`VT-100`] canvas
//!      ([`OfsBufVT100`]). This is safe because shifting the horizontal viewport does not
//!      interfere with the [`VT-100`] parser's active cursor coordinates.
//!    - **Alternate Screen**: Disabled (forwarded to the process if mouse tracking is
//!      enabled).
//!
//! # Architecture & Documentation Map
//!
//! For a complete understanding of offscreen buffers, storage, and virtual terminals,
//! refer to:
//!
//! 1. [`CanvasStorage`] ([`types.rs`]): Trait Level — *The "Why"* (Architectural
//!    evolution, storage abstraction, and motivation for 2D viewport panning).
//! 2. [`GrowableBuffer`] ([`growable_buffer.rs`]): Implementation Level — *The "How It's
//!    Stored"* (The [Canvas and Viewport concept], [`VecDeque`] history storage, and 2D
//!    grid mechanics).
//! 3. [`pty_mux`] ([`mod.rs`]): UX & Parser Level — *The "How It's Triggered"* (Viewport
//!    mechanics, mouse scroll vs horizontal pan, and [`VT-100`] parser cursor anchoring).
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
//! [`ANSI Parser`]: crate::core::ansi::AnsiToOfsBufPerformer
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`Canvas`]: mod@crate::core::coordinates::canvas
//! [`CanvasStorage`]: crate::CanvasStorage
//! [`core::ansi`]: mod@crate::core::ansi
//! [`growable_buffer.rs`]: crate::tui::GrowableBuffer
//! [`GrowableBuffer`]: crate::tui::GrowableBuffer
//! [`mod.rs`]: mod@crate::pty_mux
//! [`OfsBuf`]: crate::tui::OfsBuf
//! [`OfsBufVT100::apply_ansi_bytes`]: crate::core::ansi::OfsBufVT100::apply_ansi_bytes
//! [`OfsBufVT100`]: crate::core::ansi::OfsBufVT100
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY Session`]: crate::PtySession
//! [`pty_mux`]: mod@crate::pty_mux
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`readline_async`]: crate::readline_async::ReadlineAsyncContext::try_new
//! [`TUI`]: crate::tui::TerminalWindow::main_event_loop
//! [`types.rs`]: crate::CanvasStorage
//! [`VecDeque`]: std::collections::VecDeque
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept

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

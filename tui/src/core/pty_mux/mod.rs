// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal multiplexer module for `r3bl_tui`.
//!
//! This module provides tmux-like functionality for multiplexing terminal sessions,
//! with universal compatibility for ALL programs: TUI applications, interactive shells,
//! and command-line tools.
//!
//! ## Key Features
//!
//! - **Per-process virtual terminals**: Each process maintains its own
//!   [`OffscreenBuffer`]
//! - **Universal compatibility**: Works with bash, TUI apps, CLI tools, and more
//! - **Instant switching**: No delays or hacks needed - just display different buffers
//! - **Dynamic keyboard-driven process switching**: F1 through F9 (based on process
//!   count)
//! - **Status bar with process information**: Live status indicators for each process
//! - **OSC sequence integration**: Dynamic terminal title updates
//! - **Resource management**: Clean cleanup of PTY sessions and raw mode
//!
//! ## Architecture
//!
//! The module is designed around a **per-process virtual terminal** architecture where
//! each process maintains its own complete terminal state through an [`OffscreenBuffer`].
//! This enables true terminal multiplexing similar to tmux, but with enhanced support for
//! truecolor and TUI apps that frequently re-render their UI, with instant switching and
//! universal compatibility.
//!
//! ### Key Components:
//!
//! - [`PTYMux`]: Main orchestrator that manages the event loop and coordinates components
//! - [`ProcessManager`]: Handles PTY lifecycle management and maintains per-process
//!   virtual terminals
//! - [`InputRouter`]: Routes keyboard input and handles dynamic shortcuts
//! - [`OutputRenderer`]: Renders the active process's buffer with status bar compositing
//!
//! ### Virtual Terminal Architecture:
//!
//! Each Process contains:
//! - **[`OffscreenBuffer`]**: Acts as a virtual terminal maintaining complete screen
//!   state
//! - **[`ANSI Parser`]**: Processes PTY output and updates the virtual terminal
//! - **[`PTY Session`]**: The actual process communication channel
//!
//! The multiplexer continuously polls ALL processes and updates their virtual terminals
//! independently when they produce output, but only renders the active process's buffer
//! to the actual terminal.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use r3bl_tui::core::{pty_mux::{PTYMux, Process}, get_size};
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     let terminal_size = get_size()?;
//!
//!     // Mix of different program types - all supported!
//!     let processes = vec![
//!         Process::new("bash", "bash", vec![], terminal_size),
//!         Process::new("editor", "nvim", vec![], terminal_size),
//!         Process::new("monitor", "htop", vec![], terminal_size),
//!     ];
//!
//!     let multiplexer = PTYMux::builder()
//!         .processes(processes)
//!         .build()?;
//!
//!     // F1/F2/F3 to switch processes, Ctrl+Q to quit
//!     multiplexer.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`ANSI Parser`]: crate::core::ansi::parser::AnsiToOfsBufPerformer
//! [`PTY Session`]: crate::PtyReadWriteSession

// Attach.
mod input_router;
mod mux;
mod output_renderer;
mod process_manager;

// Re-export.
pub use input_router::*;
pub use mux::*;
pub use output_renderer::*;
pub use process_manager::*;

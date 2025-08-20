// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal multiplexer module for `r3bl_tui`.
//!
//! This module provides tmux-like functionality for multiplexing terminal sessions,
//! allowing users to run multiple TUI processes in a single terminal window and switch
//! between them using keyboard shortcuts.
//!
//! ## Key Features
//!
//! - **Multiple PTY session management**: Spawn and manage multiple TUI applications
//! - **Dynamic keyboard-driven process switching**: Ctrl+1 through Ctrl+9 (based on
//!   process count)
//! - **Status bar with process information**: Live status indicators for each process
//! - **OSC sequence integration**: Dynamic terminal title updates
//! - **Fake resize technique**: Ensures proper TUI app repainting when switching
//! - **Resource management**: Clean cleanup of PTY sessions and raw mode
//!
//! ## Architecture
//!
//! The module is designed around several key components:
//!
//! - [`PTYMux`]: Main orchestrator that manages the event loop and coordinates components
//! - [`ProcessManager`]: Handles PTY lifecycle management and process switching
//! - [`InputRouter`]: Routes keyboard input and handles dynamic shortcuts
//! - [`OutputRenderer`]: Manages display rendering and status bar
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use r3bl_tui::core::pty_mux::{PTYMux, Process};
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     let processes = vec![
//!         Process::new("editor", "nvim", vec![]),
//!         Process::new("top", "btop", vec![]),
//!     ];
//!
//!     let multiplexer = PTYMux::builder()
//!         .processes(processes)
//!         .build()?;
//!
//!     multiplexer.run().await?;
//!     Ok(())
//! }
//! ```

pub mod ansi_parser;
pub mod input_router;
pub mod mux;
pub mod output_renderer;
pub mod process_manager;

pub use input_router::InputRouter;
pub use mux::{PTYMux, PTYMuxBuilder};
pub use output_renderer::OutputRenderer;
pub use process_manager::{Process, ProcessManager, ProcessOutput};

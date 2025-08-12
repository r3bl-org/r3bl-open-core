// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # PTY Module
//!
//! This module provides a high-level, async interface for spawning and controlling
//! processes in pseudo-terminals (PTYs). It supports both read-only and interactive
//! (read-write) sessions with optional OSC sequence capture for enhanced terminal
//! features.
//!
//! ## Key Features
//!
//! - **Read-only sessions**: Capture command output with optional OSC sequence processing
//! - **Interactive sessions**: Full bidirectional communication with PTY processes
//! - **OSC sequence support**: Capture progress updates and terminal escape sequences
//! - **Flexible configuration**: Control what data is captured and processed
//! - **Async/await support**: Built on tokio for non-blocking operation
//!
//! ## Main Types
//!
//! - [`PtyCommandBuilder`]: Builder for configuring and spawning PTY commands
//! - [`PtySession`]: Interactive (read-write) PTY session handle
//! - [`PtyReadOnlySession`]: Read-only PTY session handle
//! - [`PtyEvent`]: Events received from PTY processes (output, OSC sequences, exit)
//! - [`PtyInput`]: Input types that can be sent to interactive sessions
//!
//! ## Quick Start
//!
//! ### Read-only session (capture command output):
//! ```rust
//! use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut session = PtyCommandBuilder::new("ls")
//!     .args(["-la"])
//!     .spawn_read_only(PtyConfigOption::Output)?;
//!
//! while let Some(event) = session.event_receiver_half.recv().await {
//!     match event {
//!         PtyEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Interactive session (send input to process):
//! ```rust
//! use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent, PtyInput, ControlChar};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut session = PtyCommandBuilder::new("cat")
//!     .spawn_read_write(PtyConfigOption::Output)?;
//!
//! // Send input
//! session.input_sender_half.send(PtyInput::WriteLine("Hello, PTY!".into()))?;
//! session.input_sender_half.send(PtyInput::SendControl(ControlChar::CtrlD))?; // EOF
//!
//! // Read output
//! while let Some(event) = session.event_receiver_half.recv().await {
//!     match event {
//!         PtyEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

// Attach.
pub mod common_impl;
pub mod osc_seq;
pub mod pty_config;
pub mod pty_core;
pub mod pty_read_only;
pub mod pty_read_write;

// Re-export.
pub use osc_seq::*;
pub use pty_config::*;
pub use pty_core::*;
// Internal implementations - not exported.
pub(crate) use pty_read_only::spawn_pty_read_only_impl;
pub(crate) use pty_read_write::spawn_pty_read_write_impl;

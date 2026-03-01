// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words hndlr

//! # [`PTY`] Module
//!
//! This module provides a high-level, async interface for spawning and controlling
//! processes in [pseudo-terminals] ([`PTY`]s). It supports both read-only and read-write
//! (read-write) sessions with optional [`OSC`] sequence capture for enhanced terminal
//! features.
//!
//! ## [`PTY`] Architecture Overview
//!
//! The [`PTY`] (pseudo-terminal) acts as a bridge between your program and spawned
//! processes:
//!
//! ### Read-Only Mode
//!
//! ```text
//! ┌──────────────┐ ◄── events ◄── ┌───────────────────────────────┐
//! │ Your Program │                │ Spawned Task (1) in Read Only │
//! │              │                │         session               │
//! │              │                │            ↓                  │
//! │ Handle       │                │ ◄─── PTY creates pair ───►    │
//! │ events and   │                │ ┊Controller┊     ┊Controlled┊ │
//! │ process      │                │     ↓                 ↓       │
//! │ completion   │                │ Spawn Tokio       Controlled  │
//! │ from read    │                │ blocking task     spawns      │
//! │ only session │                │ (2) to read       child       │
//! │              │                │ from              process (3) │
//! │              │                │ Controller and                │
//! │              │                │ generate events               │
//! │              │                │ for your program              │
//! └──────────────┘                └───────────────────────────────┘
//! ```
//!
//! ### Read-Write Mode
//!
//! ```text
//! ┌──────────────┐   ┌────────────┐   ┌───────────────────┐
//! │ Your Program │◄─►│    PTY     │   │ Spawned Process   │
//! │ Reads/writes │   │ Controller │   │ stdin/stdout/     │
//! │ through      │   │     ↕      │   │ stderr redirected │
//! │ controller   │   │    PTY     │   │ to controlled     │
//! │ side         │   │     ↕      │   │ side              │
//! │              │   │ Controlled │◄─►│                   │
//! └──────────────┘   └────────────┘   └───────────────────┘
//! ```
//!
//! ## Key Features
//!
//! - **Read-only sessions**: Capture command output with optional [`OSC`] sequence
//!   processing
//! - **Read-write sessions**: Full bidirectional communication with [`PTY`] processes
//! - **[`OSC`] sequence support**: Capture progress updates and terminal escape sequences
//! - **Flexible configuration**: Control what data is captured and processed
//! - **Async/await support**: Built on [`tokio`] for non-blocking operation
//!
//! ## Implementation Strategy
//!
//! Both read-only and read-write modes use a multi-task async architecture:
//!
//! - **Completion Task**: Manages [`PTY`] pair creation, child process lifecycle, and
//!   coordinates shutdown
//! - **Reader Task**: Handles output from the spawned process using `spawn_blocking`
//!   ([`PTY`] I/O is inherently synchronous)
//! - **Input Handler Task**: (Read-write only) Manages input to the spawned process
//! - **Bridge Task**: (Read-write only) Converts async input events to sync channel for
//!   blocking I/O
//!
//! ### Task Coordination & Lifecycle
//!
//! | Time | Completion Task        | Reader Task    | Input Handler     | Bridge Task    |
//! | :--- | :--------------------- | :------------- | :---------------- | :------------- |
//! | 0    | 🛫 Spawn child         |                |                   |                |
//! | 1    | 🛫 Spawn reader        | 🛫 Start read  |                   |                |
//! | 2    | 🛫 Spawn input hndlr▪  | 📖 Read data   | 🛫 Start▪         |                |
//! | 3    | 🛫 Spawn bridge▪       | 📤 Send events | 📥 Wait input▪    | 🛫 Start▪      |
//! | 4    | 🛬 Wait `child.wait()` | 📖 Read data   | ✍️ Write [`PTY`]▪ | 🔄 Bridge I/O▪ |
//! | 5    | 📤 Send Exit event     | 📖 Read EOF    | 📥 Wait input▪    | 🔄 Bridge I/O▪ |
//! | 6    | 💀 drop(controlled)    | 🛬 Exit        | 🛬 Exit▪          | 🛬 Exit▪       |
//! | 7    | 🛬 Wait all tasks      |                |                   |                |
//! | 8    | ✅ Return status       |                |                   |                |
//!
//! Legends:
//! ```txt
//! Task lifecycle:    🛫 start · 🛬 wait/exit · 💀 drop · ✅ done
//! IO operations:     📖 read · ✍️ write
//! Send/receive pair: 📤 send · 📥 receive
//! Others:            ▪ Read-write mode only · 🔄 bridge
//! ```
//!
//! ### Event Communication
//!
//! ```text
//! Your Program ◄─── MPSC Channel ◄─── Background Tasks
//!              │                      │
//!              │                      ├─ Reader Task → Output/OSC Events
//!              │                      └─ Completion Task → Exit Events
//!              │
//!              └─ MPSC Channel ──► Input Handler (read-write only)
//! ```
//!
//! Events flow through unbounded [MPSC channels], allowing your program to receive output
//! asynchronously while background tasks handle the blocking [`PTY`] operations.
//!
//! ### Channel Architecture (Read-Write Mode)
//!
//! ```text
//! Your Program
//!      │
//!      │ (async input)
//!      │
//! ┌────▼────────────────────────────────┐
//! │    Async Input Channel              │
//! │    (unbounded MPSC)                 │
//! └─────────────────────────────────────┘
//!      │
//!      │ (Bridge Task converts async→sync)
//!      │
//! ┌────▼────────────────────────────────┐
//! │    Sync Input Channel               │
//! │    (std::sync::mpsc)                │
//! └─────────────────────────────────────┘
//!      │
//!      │ (Input Handler Task - blocking)
//!      │
//! ┌────▼────────────────────────────────┐
//! │    PTY Controller                   │
//! │    (write to spawned)               │
//! └─────────────────────────────────────┘
//!      │
//! ┌────▼────────────────────────────────┐
//! │    Spawned Process Input            │
//! └─────────────────────────────────────┘
//!
//! ──── Spawned process boundary ────────
//!
//! ┌─────────────────────────────────────┐
//! │    Spawned Process Output           │
//! └─────────────────────────────────────┘
//!      │
//!      │
//! ┌────▼────────────────────────────────┐
//! │    PTY Controller                   │
//! │    (read from spawned)              │
//! └─────────────────────────────────────┘
//!      │
//!      │ (Reader Task - blocking)
//!      │
//! ┌────▼────────────────────────────────┐
//! │     Async Output Channel            │
//! │     (unbounded MPSC)                │
//! └─────────────────────────────────────┘
//!      │
//!      │ (async output)
//!      │
//!      ▼
//! Your Program
//! ```
//!
//! ## Critical [`PTY`] Lifecycle Management
//!
//! The controlled side of the [`PTY`] must be closed in the parent process immediately
//! after spawning the child. Without this, reads from the controller will never see
//! [`EOF`] -- even after the child exits -- causing permanent deadlocks.
//!
//! See [`PtyPair`] for the full explanation, including file descriptor ownership,
//! the `Drop` chain, and why [`PtyPair::spawn_command_and_close_controlled`] exists.
//!
//!
//! ## Main Types
//!
//! - [`PtyCommandBuilder`]: Builder for configuring and spawning [`PTY`] commands
//! - [`PtyReadWriteSession`]: Read-write [`PTY`] session handle
//! - [`PtyReadOnlySession`]: Read-only [`PTY`] session handle
//! - [`PtyReadWriteOutputEvent`]: Events received from [`PTY`] processes (output, [`OSC`]
//!   sequences, exit)
//! - [`PtyInputEvent`]: Input types that can be sent to interactive sessions
//!
//! ## Quick Start
//!
//! ### Read-only session (capture command output):
//! ```rust
//! use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyReadOnlyOutputEvent};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut session = PtyCommandBuilder::new("ls")
//!     .args(["-la"])
//!     .spawn_read_only(PtyConfigOption::Output)?;
//!
//! while let Some(event) = session.output_evt_ch_rx_half.recv().await {
//!     match event {
//!         PtyReadOnlyOutputEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyReadOnlyOutputEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Interactive session (send input to process):
//! ```no_run
//! # #[cfg(not(unix))]
//! # fn main() {}
//! # #[cfg(unix)]
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use r3bl_tui::{PtyCommandBuilder, PtyReadWriteOutputEvent, PtyInputEvent, ControlSequence, CursorKeyMode, size, width, height};
//!
//! let mut session = PtyCommandBuilder::new("cat")
//!     .spawn_read_write(size(width(80) + height(24)))?;
//!
//! // Send input
//! session.input_event_ch_tx_half.send(PtyInputEvent::WriteLine("Hello, PTY!".into()))?;
//! session.input_event_ch_tx_half.send(PtyInputEvent::SendControl(ControlSequence::CtrlD, CursorKeyMode::default()))?; // EOF
//!
//! // Read output
//! while let Some(event) = session.output_event_receiver_half.recv().await {
//!     match event {
//!         PtyReadWriteOutputEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyReadWriteOutputEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`tokio`]: tokio
//! [MPSC channels]: tokio::sync::mpsc
//! [pseudo-terminals]: https://en.wikipedia.org/wiki/Pseudoterminal

// Attach.
pub mod pty_command_builder;
pub mod pty_config;
pub mod pty_core;
pub mod pty_read_only;
pub mod pty_read_write;

// Re-export.
pub use pty_command_builder::*;
pub use pty_config::*;
pub use pty_core::*;

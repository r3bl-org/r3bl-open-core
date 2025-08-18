// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # PTY Module
//!
//! This module provides a high-level, async interface for spawning and controlling
//! processes in pseudo-terminals (PTYs). It supports both read-only and read-write
//! (read-write) sessions with optional OSC sequence capture for enhanced terminal
//! features.
//!
//! ## PTY Architecture Overview
//!
//! The PTY (pseudo-terminal) acts as a bridge between your program and spawned processes:
//!
//! ### Read-Only Mode
//!
//! ```text
//! ┌──────────────┐ ◄── events ◄── ┌───────────────────────────────┐
//! │ Your Program │                │ Spawned Task (1) in Read Only │
//! │              │                │         session               │
//! │              │                │            ↓                  │
//! │ Handle       │                │ ◄─── PTY creates pair ───►    │
//! │ events and   │                │ ┊Master/   ┊     ┊Slave/    ┊ │
//! │ process      │                │ ┊Controller┊     ┊Controlled┊ │
//! │ completion   │                │     ↓                 ↓       │
//! │ from read    │                │ Spawn Tokio       Controlled  │
//! │ only session │                │ blocking task     spawns      │
//! │              │                │ (2) to read       child       │
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
//! ┌────────────┐   ┌────────────┐   ┌─────────────────┐
//! │Your Program│◄─►│    PTY     │   │Spawned Process  │
//! │            │   │Controller/ │   │                 │
//! │Reads/writes│   │  Master    │   │stdin/stdout/    │
//! │through     │   │     ↕      │   │stderr redirected│
//! │controller/ │   │    PTY     │   │to slave/        │
//! │master side │   │     ↕      │   │controlled side  │
//! │            │   │ Slave/     │◄─►│                 │
//! │            │   │Controlled  │   │                 │
//! └────────────┘   └────────────┘   └─────────────────┘
//! ```
//!
//! ## Key Features
//!
//! - **Read-only sessions**: Capture command output with optional OSC sequence processing
//! - **Read-write sessions**: Full bidirectional communication with PTY processes
//! - **OSC sequence support**: Capture progress updates and terminal escape sequences
//! - **Flexible configuration**: Control what data is captured and processed
//! - **Async/await support**: Built on tokio for non-blocking operation
//!
//! ## Implementation Strategy
//!
//! Both read-only and read-write modes use a multi-task async architecture:
//!
//! - **Completion Task**: Manages PTY pair creation, child process lifecycle, and
//!   coordinates shutdown
//! - **Reader Task**: Handles output from the spawned process using `spawn_blocking` (PTY
//!   I/O is inherently synchronous)
//! - **Input Handler Task**: (Read-write only) Manages input to the spawned process
//! - **Bridge Task**: (Read-write only) Converts async input events to sync channel for
//!   blocking I/O
//!
//! ### Task Coordination & Lifecycle
//!
//! ```text
//! Time │ Completion Task      │ Reader Task    │ Input Handler │ Bridge Task
//! ─────┼──────────────────────┼────────────────┼───────────────┼─────────────
//!   0  │ 🛫 Spawn child       │                │               │
//!   1  │ 🛫 Spawn reader      │ 🛫 Start read  │               │
//!   2  │ 🛫 Spawn input hdlr* │ 📖 Read data   │ 🛫 Start*     │
//!   3  │ 🛫 Spawn bridge*     │ 📤 Send events │ 📥 Wait input*│ 🛫 Start*
//!   4  │ 🛬 Wait child.wait() │ 📖 Read data   │ ✍️  Write PTY* │ 🔄 Bridge I/O*
//!   5  │ 📤 Send Exit event   │ 📖 Read EOF    │ 📥 Wait input*│ 🔄 Bridge I/O*
//!   6  │ 💀 drop(controlled)  │ 🛬 Exit        │ 🛬 Exit*      │ 🛬 Exit*
//!   7  │ 🛬 Wait all tasks    │                │               │
//!   8  │ ✅ Return status     │                │               │
//! ```
//! *Read-write mode only
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
//! Events flow through unbounded MPSC channels, allowing your program to receive output
//! asynchronously while background tasks handle the blocking PTY operations.
//!
//! ### Channel Architecture (Read-Write Mode)
//!
//! ```text
//! Your Program
//!      │ (async input)
//!      ▼
//! ┌─────────────────────────────────────┐
//! │        Async Input Channel          │
//! │  (unbounded MPSC)                   │
//! └─────────────────────────────────────┘
//!      │
//!      ▼ (Bridge Task converts async→sync)
//! ┌─────────────────────────────────────┐
//! │        Sync Input Channel           │
//! │  (std::sync::mpsc)                  │
//! └─────────────────────────────────────┘
//!      │
//!      ▼ (Input Handler Task - blocking)
//! ┌─────────────────────────────────────┐
//! │         PTY Controller              │
//! │       (write to spawned)            │
//! └─────────────────────────────────────┘
//!      │
//!      ▼
//! ┌─────────────────────────────────────┐
//! │      Spawned Process Input          │
//! └─────────────────────────────────────┘
//!
//!
//! ┌─────────────────────────────────────┐
//! │      Spawned Process Output         │
//! └─────────────────────────────────────┘
//!      │
//!      ▼
//! ┌─────────────────────────────────────┐
//! │         PTY Controller              │
//! │       (read from spawned)           │
//! └─────────────────────────────────────┘
//!      │
//!      ▼ (Reader Task - blocking)
//! ┌─────────────────────────────────────┐
//! │       Async Output Channel          │
//! │     (unbounded MPSC)                │
//! └─────────────────────────────────────┘
//!      │
//!      ▼ (async output)
//! Your Program
//! ```
//!
//! ## Critical PTY Lifecycle Management
//!
//! **Understanding PTY file descriptor management is crucial for all implementations
//! in this module to avoid deadlocks.**
//!
//! ### The PTY File Descriptor Reference Counting Problem
//!
//! A PTY consists of two halves: master (controller) and slave (controlled). The
//! kernel's PTY implementation requires **BOTH** conditions for EOF:
//!
//! 1. The slave side must be closed (happens when the child process exits)
//! 2. The reader must be the ONLY remaining reference to the master
//!
//! ### Why Explicit Resource Management is Required
//!
//! Even though the child process has exited and closed its slave FD, our `controlled`
//! variable keeps the slave side open. The PTY won't send EOF to the master until ALL
//! slave file descriptors are closed. Without explicitly dropping `controlled`, it would
//! remain open until the entire function returns, causing the reader to block forever
//! waiting for EOF that never comes.
//!
//! ### The Solution Strategy (Applied in All Implementations)
//!
//! 1. Clone a reader from controller, keeping controller in scope
//! 2. **Explicitly drop controlled after process exits** - closes our controlled half FD
//! 3. Drop controller after process exits to release master FD
//! 4. This allows the reader to receive EOF and exit cleanly
//!
//! **This pattern is critical in both read-only and read-write implementations.**
//!
//! ## Main Types
//!
//! - [`PtyCommandBuilder`]: Builder for configuring and spawning PTY commands
//! - [`PtyReadWriteSession`]: Read-write PTY session handle
//! - [`PtyReadOnlySession`]: Read-only PTY session handle
//! - [`PtyOutputEvent`]: Events received from PTY processes (output, OSC sequences, exit)
//! - [`PtyInputEvent`]: Input types that can be sent to interactive sessions
//!
//! ## Quick Start
//!
//! ### Read-only session (capture command output):
//! ```rust
//! use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyOutputEvent};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut session = PtyCommandBuilder::new("ls")
//!     .args(["-la"])
//!     .spawn_read_only(PtyConfigOption::Output)?;
//!
//! while let Some(event) = session.output_event_ch_rx_half.recv().await {
//!     match event {
//!         PtyOutputEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyOutputEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Interactive session (send input to process):
//! ```rust
//! use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyOutputEvent, PtyInputEvent, ControlChar};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut session = PtyCommandBuilder::new("cat")
//!     .spawn_read_write(PtyConfigOption::Output)?;
//!
//! // Send input
//! session.input_event_sender_half.send(PtyInputEvent::WriteLine("Hello, PTY!".into()))?;
//! session.input_event_sender_half.send(PtyInputEvent::SendControl(ControlChar::CtrlD))?; // EOF
//!
//! // Read output
//! while let Some(event) = session.output_event_receiver_half.recv().await {
//!     match event {
//!         PtyOutputEvent::Output(data) => {
//!             print!("{}", String::from_utf8_lossy(&data));
//!         }
//!         PtyOutputEvent::Exit(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

// Attach.
pub mod pty_command_builder;
pub mod pty_common_io;
pub mod pty_config;
pub mod pty_core;
pub mod pty_read_only;
pub mod pty_read_write;

// Re-export.
pub use pty_command_builder::*;
pub use pty_config::*;
pub use pty_core::*;

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Session Layer
//!
//! The **Session Layer** is the middle tier of the [`PTY`] stack. It orchestrates the
//! async lifecycle of a [`PTY`] process, bridging the gap between low-level OS [`PTY`]
//! I/O (Engine Layer) and your application code.
//!
//! ## Key Responsibilities
//!
//! - **Async Orchestration**: Manages the [Background Tasks] (Reader, Writer,
//!   Orchestrator).
//! - **Event Routing**: Converts raw [`PTY`] output into structured [`PtyOutputEvent`]s
//!   and routes input via [`PtyInputEvent`]s.
//! - **Resource Cleanup**: Ensures all background tasks are joined and resources are
//!   freed when the process exits.
//!
//! ## Lifecycle Diagram
//!
//! The typical lifecycle of a session follows this flow:
//!
//! ```text
//! ┌──────────────────────┐      ┌─────────────┐      ┌─────────────────────────┐
//! │  PtySessionBuilder   │ ───► │   Spawn     │ ───► │      tokio::select!     │
//! │  (Configuration)     │      │  (Startup)  │      │  (Active Interaction)   │
//! └──────────────────────┘      └─────────────┘      └────────────┬────────────┘
//!                                                                 │
//!                                           ┌─────────────────────┴───────────────────┐
//!                                           │                                         │
//!                                 📥 Receive Output Events                  📤 Send Input Events
//!                                (PtyOutputEvent::Output)                  (PtyInputEvent::Write)
//! ```
//!
//! ## Standard Usage Pattern
//!
//! Most applications interact with a session using a [`tokio::select!`] loop. See [Core
//! Async Concepts] for details on why [`tokio::task::JoinHandle`] doesn't require pinning
//! for use in [`select!`] branches.
//!
//! ```rust,ignore
//! use r3bl_tui::{PtySessionBuilder, PtyOutputEvent, PtyInputEvent};
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     let mut session = PtySessionBuilder::new("bash")
//!         .start()?;
//!
//!     loop {
//!         tokio::select! {
//!             // 1. Handle output from the PTY
//!             Some(event) = session.rx_output_event.recv() => {
//!                 match event {
//!                     PtyOutputEvent::Output(bytes) => { /* render bytes */ }
//!                     PtyOutputEvent::Exit(status) => { break; }
//!                     _ => {}
//!                 }
//!             }
//!             // 2. Await process orchestration and completion
//!             status = &mut session.orchestrator_task_handle => {
//!                 break;
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`select!`]: tokio::select
//! [Background Tasks]: crate::core::pty#background-tasks-the-task-trio
//! [Core Async Concepts]: crate::main_event_loop_impl#core-async-concepts-pin-and-unpin

pub mod pty_input_event;
pub mod pty_output_event;
pub mod pty_session_builder;
pub mod pty_session_types;
pub mod tasks;

pub use pty_input_event::*;
pub use pty_output_event::*;
pub use pty_session_builder::*;
pub use pty_session_types::*;

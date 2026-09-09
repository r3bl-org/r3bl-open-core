// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Session Layer
//!
//! The **Session Layer** is the middle tier of the [3-layer Functional Stack]. It
//! orchestrates the lifecycle of a [`PTY`] process, bridging the gap between low-level OS
//! [`PTY`] I/O ([Engine Layer]) and your application code ([Application Layer]).
//!
//! ## Synchronous Core and Async Adapter
//!
//! 1. **Synchronous Mode ([`PtySessionBuilder::start()`])**: Returns a purely synchronous
//!    [`PtySession`] using standard library channels and OS threads. No [`tokio`] runtime
//!    is needed.
//! 2. **Asynchronous Adapter ([`PtySessionBuilder::start_async()`])**: Returns an
//!    [`AsyncPtySession`] backed by [`tokio`] channels and bridge tasks for applications
//!    driving interactions via [`tokio::select!`].
//!
//! ## Key Responsibilities
//!
//! - **Thread Orchestration**: Manages the [Thread Trio] (Reader, Writer, Orchestrator).
//! - **Event Routing**: Converts raw [`PTY`] output into structured [`PtyOutputEvent`]s
//!   and routes input via [`PtyInputEvent`]s.
//! - **Resource Cleanup**: Ensures all background threads are joined and resources are
//!   freed when the process exits.
//!
//! ## Synchronous Lifecycle (Core)
//!
//! Synchronous applications configure the builder and call
//! [`PtySessionBuilder::start()`]. Events are drained using the standard library
//! receiver, and the session is joined on exit:
//!
//! ```text
//! ┌───────────────────┐     ┌───────────────┐     ┌──────────────────────┐
//! │ PtySessionBuilder │ ──► │     start     │ ──► │  rx_output_event.    │
//! │ (Configuration)   │     │   (Startup)   │     │       recv()         │
//! └───────────────────┘     └───────────────┘     └───────┬──────────────┘
//!                                                         │
//!                                     ┌───────────────────┴───────────────────┐
//!                                     │                                       │
//!                           📥 Receive Output Events                📤 Send Input Events
//!                          (PtyOutputEvent::Output)                (session.send_input())
//! ```
//!
//! See [`PtySession`] and [`PtySessionBuilder::start()`] for complete code examples.
//!
//! ## Asynchronous Lifecycle (Adapter)
//!
//! Most asynchronous applications interact with a session using a [`tokio::select!`]
//! loop. See [Core Async Concepts] for details on why [`tokio::task::JoinHandle`] does
//! not require pinning for use in [`select!`] branches.
//!
//! ```text
//! ┌───────────────────┐     ┌───────────────┐     ┌──────────────────────┐
//! │ PtySessionBuilder │ ──► │  start_async  │ ──► │    tokio::select!    │
//! │ (Configuration)   │     │   (Startup)   │     │ (Active Interaction) │
//! └───────────────────┘     └───────────────┘     └───────┬──────────────┘
//!                                                         │
//!                                     ┌───────────────────┴───────────────────┐
//!                                     │                                       │
//!                           📥 Receive Output Events                📤 Send Input Events
//!                          (PtyOutputEvent::Output)                (session.tx_input_event)
//! ```
//!
//! See [`AsyncPtySession`] and [`PtySessionBuilder::start_async()`] for complete code
//! examples.
//!
//! [3-layer Functional Stack]: crate::core::pty#the-functional-stack
//! [`AsyncPtySession`]: crate::AsyncPtySession
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtyInputEvent`]: crate::PtyInputEvent
//! [`PtyOutputEvent`]: crate::PtyOutputEvent
//! [`PtySession`]: crate::PtySession
//! [`PtySessionBuilder::start()`]: crate::PtySessionBuilder::start
//! [`PtySessionBuilder::start_async()`]: crate::PtySessionBuilder::start_async
//! [`PtySessionBuilder`]: crate::PtySessionBuilder
//! [`select!`]: tokio::select
//! [`tokio::select!`]: tokio::select
//! [`tokio::task::JoinHandle`]: tokio::task::JoinHandle
//! [`tokio`]: tokio
//! [Application Layer]: crate::core::pty#the-functional-stack
//! [Core Async Concepts]: crate::main_event_loop_impl#core-async-concepts-pin-and-unpin
//! [Engine Layer]: crate::core::pty::pty_engine
//! [Thread Trio]: crate::core::pty#the-thread-trio

#![rustfmt::skip]

// Attach.

mod builder;
mod config;
#[cfg(any(test, doc))]
pub mod events;
#[cfg(not(any(test, doc)))]
mod events;
mod session;
pub mod threads;
mod type_aliases;

// Re-export.

pub use builder::*;
pub use config::*;
pub use events::*;
pub use session::*;
pub use threads::*;
pub use type_aliases::*;

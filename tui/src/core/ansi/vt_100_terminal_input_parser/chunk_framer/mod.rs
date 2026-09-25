// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateful chunk framing and stream safety layer.
//!
//! This module implements Layer 2 (Stateful Chunk Framing & Safety) of the
//! [`Sans-IO Parser Architecture`][sans_io_arch]. It provides the stateful streaming
//! bridge between raw operating system [`read()`] syscall chunks (Layer 1) and the
//! stateless sequence syntax parser ([`try_parse_input_event()`], Layer 3).
//!
//! # Primary Entry Points
//!
//! This framing layer coordinates between two specialized submodules:
//!
//! 1. **[`accumulator`]**: Stateful sequence framing and event buffering.
//!    - **[`ChunkFramer::process_incoming_bytes()`]**: Primary ingestion entry point.
//!      Ingests raw OS read chunks, passes them through the circuit breaker, invokes the
//!      chunk parser, accumulates incomplete trailing fragments across syscall
//!      boundaries, and queues extracted [`VT100InputEventIR`] events.
//!    - **[`ChunkFramer`] ([`Iterator`])**: Draining entry point. Yields extracted
//!      [`VT100InputEventIR`] events one-by-one via [`Iterator::next()`].
//!    - **[`UnparsedBufferAction::determine_action()`]**: Inspection entry point.
//!      Evaluates unparsed accumulator bytes when the decoder returns `None` to determine
//!      whether to keep and await more bytes, purge malformed sequences (preventing
//!      accumulator poisoning), or trip the circuit breaker on runaway [`OSC`] streams.
//!
//! 2. **[`circuit_breaker`]**: Zero-allocation streaming defense against runaway
//!    sequences.
//!    - **[`OscCircuitBreaker::try_drain()`]**: Filter entry point. Intercepts incoming
//!      chunks while the breaker is open and swallows runaway [`OSC`] payload bytes on
//!      the fly without heap allocation until a valid terminator or abort condition
//!      occurs.
//!    - **[`OscCircuitBreaker::trip()`]**: State transition entry point. Trips the
//!      circuit breaker into the open state when unparsed bytes exceed
//!      [`MAX_OSC_SEQUENCE_LENGTH`].
//!    - **[`OscCircuitBreaker::Open`]**: State variant representing the open circuit
//!      condition where incoming chunk bytes are actively drained.
//!
//! # Architecture: Chunk Framing & Circuit-Breaker Pipeline
//!
//! In raw terminal mode, user keystrokes, mouse events, and bidirectional terminal
//! emulator responses arrive as an unframed, fragmented byte stream over standard input.
//!
//! For the overarching protocol model covering bidirectional terminal queries, dynamic
//! color reports, and why unhandled sequences (such as [`OSC`] 52 clipboard reads) are
//! absorbed and discarded, see
//! [`Bidirectional Communication` in the parent module][bidirectional_comm].
//!
//! ## Stream Safety & Circuit-Breaker Strategy
//!
//! While normal terminal responses (< 1 MiB) are cleanly framed across chunk boundaries
//! and parsed into intermediate events, runaway or malformed sequences pose severe
//! threats to application stability:
//!
//! - **Normal Inbound Sequences (< 1 MiB)**: Accumulate across chunk boundaries until a
//!   complete sequence is recognized by [`try_parse_input_event()`]. Supported events are
//!   queued into the event buffer, and unhandled sequences are absorbed as
//!   [`VT100InputEventIR::Ignored`].
//! - **Runaway / Oversized Sequences (> 1 MiB)**: If an incoming [`OSC`] sequence exceeds
//!   [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB) without encountering a terminator, the
//!   **Circuit-Breaker trips**:
//!   1. The accumulated 1 MiB is immediately cleared to reclaim memory.
//!   2. The circuit breaker transitions into [`OscCircuitBreaker::Open`].
//!   3. Subsequent incoming chunks are swallowed on-the-fly with **zero heap
//!      allocations** and **zero emitted events** until a terminator ([`ANSI_BEL`] or
//!      [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]), syntax abort, or the 16 MiB
//!      [`MAX_OSC_DRAIN_BYTES`] safety ceiling is encountered.
//! - **Structural Buffer Purging**: If unparsed bytes fail to parse and exceed safe
//!   thresholds without matching any valid prefix, [`UnparsedBufferAction`] purges
//!   malformed garbage to prevent accumulator poisoning.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────────┐
//! │                   Inbound Chunk Framing & Circuit-Breaker Pipeline               │
//! ├──────────────────────────────────────────────────────────────────────────────────┤
//! │                                                                                  │
//! │  Terminal Emulator / Subprocess                       ChunkFramer                │
//! │  ┌───────────────────┐                             ┌──────────────────────────┐  │
//! │  │ Inbound Responses │ ── stdin: "\x1b]52;..." ──► │ process_incoming_bytes() │  │
//! │  └───────────────────┘                             └────────────┬─────────────┘  │
//! │                                                                 │                │
//! │                                               ┌─────────────────┘                │
//! │                                               ▼                                  │
//! │                                     Is Circuit Breaker Open?                     │
//! │                                               │                                  │
//! │                       ┌───────────────────────┴──────────────────────┐           │
//! │                       ▼ YES (Draining)                               ▼ NO        │
//! │              ┌───────────────────┐                         ┌──────────────────┐  │
//! │              │    try_drain()    │                         │                  │  │
//! │              │  (Zero-allocation │                         │                  │  │
//! │              │   byte swallow)   │                         │                  │  │
//! │              └────────┬──────────┘                         │                  │  │
//! │                       │                                    │   accumulator    │  │
//! │                       │ Terminator (BEL/ST)?               │  .extend(slice)  │  │
//! │                       ├─► NO:  Await Next Chunk            │                  │  │
//! │                       │                                    │                  │  │
//! │                       └─► YES: Reset to Closed             │                  │  │
//! │                                │                           │                  │  │
//! │                                └─► (chunk remainder) ─────►│                  │  │
//! │                                                            └────────┬─────────┘  │
//! │                                                                     │            │
//! │                                                                     ▼            │
//! │                                                           try_parse_input_event  │
//! │                                                  ┌──────────────────┴─┐          │
//! │                                                  ▼ Some               ▼ None     │
//! │                                         ┌────────────────┐     ┌─────────────┐   │
//! │                                         │  Push Event &  │     │  classify_  │   │
//! │                                         │ Drain Consumed │     │  unparsed_  │   │
//! │                                         └───────┬────────┘     │   buffer    │   │
//! │                                                 │              └──────┬──────┘   │
//! │                                                 ▼                     │          │
//! │                                        (Loop while !empty)            │          │
//! │                              ┌──────────────────┬─────────────────────┤          │
//! │                              ▼ Incomplete       ▼ Malformed           ▼ Runaway  │
//! │                            ┌─────────────┐   ┌─────────────┐   ┌───────────────┐ │
//! │                            │    Wait     │   │  Discard &  │   │ TRIP BREAKER  │ │
//! │                            │ (Next Read) │   │  Clear Buf  │   │ 1. Open Drain │ │
//! │                            │             │   │ (Malformed  │   │ 2. Clear 1MiB │ │
//! │                            │             │   │   & > 64B)  │   │               │ │
//! │                            └─────────────┘   └─────────────┘   └───────────────┘ │
//! └──────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Primary Types
//!
//! - [`ChunkFramer`]: Stateful accumulator converting raw [`stdin`] bytes into
//!   [`VT100InputEventIR`].
//! - [`UnparsedBufferAction`]: Evaluates unparsed residual bytes when decoding returns
//!   `None` (`KeepAndAwaitMore`, `PurgeMalformed`, `TripCircuitBreaker`).
//! - [`OscCircuitBreaker`]: Streaming circuit-breaker state machine for swallowing
//!   runaway [`OSC`] payloads.
//!
//! [`accumulator`]: mod@accumulator
//! [`ANSI_BEL`]: crate::ANSI_BEL
//! [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
//! [`ChunkFramer::process_incoming_bytes()`]: ChunkFramer::process_incoming_bytes
//! [`ChunkFramer`]: ChunkFramer
//! [`circuit_breaker`]: mod@circuit_breaker
//! [`Iterator::next()`]: Iterator::next
//! [`Iterator`]: std::iter::Iterator
//! [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
//! [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscCircuitBreaker::Open`]: circuit_breaker::OscCircuitBreaker::Open
//! [`OscCircuitBreaker::trip()`]: circuit_breaker::OscCircuitBreaker::trip
//! [`OscCircuitBreaker::try_drain()`]: circuit_breaker::OscCircuitBreaker::try_drain
//! [`OscCircuitBreaker`]: circuit_breaker::OscCircuitBreaker
//! [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`stdin`]: std::io::stdin
//! [`try_parse_input_event()`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
//! [`UnparsedBufferAction::determine_action()`]: accumulator::UnparsedBufferAction::determine_action
//! [`UnparsedBufferAction`]: accumulator::UnparsedBufferAction
//! [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`VT100InputEventIR`]: super::VT100InputEventIR
//! [bidirectional_comm]: mod@crate::core::ansi::vt_100_terminal_input_parser#bidirectional-communication-user-input-vs-terminal-responses
//! [sans_io_arch]: mod@crate::core::ansi::vt_100_terminal_input_parser#parser-architecture-sans-io

// Attach source files.
// Private.
mod chunk_framer_struct;

// Private in production, public for docs/tests (enables rustdoc links to submodules).
#[cfg(any(test, doc))]
pub mod circuit_breaker;
#[cfg(not(any(test, doc)))]
mod circuit_breaker;

#[cfg(any(test, doc))]
pub mod accumulator;
#[cfg(not(any(test, doc)))]
mod accumulator;

// Tests.
#[cfg(any(test, doc))]
pub mod unit_tests;

// Public re-exports (barrel export pattern).
pub use accumulator::*;
pub use chunk_framer_struct::*;
pub use circuit_breaker::*;

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Streaming byte-to-IR parser for terminal input bytes.
//!
//! This module provides the stateful streaming bridge between raw operating system
//! [`read()`] syscall buffers and the stateless pure sequence parser
//! ([`try_parse_input_event()`]).
//!
//! # Architecture: Stateful Stream Processing & Circuit-Breaker
//!
//! In raw mode, user keystrokes, mouse events, and bidirectional terminal emulator
//! responses arrive as an unframed, fragmented byte stream over standard input.
//!
//! ## 1. Outbound Copying via [`OSC`] 52
//!
//! When an application copies text in headless or remote SSH environments (where local
//! display servers like Wayland or macOS Cocoa are unavailable), it emits an in-band
//! [`OSC`] 52 escape sequence directly to standard output via [`ClipboardService`]
//! (orchestrated by [`copy_to_clipboard()`]; see [`SystemClipboard`] and
//! [`Osc52Clipboard`]):
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                      Outbound Copy Workflow                           │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │   Application Layer                     Terminal Emulator / Host      │
//! │  ┌──────────────────┐                  ┌─────────────────────────┐    │
//! │  │ Editor Selection │                  │ Client System Clipboard │    │
//! │  └────────┬─────────┘                  └───────────▲─────────────┘    │
//! │           │ copy_to_clipboard()                    │                  │
//! │           ▼                                        │ In-band capture  │
//! │  ┌──────────────────┐                              │ (decodes Base64) │
//! │  │ SystemClipboard  │                              │                  │
//! │  └────────┬─────────┘                              │                  │
//! │           │ (Remote SSH / headless $DISPLAY unset) │                  │
//! │           ▼                                        │                  │
//! │  ┌──────────────────┐                              │                  │
//! │  │  Osc52Clipboard  │ ─── stdout:                  │                  │
//! │  └──────────────────┘    "\x1b]52;c;<base64>\x07" ─┘                  │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 2. Inbound Terminal Responses & the Circuit-Breaker
//!
//! Inbound terminal query responses (such as theme background color queries [`OSC`] 11,
//! or [`OSC`] 52 clipboard responses) arrive back over [`stdin`].
//!
//! - **Normal Inbound Sequences (< 1 MiB)**: The sequence is cleanly framed and absorbed
//!   as [`VT100InputEventIR::Ignored`]. All inbound [`OSC`] responses (including color
//!   queries like [`OSC`] 11 and clipboard queries like [`OSC`] 52) are unconditionally
//!   discarded without processing, preventing raw parameters or Base64 payload bytes from
//!   leaking into the application event stream as spurious physical keystrokes.
//! - **Runaway / Oversized Sequences (> 1 MiB)**: If an incoming [`OSC`] sequence exceeds
//!   [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB) without encountering a terminator, the
//!   **Circuit-Breaker trips**:
//!   1. The accumulated 1 MiB is immediately cleared to reclaim memory.
//!   2. The circuit breaker transitions into [`OscCircuitBreaker::Open`].
//!   3. Subsequent incoming chunks are swallowed on-the-fly with **zero heap
//!      allocations** and **zero emitted events** until a terminator ([`ANSI_BEL`] or
//!      [`ANSI_ST_7BIT`]), syntax abort, or the 16 MiB [`MAX_OSC_DRAIN_BYTES`] safety
//!      ceiling is encountered.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────────┐
//! │                   Inbound Query Reply & Circuit-Breaker Pipeline                 │
//! ├──────────────────────────────────────────────────────────────────────────────────┤
//! │                                                                                  │
//! │  Terminal Emulator / Subprocess                     InputByteStreamToIrParser    │
//! │  ┌───────────────────┐                             ┌──────────────────────────┐  │
//! │  │ Inbound Responses │ ── stdin: "\x1b]52;..." ──► │  advance(chunk, maybe)   │  │
//! │  └───────────────────┘                             └────────────┬─────────────┘  │
//! │                                                                 │                │
//! │                                               ┌─────────────────┘                │
//! │                                               ▼                                  │
//! │                                     Is Circuit Breaker Open?                     │
//! │                                               │                                  │
//! │                       ┌───────────────────────┴──────────────────────┐           │
//! │                       ▼ YES (Draining)                               ▼ NO        │
//! │              ┌───────────────────┐                         ┌──────────────────┐  │
//! │              │   drain_chunk()   │                         │                  │  │
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
//! │                            │             │   │ (Shift+Home │   │ 2. Clear 1MiB │ │
//! │                            │             │   │   & > 64B)  │   │               │ │
//! │                            └─────────────┘   └─────────────┘   └───────────────┘ │
//! └──────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Primary Types
//!
//! - [`InputByteStreamToIrParser`]: Stateful accumulator converting raw [`stdin`] bytes
//!   into [`VT100InputEventIR`].
//! - [`UnparsedBufferClassification`]: Single-pass classification of unparsed accumulator
//!   bytes (`Incomplete`, `MalformedSequence`, `RunawayOsc`).
//! - [`OscCircuitBreaker`]: Streaming circuit-breaker state machine for swallowing
//!   runaway [`OSC`] payloads.
//!
//! [`ANSI_BEL`]: crate::ANSI_BEL
//! [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
//! [`ClipboardService`]: crate::ClipboardService
//! [`copy_to_clipboard()`]: crate::copy_to_clipboard
//! [`InputByteStreamToIrParser`]: core::InputByteStreamToIrParser
//! [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
//! [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
//! [`Osc52Clipboard`]: crate::Osc52Clipboard
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscCircuitBreaker::Open`]: osc_circuit_breaker::OscCircuitBreaker::Open
//! [`OscCircuitBreaker`]: osc_circuit_breaker::OscCircuitBreaker
//! [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`SSH`]: https://en.wikipedia.org/wiki/Secure_Shell
//! [`stdin`]: std::io::stdin
//! [`SystemClipboard`]: crate::SystemClipboard
//! [`try_parse_input_event()`]:
//!     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
//! [`UnparsedBufferClassification`]: core::UnparsedBufferClassification
//! [`VT100InputEventIR::Ignored`]:
//!     crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`VT100InputEventIR`]: super::VT100InputEventIR

// Private in production, public for docs/tests (enables rustdoc links to submodules).
#[cfg(any(test, doc))]
pub mod core;
#[cfg(not(any(test, doc)))]
mod core;

#[cfg(any(test, doc))]
pub mod osc_circuit_breaker;
#[cfg(not(any(test, doc)))]
mod osc_circuit_breaker;

// Public re-exports (barrel export pattern).
pub use core::*;
pub use osc_circuit_breaker::*;

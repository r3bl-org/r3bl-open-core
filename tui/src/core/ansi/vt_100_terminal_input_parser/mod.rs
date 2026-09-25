// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`VT-100`] Terminal Input Parsing Layer
//!
//! This module provides pure, reusable [`ANSI`] sequence parsing for terminal input. It
//! converts raw bytes (escape sequences, [`UTF-8`] text) into high-level input events,
//! handling both human keystrokes and terminal emulator responses.
//!
//! # Parser Architecture and Mental Model
//!
//! Reading bytes from [`stdin`] is not straightforward. The operating system delivers
//! incoming bytes in arbitrary I/O chunks (slices read from [`stdin`] via [`read()`]
//! syscalls). The bytes don't come in fully framed as a valid escape sequence in each
//! chunk. A valid "frame" may come in over multiple OS [`read()`] calls. A valid escape
//! sequence or keystroke frame may arrive fragmented across multiple read chunks, or a
//! single chunk may contain multiple concatenated sequences.
//!
//! This parsing module has 2 jobs:
//!
//! 1. _Framing_: Stitch fragmented read chunks back together across multiple [`read()`]
//!    syscall invocations into complete, valid sequence frames (and purge or drain
//!    invalid, malformed, or runaway bytes to prevent stream lockup). To preserve
//!    incomplete sequence fragments across read boundaries, framing logic must be
//!    stateful ([`chunk_framer`]).
//!
//! 2. _Decoding_: Once a sequence is fully framed, a stateless sequence decoder runs on
//!    the complete byte slice to decode it into intermediate representation (IR)
//!    [events][`VT100InputEventIR`]. This is stateless.
//!
//! ## Flow of Execution
//!
//! The flow of execution begins in the dedicated [`mio`] polling thread (managed by
//! [`MioPollWorker`] / [`RRT`]), which reads raw byte chunks from non-blocking [`stdin`]
//! in a loop and forwards them to [`ChunkFramer::process_incoming_bytes()`].
//!
//! Framing occurs inside [`ChunkFramer`]: it appends each raw chunk to an internal
//! accumulator buffer and repeatedly delegates to the stateless syntax parser
//! ([`try_parse_input_event()`]). When the parser recognizes an event, its
//! `bytes_consumed` delineates the frame boundary, enqueuing the decoded event and
//! draining those bytes from the accumulator. Any incomplete sequence fragments remain
//! stored across OS [`read()`] calls until subsequent bytes complete the frame (or they
//! get purged if structurally malformed).
//!
//! ```text
//! OS stdin fd
//!     │
//!     ▼  (1) Non-blocking read() in a loop
//! consume_stdin_input_with_sender()   [in dedicated thread via MioPollWorker / RRT]
//!     │
//!     ▼  (2) Delivers raw chunk (&[u8])
//! ChunkFramer::process_incoming_bytes()
//!     │
//!     ├─► (3) Appends chunk to internal accumulator (Vec<u8>)
//!     │
//!     └─► (4) Framing & decoding loop:
//!             Calls try_parse_input_event(&accumulator)
//!             ├─► Found: bytes_consumed defines frame boundary
//!             │          - Enqueue event into internal_events
//!             │          - Drain bytes_consumed from accumulator
//!             │          - Repeat loop for next sequence in buffer
//!             └─► None: Incomplete sequence fragment remains in accumulator
//!                        waiting for the next OS read() chunk
//! ```
//!
//! ## Primary Public Entry Point
//!
//! [`try_parse_input_event()`] is the primary stateless function for decoding a single
//! contiguous byte slice (`&[u8]`) into strongly-typed [`VT100InputEventIR`] events.
//!
//! # Bidirectional Communication: User Input vs. Terminal Responses
//!
//! Modern terminal emulators ([`Ghostty`], [`Alacritty`], [`Kitty`], [`WezTerm`],
//! [`iTerm2`], VS Code Terminal, etc.) act as bidirectional communication peers rather
//! than simple one-way keystroke sources.
//!
//! While [`stdin`] is conventionally associated with human keystrokes, terminal emulators
//! synthesize and write control sequences directly into [`stdin`] in response to queries
//! sent by the application.
//!
//! A modern TUI app often needs information about the environment. For example:
//! - "Is the user running a dark theme or a light theme?"
//! - "What is the exact hex RGB of the terminal's default background?"
//! - "What is currently in the system clipboard?" (critical over SSH where X11/Wayland
//!   are not available).
//!
//! Because there is no OS syscall like `get_terminal_background_color()`, the TUI app
//! asks the terminal emulator directly via [`OSC`] [Operating System Command][osc_1]
//! sequences, written to [`stdout`]:
//!
//! ```text
//! ┌──────────────┐                                        ┌──────────────┐
//! │   TUI App    │ ─── stdout: "\x1b]11;?\x07" ─────────► │   Terminal   │
//! │              │     Outbound OSC 11 query:             │   Emulator   │
//! │              │     "What is your background color?"   └──────┬───────┘
//! │              │                                               │
//! │              │ ◄── stdin:  "\x1b]11;rgb:1e1e/1e1e/1e1e\x07" ─┘
//! └──────────────┘     (Inbound OSC 11 response written to STDIN!)
//! ```
//!
//! [`OSC`] sequences:
//! - _Begin with_ `\x1b]` (`ESC ]`), acting as the [Operating System Command][osc_1]
//!   introducer.
//! - _End with_ `\x07` ([`BEL`]) or `ESC \` (String Terminator [`ST`]), which terminates
//!   the sequence payload.
//!
//! Common terminal-generated [`stdin`] sequences are handled as follows:
//!
//! **Supported sequences (decoded into input events)**:
//! - **Dynamic Color Reports & Theme Detection ([`OSC`] 10, 11, 12, 13, 14, 17, 19)**:
//!   - _Purpose_: Query and detect terminal colors:
//!     - [`OSC`] 10: Text foreground color.
//!     - [`OSC`] 11: Text background color (for dark/light theme adaptation).
//!     - [`OSC`] 12: Text cursor color.
//!     - [`OSC`] 13 & 14: Mouse pointer foreground and background colors.
//!     - [`OSC`] 17 & 19: Highlight (selection) background and foreground colors.
//!   - _Handling_: Parsed into [`VT100InputEventIR::ColorReport`] containing
//!     [`TerminalColorReport`].
//!   - _Outbound ([`stdout`])_: `ESC ] <code> ; ? BEL` (e.g., `ESC ] 10 ; ? BEL`,
//!     `ESC ] 11 ; ? BEL`), constructed via [`OscSequence::ColorQuery`] or emitted via
//!     [`OscSender::send_color_query`].
//!   - _Inbound ([`stdin`])_:
//!     - `ESC ] <code> ; rgb:rrrr/gggg/bbbb BEL` (or `ESC \` ST terminator)
//! - **Device Status / Cursor Position ([`CSI`] 6n)**:
//!   - _Purpose_: Terminal window size synchronization and sanity checks.
//!   - _Outbound ([`stdout`])_: `ESC [ 6 n`
//!   - _Inbound ([`stdin`])_: `ESC [ <row> ; <col> R`
//! - **Window Focus Events**:
//!   - _Purpose_: Pause/resume rendering when terminal window focus changes.
//!   - _Handling_: Parsed into [`VT100InputEventIR::Focus`] with [`VT100FocusStateIR`].
//!   - _Outbound ([`stdout`])_: Enabled via `ESC [ ? 1004 h`
//!   - _Inbound ([`stdin`])_: `ESC [ I` (Focus In) / `ESC [ O` (Focus Out)
//! - **Bracketed Paste Mode**:
//!   - _Purpose_: Prevent pasted shell code from executing unintentionally.
//!   - _Handling_: Parsed into [`VT100InputEventIR::Paste`] with [`VT100PasteModeIR`].
//!   - _Outbound ([`stdout`])_: Enabled via `ESC [ ? 2004 h`
//!   - _Inbound ([`stdin`])_: `ESC [ 200 ~` (Paste Start) / `ESC [ 201 ~` (Paste End)
//!
//! **Framed and discarded sequences**:
//! - **System Clipboard Read ([`OSC`] 52)**:
//!   - _Purpose_: Remote SSH clipboard access when `$DISPLAY` is unset.
//!   - _Handling_: Inbound clipboard replies (`ESC ] 52 ; c ; <base64-data> BEL`) are
//!     absorbed and mapped to [`VT100InputEventIR::Ignored`].
//!   - _Note_: Outbound clipboard copy to [`stdout`] remains fully supported via
//!     [`Osc52Clipboard`].
//! - **Primary Device Attributes ([`CSI`] c)**:
//!   - _Purpose_: Terminal emulator identification and feature probing during startup.
//!   - _Outbound ([`stdout`])_: `ESC [ c`
//!   - _Inbound ([`stdin`])_: `ESC [ ? 1 ; 2 c`
//! - **Other Unhandled [`OSC`] Commands**:
//!   - _Handling_: Absorbed safely as [`VT100InputEventIR::Ignored`] to prevent keystroke
//!     leakage.
//!
//! ## The Critical Challenge: Inbound Response Leakage
//!
//! Because terminal query responses arrive on the same [`stdin`] stream as user
//! keystrokes:
//!
//! 1. If an application requests a background color via [`OSC`] 11 (`ESC ] 11 ; ? BEL`),
//!    the terminal responds with [`OSC`] 11 (`ESC ] 11 ; rgb:1e1e/1e1e/1e1e BEL`).
//! 2. If the input parser does not recognize and parse or absorb this sequence, the
//!    unparsed bytes (`1`, `1`, `;`, `r`, `g`, `b`, `:`, `1`, `e`...) will fall through
//!    to the [`UTF-8`] text parser, turn into [`VT100InputEventIR`] events, and be
//! 3. When a permissive or legacy terminal emulator is configured to permit clipboard
//!    read queries (such as [`xterm`] with `allowPasteSelection`, [`rxvt-unicode`],
//!    `foot`, or `xterm.js`), an [`OSC`] 52 response can dump massive base64 payloads
//!    (such as multi-megabyte log files or database dumps) directly into [`stdin`].
//!    Additionally, piped binary inputs (`app < binary_file`) or buggy emulators can
//!    send an `ESC ]` sequence without ever sending a closing terminator. While modern
//!    terminal emulators ([`Ghostty`], [`Kitty`], [`Alacritty`]) disable clipboard reads
//!    by default for security, without strict length limits, an unbounded buffer would
//!    cause runaway memory consumption and freeze the application event loop.
//!
//! Our parser architecture cleanly handles this by framing and filtering inbound [`OSC`]
//! responses:
//!
//! - **Normal inbound responses (< 1 MiB)**: Framed across chunk boundaries. Supported
//!   color queries ([`OSC`] 10, 11, 12, 13, 14, 17, 19) are parsed into
//!   [`VT100InputEventIR::ColorReport`]. Unhandled sequences (such as [`OSC`] 52
//!   clipboard reads or unknown commands) are mapped to [`VT100InputEventIR::Ignored`]
//!   and discarded so they never leak as keystrokes.
//! - **Runaway inbound responses (> 1 MiB)**: Quarantined on-the-fly by
//!   [`OscCircuitBreaker`] inside [`chunk_framer`], draining oversized payloads
//!   with **zero heap allocations** and **zero emitted events**.
//!
//! ### Why are [`OSC`] 52 read queries discarded?
//!
//! You might wonder why we parse [`OSC`] 10, 11, 12, 13, 14, 17, 19 while we discard
//! [`OSC`] 52 clipboard read queries.
//!
//! 1. **[`OSC`] 52 inbound clipboard reads are discarded for security**: While the
//!    [`OSC`] 52 specification supports clipboard read queries (`ESC ] 52 ; c ; ? BEL`),
//!    modern terminal emulators ([`Ghostty`], [`Kitty`], [`Alacritty`], [`WezTerm`],
//!    etc.) disable reading by default for security to prevent clipboard snooping
//!    vulnerabilities. Interactive pasting is handled safely via Bracketed Paste mode
//!    instead. Outbound copying remains supported via [`Osc52Clipboard`].
//!
//!    When an application copies text in headless or remote SSH environments (where local
//!    display servers like Wayland or macOS Cocoa are unavailable), it emits an in-band
//!    [`OSC`] 52 escape sequence directly to standard output via [`ClipboardService`]
//!    (orchestrated by [`copy_to_clipboard()`]; see [`SystemClipboard`] and
//!    [`Osc52Clipboard`]):
//!
//!    ```text
//!    ┌───────────────────────────────────────────────────────────────────────┐
//!    │                      Outbound Copy Workflow                           │
//!    ├───────────────────────────────────────────────────────────────────────┤
//!    │                                                                       │
//!    │   Application Layer                     Terminal Emulator / Host      │
//!    │  ┌──────────────────┐                  ┌─────────────────────────┐    │
//!    │  │ Editor Selection │                  │ Client System Clipboard │    │
//!    │  └────────┬─────────┘                  └───────────▲─────────────┘    │
//!    │           │ copy_to_clipboard()                    │                  │
//!    │           ▼                                        │ In-band capture  │
//!    │  ┌──────────────────┐                              │ (decodes Base64) │
//!    │  │ SystemClipboard  │                              │                  │
//!    │  └────────┬─────────┘                              │                  │
//!    │           │ (Remote SSH / headless $DISPLAY unset) │                  │
//!    │           ▼                                        │                  │
//!    │  ┌──────────────────┐                              │                  │
//!    │  │  Osc52Clipboard  │ ─── stdout:                  │                  │
//!    │  └──────────────────┘    "\x1b]52;c;<base64>\x07" ─┘                  │
//!    └───────────────────────────────────────────────────────────────────────┘
//!    ```
//!
//!    Because outbound copying is strictly one-way from the application to the terminal
//!    emulator, it requires no inbound reply over [`stdin`]. Discarding inbound [`OSC`]
//!    52 replies protects the application from clipboard snooping attacks without
//!    impacting outbound copy functionality.
//!
//! 2. **Dynamic colors enable adaptive theming and styling**: Modern terminals support
//!    querying their active color palette. Originating queries are emitted via
//!    [`OscSequence::ColorQuery`] or [`OscSender::send_color_query`]. Parsing foreground
//!    ([`OSC`] 10), background ([`OSC`] 11), cursor ([`OSC`] 12), mouse pointer
//!    ([`OSC`] 13, 14), and selection highlight ([`OSC`] 17, 19) reports gives TUI
//!    applications the metadata needed to dynamically adapt syntax highlighting,
//!    contrast, cursor visibility, and widget styles when the user switches their OS or
//!    terminal between dark and light themes or custom color schemes (emitted as
//!    [`InputEvent::TerminalColor`]).
//!
//! 3. **Preventing spurious keystroke leaks**: Unhandled [`OSC`] sequences (such as
//!    window title queries or unsupported control codes) are machine-level metadata.
//!    Absorbing them prevents unparsed characters from polluting the application input
//!    stream as spurious keystrokes.
//!
//! # Parser Architecture (Sans-IO)
//!
//! The parser is designed around **Sans-IO principles**: it operates on raw byte slices
//! and maintains zero OS I/O dependencies. This separation of concerns creates a clean,
//! testable architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  1. IO & Polling Layer (Platform-Specific)                              │
//! │  • Dedicated mio thread reading non-blocking STDIN                      │
//! │  • Computes MaybeMore availability heuristic from kernel buffer state   │
//! └───────────────────────────────────┬─────────────────────────────────────┘
//!                                     │
//!                                     │ Raw &[u8] bytes + MaybeMore
//!                                     ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  2. Stateful Chunk Framing & Safety (chunk_framer)                      │
//! │  • Buffers raw chunks across OS read boundaries                         │
//! │  • Evaluates stream heuristics (MaybeMore) for 0ms ESC latency          │
//! │  • Prevents accumulator poisoning via structural classification         │
//! │  • Quarantines runaway OSC sequences on-the-fly (OscCircuitBreaker)     │
//! └───────────────────────────────────┬─────────────────────────────────────┘
//!                                     │
//!                                     │ Complete sequence slices (&[u8])
//!                                     ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  3. Stateless Sequence Parser & Router (chunk_decoder)                  │
//! │  • Zero-allocation pure slice pattern-matching                          │
//! │  • Prioritized pipeline: CSI -> SS3 -> OSC/Alt+] -> Alt -> Ctrl -> UTF8 │
//! └───────────────────────────────────┬─────────────────────────────────────┘
//!                                     │
//!                                     │ Strongly-typed AST (VT100InputEventIR)
//!                                     ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  4. Protocol Conversion & Public API                                    │
//! │  • Converts intermediate IR events into application InputEvent          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Responsibilities
//!
//! Click through to the submodules for full implementation details and documentation:
//!
//! ## [`try_parse_input_event()`]
//! - Single source of truth for the public API surface.
//! - Pure stateless function decoding contiguous byte slices into IR events.
//!
//! ## [`chunk_framer`]
//! - Stateful stream framing, buffering, and safety engine.
//! - [`ChunkFramer`]: Accumulates unparsed chunks across [`read()`] calls.
//! - [`OscCircuitBreaker`]: Quarantines and drains runaway [`OSC`] payloads with zero
//!   heap allocations.
//! - [`UnparsedBufferAction`]: Evaluates unparsed residual sequences to prevent
//!   accumulator poisoning.
//!
//! ## [`chunk_decoder`]
//! - Stateless sequence parser and prioritized sequence dispatcher
//!   ([`try_parse_input_event()`]).
//! - Specialized decoders:
//!   - `keyboard`: Decodes [Kitty Keyboard Protocol] (`CSI u`), legacy [`VT-100`],
//!     [`SS3`] function keys, Alt+key combinations, and control characters.
//!   - `mouse`: Decodes [`SGR`] and legacy mouse tracking sequences.
//!   - `terminal_events`: Decodes window focus events, bracketed paste delimiters, and
//!     window resize responses.
//!   - `utf8`: Decodes multi-byte [`UTF-8`] printable text.
//!   - `csi_scanner` / `osc_scanner`: Zero-copy lexical scanners for [`CSI`] and [`OSC`]
//!     syntax.
//!
//! ## [`ir_event_types`]
//! - Strongly-typed intermediate representation (IR) AST definitions
//!   ([`VT100InputEventIR`]).
//!
//! ## [`maybe_more`]
//! - Stream availability heuristic ([`MaybeMore`]) for zero-latency [`ESC`]
//!   disambiguation.
//!
//! # Establishing Ground Truth Through Validation Testing
//!
//! The [`observe_terminal`] validation test is a critical tool for validating parser
//! accuracy against real terminal emulators.
//!
//! Run it with:
//! ```bash
//! cargo test observe_terminal -- --ignored --nocapture
//! ```
//!
//! # Testing Strategy
//!
//! To prevent regressions and ensure cross-terminal compatibility, the parser uses a
//! three-tiered testing strategy:
//!
//! 1. **Validation Tests** ([`validation_tests`]): Acceptance tests that establish ground
//!    truth against real terminal emulators.
//! 2. **Round-Trip Unit Tests** ([`generator_round_trip_tests`]): Component tests that
//!    ensure symmetry between generator output and parser input.
//! 3. **[`PTY`] Integration Tests** ([`vt_100_parser_integration_tests`]): System tests
//!    that run in real [`PTY`] processes to verify end-to-end behavior across platforms.
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`BEL`]: crate::ANSI_BEL
//! [`chunk_decoder`]: mod@chunk_decoder
//! [`chunk_framer`]: mod@chunk_framer
//! [`ChunkFramer`]: ChunkFramer
//! [`ClipboardService`]: crate::ClipboardService
//! [`consume_stdin_input_with_sender()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::consume_stdin_input_with_sender
//! [`copy_to_clipboard()`]: crate::copy_to_clipboard
//! [`CSI`]: crate::CsiSequence
//! [`ESC`]: crate::EscSequence
//! [`generator_round_trip_tests`]: mod@unit_tests::generator_round_trip_tests
//! [`Ghostty`]: https://ghostty.org/
//! [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
//! [`ir_event_types`]: mod@ir_event_types
//! [`iTerm2`]: https://iterm2.com/
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`maybe_more`]: mod@maybe_more
//! [`MaybeMore`]: MaybeMore
//! [`mio`]: mio
//! [`MioPollWorker`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
//! [`observe_terminal`]: crate::vt_100_terminal_input_parser::validation_tests::observe_real_interactive_terminal_input_events::observe_terminal
//! [`Osc52Clipboard`]: crate::Osc52Clipboard
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscCircuitBreaker`]: crate::vt_100_terminal_input_parser::chunk_framer::OscCircuitBreaker
//! [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
//! [`OscSequence::ColorQuery`]: crate::core::ansi::osc::OscSequence::ColorQuery
//! [`process_incoming_bytes()`]: ChunkFramer::process_incoming_bytes
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
//! [`RRT`]: crate::RRT
//! [`rxvt-unicode`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`SGR`]: crate::SgrCode
//! [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
//! [`SSH`]: https://en.wikipedia.org/wiki/Secure_Shell
//! [`ST`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`SystemClipboard`]: crate::SystemClipboard
//! [`TerminalColorReport`]: crate::TerminalColorReport
//! [`try_parse_input_event()`]: crate::vt_100_terminal_input_parser::try_parse_input_event
//! [`UnparsedBufferAction`]: crate::vt_100_terminal_input_parser::chunk_framer::UnparsedBufferAction
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`validation_tests`]: mod@validation_tests
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT100FocusStateIR`]: crate::vt_100_terminal_input_parser::VT100FocusStateIR
//! [`VT100InputEventIR::ColorReport`]: crate::vt_100_terminal_input_parser::VT100InputEventIR::ColorReport
//! [`VT100InputEventIR::Focus`]: crate::vt_100_terminal_input_parser::VT100InputEventIR::Focus
//! [`VT100InputEventIR::Ignored`]: crate::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`VT100InputEventIR::Paste`]: crate::vt_100_terminal_input_parser::VT100InputEventIR::Paste
//! [`VT100InputEventIR`]: crate::vt_100_terminal_input_parser::VT100InputEventIR
//! [`VT100PasteModeIR`]: crate::vt_100_terminal_input_parser::VT100PasteModeIR
//! [`vt_100_parser_integration_tests`]: mod@vt_100_parser_integration_tests
//! [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
//! [osc_1]: https://en.wikipedia.org/wiki/ANSI_escape_code#Operating_System_Commands

// Skip rustfmt for rest of file.
#![rustfmt::skip]

// Public API re-export from private module.
mod input_parser_entry_point;
pub use input_parser_entry_point::*;

// Framing incoming bytes from stdin.
#[cfg(any(test, doc))]
pub mod chunk_framer;
#[cfg(not(any(test, doc)))]
pub(crate) mod chunk_framer;
pub use chunk_framer::*;

// Decoding framed sequences into input event IR.
#[cfg(any(test, doc))]
pub mod chunk_decoder;
#[cfg(not(any(test, doc)))]
pub(crate) mod chunk_decoder;

// Shared AST Types.
#[cfg(any(test, doc))]
pub mod ir_event_types;
#[cfg(not(any(test, doc)))]
pub(crate) mod ir_event_types;
pub use ir_event_types::*;

// Stream Availability Heuristics.
#[cfg(any(test, doc))]
pub mod maybe_more;
#[cfg(not(any(test, doc)))]
pub(crate) mod maybe_more;
pub use maybe_more::*;

// Three-tier test architecture.
#[cfg(any(test, doc))]
pub mod validation_tests;
#[cfg(any(test, doc))]
pub mod unit_tests;
#[cfg(any(test, doc))]
pub mod vt_100_parser_integration_tests;
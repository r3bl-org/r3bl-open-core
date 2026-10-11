// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`OSC`] (Operating System Command) sequence parsing, formatting, and event pipeline.
//!
//! # Architecture & Mental Model: Bidirectional [`OSC`] Pipeline
//!
//! In terminal emulation and CLI environments, [`OSC`] sequences span both outbound
//! generation ([`stdout`]) and inbound event parsing ([`stdin`] and child [`PTY`]):
//!
//! 1. **Outbound Sequence Generation ([`stdout`])**:
//!    - Construct type-safe [`OSC`] escape sequences using [`OscSequence`] and emit them
//!      via [`OscSender`] to query terminal attributes or set terminal state.
//!    - Examples include querying colors ([`OscSequence::ColorQuery`]), setting window
//!      titles ([`OscSequence::SetTitleAndIcon`]), creating hyperlinks
//!      ([`OscSequence::HyperlinkStart`]), or copying to host clipboard
//!      ([`OscSequence::ClipboardSet`]).
//!
//! 2. **Inbound Terminal Response Parsing ([`stdin`])**:
//!    - Terminal responses to queries (e.g. dynamic color reports) arrive asynchronously
//!      over `stdin`.
//!    - The [`vt_100_terminal_input_parser`] frames incoming bytes and uses
//!      [`OscSequence::try_parse`] to decode responses into
//!      [`InputEvent::TerminalColor`].
//!
//! 3. **Child [`PTY`] Stream Interception (Specialized Outlier)**:
//!    - When running child processes inside a [`PTY`], the child may emit out-of-band
//!      `OSC 9;4` build progress notifications (such as Cargo or rustup build status).
//!    - The [`PtyOscProgressScanner`] acts as a specialized streaming pre-filter in the
//!      [`PTY`] reader thread, extracting progress telemetry into [`OscPtyEvent`]
//!      instances for status bars and spinners.
//!    - Standard in-band terminal emulation (such as `OSC 8` hyperlinks and `OSC 0/1/2`
//!      window titles) is handled separately downstream by [`OfsBufVT100`].
//!
//! # Supported [`OSC`] Sequences
//!
//! - **Dynamic Terminal Color Queries & Reports (`OSC 10..19`)**: Query and detect
//!   terminal color schemes (e.g. background color `OSC 11` for light/dark mode
//!   adaptation, foreground `OSC 10`, cursor `OSC 12`). Originates via
//!   [`OscSequence::ColorQuery`] and decodes into [`TerminalColorReport`].
//! - **Progress Reporting (`OSC 9;4`)**: Sequences (`ESC ] 9 ; 4 ... ESC \ (ST)`) used by
//!   Cargo and other build tools to communicate progress information (0-100%, cleared,
//!   error, indeterminate).
//! - **Hyperlinks (`OSC 8`)**: Clickable terminal hyperlinks for opening URLs or file
//!   paths, formatted via [`format_hyperlink`] and [`format_file_hyperlink`], for e.g.,
//!   - `ESC ] 8 ; [params] ; uri ST ... ESC ] 8 ; ; ST`,
//!   - or without params `ESC ] 8 ; ; uri ST ... ESC ] 8 ; ; ST`.
//! - **Window Title and Tabs (`OSC 0`, `OSC 1`, `OSC 2`)**: Setting window title and tab
//!   names via [`OscSender::send_set_title_and_tab`].
//! - **System Clipboard (`OSC 52`)**: Copying text to the host system clipboard or
//!   primary selection buffer over [`stdout`] in remote or headless terminal sessions.
//!
//! # Platform & Backend Availability
//!
//! [`OSC`] sequence handling is split between outbound query emission and inbound
//! response parsing:
//!
//! - **Outbound Request Generation ([`stdout`])**: Cross-platform. Available on all
//!   platforms and terminal backends ([`DirectToAnsi`] on Linux, [`Crossterm`] on macOS
//!   and Windows) via [`OscSender`] and [`OscSequence`].
//! - **Inbound Response Reception ([`stdin`])**: **Linux-only**. Terminal query replies
//!   (e.g. dynamic color reports into [`InputEvent::TerminalColor`]) and unhandled
//!   sequence absorption (e.g. [`OSC 52`] replies) require [`DirectToAnsiInputDevice`].
//!   On macOS and Windows:
//!   1. The underlying [`Crossterm`] backend lacks [`OSC`] parsing support; incoming
//!      responses are misparsed as `Alt + ']'` followed by individual character
//!      keypresses (`1`, `1`, `;`, etc.).
//!   2. Attempting to bypass Crossterm by reading [`stdin`] directly creates an input
//!      stream race condition with Crossterm's background [`EventStream`] reader thread
//!      (a known limitation in Crossterm; see [Issue #963]).
//!
//!   Applications should perform any terminal queries *before* initializing the TUI
//!   event loop or run on Linux where [`DirectToAnsi`] safely arbitrates all [`stdin`]
//!   streams.
//!
//! For end-to-end execution, SSH clipboard behavior, and `Alt+]` input disambiguation,
//! see the [`direct_to_ansi` mod docs: Terminal Protocols & Capabilities Hub][hub].
//!
//! [`Crossterm`]: crate::terminal_lib_backends::TerminalLibBackend::Crossterm
//! [`direct_to_ansi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
//! [`DirectToAnsi`]: crate::terminal_lib_backends::TerminalLibBackend::DirectToAnsi
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`EventStream`]: crossterm::event::EventStream
//! [`format_file_hyperlink`]: crate::core::ansi::osc::format_file_hyperlink
//! [`format_hyperlink`]: crate::core::ansi::osc::format_hyperlink
//! [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
//! [`OfsBufVT100`]: crate::OfsBufVT100
//! [`OSC 52`]: crate::core::ansi::osc::OscSequence::ClipboardSet
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscPtyEvent`]: crate::core::ansi::osc::OscPtyEvent
//! [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
//! [`OscSender::send_set_title_and_tab`]: crate::core::ansi::osc::OscSender::send_set_title_and_tab
//! [`OscSender`]: crate::core::ansi::osc::OscSender
//! [`OscSequence::ClipboardSet`]: crate::core::ansi::osc::OscSequence::ClipboardSet
//! [`OscSequence::ColorQuery`]: crate::core::ansi::osc::OscSequence::ColorQuery
//! [`OscSequence::HyperlinkStart`]: crate::core::ansi::osc::OscSequence::HyperlinkStart
//! [`OscSequence::SetTitleAndIcon`]: crate::core::ansi::osc::OscSequence::SetTitleAndIcon
//! [`OscSequence::try_parse`]: crate::core::ansi::osc::OscSequence::try_parse
//! [`OscSequence`]: crate::core::ansi::osc::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtyOscProgressScanner`]: crate::core::ansi::osc::PtyOscProgressScanner
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`TerminalColorReport`]: crate::TerminalColorReport
//! [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser
//! [hub]: mod@crate::tui::terminal_lib_backends::direct_to_ansi#terminal-protocols--capabilities-hub
//! [Issue #963]: https://github.com/crossterm-rs/crossterm/issues/963

pub mod osc_codes;
mod osc_color;
pub mod osc_hyperlink;
pub mod osc_pty_event;
pub mod osc_sender;
pub mod pty_osc_progress_scanner;

// Re-export main types and functions for convenience.
pub use osc_codes::*;
pub use osc_color::*;
pub use osc_hyperlink::*;
pub use osc_pty_event::*;
pub use osc_sender::*;
pub use pty_osc_progress_scanner::*;

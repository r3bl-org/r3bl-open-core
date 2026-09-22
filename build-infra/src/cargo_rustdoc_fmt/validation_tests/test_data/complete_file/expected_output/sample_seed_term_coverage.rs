// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Seed Term Coverage Test
//!
//! This file exercises every term in the seed dictionary
//! (`known_technical_term_link_dictionary.jsonc`) across three states: bare (plain text),
//! backticked only, and already linked. It also covers edge cases like qualified paths
//! and compound spec terms.
//!
//! ## Tier 1: Internal types (bare)
//!
//! Parses [`CSI`] sequences from the input stream. Applies [`SGR`] codes for styling. Sends
//! [`ESC`] escapes to the terminal. Reads [`DSR`] reports for cursor position. Processes [`OSC`]
//! commands for clipboard and title. Uses the [`pty`] module for pseudoterminal support.
//!
//! ## Tier 1: Internal types (backticked only)
//!
//! Parses [`CSI`] sequences from the input stream. Applies [`SGR`] codes for styling.
//! Sends [`ESC`] escapes to the terminal. Reads [`DSR`] reports for cursor position.
//! Processes [`OSC`] commands for clipboard and title. Uses the [`pty`] module for
//! pseudoterminal support.
//!
//! ## Tier 1: Internal types (already linked)
//!
//! Parses [`CSI`] sequences from the input stream. Applies [`SGR`] codes for styling.
//! Sends [`ESC`] escapes to the terminal. Reads [`DSR`] reports for cursor position.
//! Processes [`OSC`] commands for clipboard and title. Uses the [`pty`] module for
//! pseudoterminal support.
//!
//! ## Tier 1b: Compound spec terms (bare)
//!
//! See the [`CSI` spec] and [`SGR` spec] for details on control sequences. The [`ESC` spec]
//! covers escape codes. The [`DSR` spec] defines device status reports. The [`OSC` spec]
//! handles operating system commands. The [`VT-100` spec] is the original terminal
//! reference.
//!
//! ## Tier 1c: Stdlib types (bare)
//!
//! Writes output to [`stdout`] and errors to [`stderr`] while reading input from [`stdin`] over [`stdio`].
//!
//! ## Tier 1c: Stdlib types (backticked only)
//!
//! Writes output to [`stdout`] and errors to [`stderr`] while reading input from [`stdin`] over
//! [`stdio`].
//!
//! ## Tier 1c: Stdlib types (already linked)
//!
//! Writes output to [`stdout`] and errors to [`stderr`] while reading input from [`stdin`]
//! over [`stdio`].
//!
//! ## Tier 1d: Crate dependencies (bare)
//!
//! Uses [`crossterm`] for terminal control, [`mio`] for non-blocking I/O polling, and
//! [`portable_pty`] for cross-platform pseudoterminal spawning.
//!
//! ## Tier 1d: Crate dependencies (backticked only)
//!
//! Uses [`crossterm`] for terminal control, [`mio`] for non-blocking I/O polling, and
//! [`portable_pty`] for cross-platform pseudoterminal spawning.
//!
//! ## Tier 1d: Crate dependencies (already linked)
//!
//! Uses [`crossterm`] for terminal control, [`mio`] for non-blocking I/O polling, and
//! [`portable_pty`] for cross-platform pseudoterminal spawning.
//!
//! ## Tier 2: External terms (bare)
//!
//! Supports [`ANSI`] escape codes for terminal control. Uses [`ASCII`] text as the base
//! character set. Handles [`UTF-8`] encoding for international text. Sends [`DCS`] device
//! control strings for advanced features. Emulates [`VT-100`] and [`VT-220`] terminals.
//! Uses [`DECSTBM`] for scroll region setup. Built by [`DEC`] in the 1970s. Compatible
//! with [`xterm`] and [`Alacritty`] and [`Kitty`] terminal emulators. Also supports [`RXVT`] and
//! [`rxvt-unicode`] (also called [`urxvt`]) on Linux. Uses [`ConPTY`] on Windows and
//! [`gnome-terminal`] on GNOME desktops. Renders [`ReGIS`] vector graphics and [`Sixel`]
//! bitmap graphics. Tracks [`X10`] mouse events for legacy compatibility. Colors
//! follow the [`ITU-T Rec. T.416`] standard. Runs on a [`tokio`] async runtime. Parses
//! escape sequences with the [`vte`] crate. Allocates a [`PTY`] for each test process.
//! Reads until [`EOF`] and handles [`SIGWINCH`], [`SIGINT`], and [`SIGTERM`] signals. Segments
//! each [`grapheme`] in [`Unicode`] text and renders [`RGB`] or [`truecolor`] palettes. Polls
//! readiness with [`epoll`] on Linux and [`kqueue`] on macOS. Enables [`Bracketed paste`] and
//! configures raw mode via [`termios`].
//!
//! ## Tier 2: External terms (backticked only)
//!
//! Uses [`ANSI`] codes and [`ASCII`] text with [`UTF-8`] encoding. Sends [`DCS`] strings
//! to [`VT-100`] and [`VT-220`] terminals. Sets [`DECSTBM`] scroll regions on [`DEC`]
//! hardware. Runs in [`xterm`] or [`Alacritty`] or [`Kitty`] emulators. Supports [`RXVT`]
//! and [`rxvt-unicode`] (also called [`urxvt`]) on Linux. Uses [`ConPTY`] on Windows
//! and [`gnome-terminal`] on GNOME. Renders [`ReGIS`] and [`Sixel`] graphics. Handles
//! [`X10`] mouse events. Colors per [`ITU-T Rec. T.416`] standard. Powered by [`tokio`]
//! runtime. Parses with [`vte`] crate. Allocates a [`PTY`] for each test process.
//! Reads until [`EOF`] and handles [`SIGWINCH`], [`SIGINT`], and [`SIGTERM`] signals.
//! Segments each [`grapheme`] in [`Unicode`] text and renders [`RGB`] or [`truecolor`]
//! palettes. Polls readiness with [`epoll`] on Linux and [`kqueue`] on macOS. Enables
//! [`Bracketed paste`] and configures raw mode via [`termios`].
//!
//! ## Tier 2: External terms (already linked)
//!
//! Uses [`ANSI`] codes and [`ASCII`] text with [`UTF-8`] encoding. Sends [`DCS`]
//! strings to [`VT-100`] and [`VT-220`] terminals. Sets [`DECSTBM`] scroll regions
//! on [`DEC`] hardware. Runs in [`xterm`] or [`Alacritty`] or [`Kitty`] emulators.
//! Supports [`RXVT`] and [`rxvt-unicode`] (also called [`urxvt`]) on Linux. Uses
//! [`ConPTY`] on Windows and [`gnome-terminal`] on GNOME. Renders [`ReGIS`] and
//! [`Sixel`] graphics. Handles [`X10`] mouse events. Colors per [`ITU-T Rec. T.416`]
//! standard. Powered by [`tokio`] runtime. Parses with [`vte`] crate. Allocates a
//! [`PTY`] for each test process. Reads until [`EOF`] and handles [`SIGWINCH`],
//! [`SIGINT`], and [`SIGTERM`] signals. Segments each [`grapheme`] in [`Unicode`]
//! text and renders [`RGB`] or [`truecolor`] palettes. Polls readiness with
//! [`epoll`] on Linux and [`kqueue`] on macOS. Enables [`Bracketed paste`] and
//! configures raw mode via [`termios`].
//!
//! ## Edge cases: Qualified paths (must NOT be split)
//!
//! Reads from tokio::io::stdin() in a loop. Uses vte::Parser for parsing. Calls
//! std::io::stdout(), crossterm::event::read(), and mio::Poll::new() directly.
//!
//! ## Edge cases: Single colon after term (must still linkify)
//!
//! When first byte is not [`ESC`]:
//!
//! ## Edge cases: Term inside code fence (must NOT linkify)
//!
//! ```text
//! CSI 38 ; 5 ; n m    -- set foreground to ANSI 256 color
//! ESC [ 0 m           -- reset SGR attributes
//! tokio::spawn(async { ... })
//! ```
//!
//! ## Edge cases: Autolink URLs, multi-line links, plain shortcut links, and compound links
//!
//! More info on terminal piping:
//! - <https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>
//!
//! The POSIX [`termios`] flag controls signal generation. See our [Unix
//! implementation's termios section] for details.
//!
//! Controls debug logging for the [`VT-100` PTY output parser].
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`Bracketed paste`]: https://en.wikipedia.org/wiki/Bracketed-paste
//! [`ConPTY`]: https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
//! [`crossterm`]: crossterm
//! [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
//! [`CSI`]: crate::CsiSequence
//! [`DCS`]: https://vt100.net/docs/vt510-rm/chapter4.html#S4.3.4
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
//! [`DSR` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#DSR
//! [`DSR`]: crate::DsrSequence
//! [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
//! [`epoll`]: https://manpages.debian.org/epoll.7
//! [`ESC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#ESC
//! [`ESC`]: crate::EscSequence
//! [`gnome-terminal`]: https://en.wikipedia.org/wiki/GNOME_Terminal
//! [`grapheme`]: https://unicode.org/reports/tr29/
//! [`ITU-T Rec. T.416`]: https://www.itu.int/rec/T-REC-T.416-199303-I
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`kqueue`]: https://en.wikipedia.org/wiki/Kqueue
//! [`mio`]: mio
//! [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`portable_pty`]: portable_pty
//! [`pty`]: crate::core::pty
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`ReGIS`]: https://en.wikipedia.org/wiki/ReGIS
//! [`RGB`]: https://en.wikipedia.org/wiki/RGB_color_model
//! [`rxvt-unicode`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR
//! [`SGR`]: crate::SgrCode
//! [`SIGINT`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGINT
//! [`SIGTERM`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGTERM
//! [`SIGWINCH`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGWINCH
//! [`Sixel`]: https://en.wikipedia.org/wiki/Sixel
//! [`stderr`]: std::io::stderr
//! [`stdin`]: std::io::stdin
//! [`stdio`]: std::io
//! [`stdout`]: std::io::stdout
//! [`termios`]: https://en.wikipedia.org/wiki/Termios
//! [`tokio`]: tokio
//! [`truecolor`]: https://en.wikipedia.org/wiki/Color_depth#True_color_(24-bit)
//! [`Unicode`]: https://en.wikipedia.org/wiki/Unicode
//! [`urxvt`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VT-100` PTY output parser]: crate::core::ansi::vt_100_pty_output_parser
//! [`VT-100` spec]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-220`]: https://en.wikipedia.org/wiki/VT220
//! [`vte`]: https://docs.rs/vte
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [Unix implementation's termios section]: mod@crate::terminal_raw_mode::raw_mode_unix#the-termios-interface

/// # [`CSI`] Edge Cases
///
/// Fixture item testing multi-line inline link normalization and intra-doc ref def
/// precedence over duplicate plain external ref defs.
///
/// <!-- TOC -->
/// - [stdin and termios overview](#stdin-handling)
/// <!-- /TOC -->
///
/// > For complete workflows, see the [coordinates module
/// > documentation].
///
/// - Fast (over 100x faster than [Python-based
///   alternatives]).
///
/// Uses [`termios`] API and **[`termios`]** flags for [`Unicode`]-safe, [`grapheme`]-cluster-aware
/// ([`stdin`]) reading and [`RGB`]/[`truecolor`] rendering.
///
/// Diagram: ![termios and stdin diagram](https://example.com/stdin.png) and
/// [see the stdin section](#stdin-handling), [CSI overview][csi-ref], and
/// [Linux TTY and CSI codes].
///
/// Parses `` `code` `` and `[inline link inside code](https://example.com)` and
/// `` `CSI` `` without converting code spans.
///
/// - Bold (`**text**`), italic (`_text_`), inline code (`` `code` ``)
/// - Images (`![alt](url)`), links (`[text](url)`)
/// - Code spans with terms: `cargo test -p CSI --lib foo -- --nocapture`, `cargo test -p r3bl_tui --lib CSI --
///   --nocapture`, and ``cargo test CSI
///   --nocapture``
///
/// | Stream    | Signal       | Backend   |
/// | --------- | ------------ | --------- |
/// | [`stdin`] | [`SIGWINCH`] | [`epoll`] |
///
/// [`CSI`]: crate::CsiSequence
/// [`epoll`]: https://manpages.debian.org/epoll.7
/// [`grapheme`]: https://unicode.org/reports/tr29/
/// [`RGB`]: https://en.wikipedia.org/wiki/RGB_color_model
/// [`SIGWINCH`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGWINCH
/// [`stdin`]: std::io::stdin
/// [`termios`]: rustix::termios
/// [`truecolor`]: https://en.wikipedia.org/wiki/Color_depth#True_color_(24-bit)
/// [`Unicode`]: https://en.wikipedia.org/wiki/Unicode
/// [coordinates module documentation]: crate::coordinates#common-workflows
/// [csi-ref]: https://example.com/csi-overview
/// [Linux TTY and CSI codes]: https://example.com/tty
/// [Python-based alternatives]: https://github.com/edc/bass
pub struct SeedTermEdgeCaseFixture;


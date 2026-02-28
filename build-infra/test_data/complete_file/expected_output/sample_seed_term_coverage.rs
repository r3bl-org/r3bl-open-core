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
//! commands for clipboard and title.
//!
//! ## Tier 1: Internal types (backticked only)
//!
//! Parses [`CSI`] sequences from the input stream. Applies [`SGR`] codes for styling.
//! Sends [`ESC`] escapes to the terminal. Reads [`DSR`] reports for cursor position.
//! Processes [`OSC`] commands for clipboard and title.
//!
//! ## Tier 1: Internal types (already linked)
//!
//! Parses [`CSI`] sequences from the input stream. Applies [`SGR`] codes for styling.
//! Sends [`ESC`] escapes to the terminal. Reads [`DSR`] reports for cursor position.
//! Processes [`OSC`] commands for clipboard and title.
//!
//! ## Tier 1b: Compound spec terms (bare)
//!
//! See the [`CSI` spec] and [`SGR` spec] for details on control sequences. The [`ESC` spec]
//! covers escape codes. The [`DSR` spec] defines device status reports. The [`OSC` spec]
//! handles operating system commands. The [`VT-100` spec] is the original terminal
//! reference.
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
//! escape sequences with the [`vte`] crate.
//!
//! ## Tier 2: External terms (backticked only)
//!
//! Uses [`ANSI`] codes and [`ASCII`] text with [`UTF-8`] encoding. Sends [`DCS`] strings
//! to [`VT-100`] and [`VT-220`] terminals. Sets [`DECSTBM`] scroll regions on [`DEC`]
//! hardware. Runs in [`xterm`] or [`Alacritty`] or [`Kitty`] emulators. Supports [`RXVT`]
//! and [`rxvt-unicode`] (also called [`urxvt`]) on Linux. Uses [`ConPTY`] on Windows
//! and [`gnome-terminal`] on GNOME. Renders [`ReGIS`] and [`Sixel`] graphics. Handles
//! [`X10`] mouse events. Colors per [`ITU-T Rec. T.416`] standard. Powered by [`tokio`]
//! runtime. Parses with [`vte`] crate.
//!
//! ## Tier 2: External terms (already linked)
//!
//! Uses [`ANSI`] codes and [`ASCII`] text with [`UTF-8`] encoding. Sends [`DCS`]
//! strings to [`VT-100`] and [`VT-220`] terminals. Sets [`DECSTBM`] scroll regions
//! on [`DEC`] hardware. Runs in [`xterm`] or [`Alacritty`] or [`Kitty`] emulators.
//! Supports [`RXVT`] and [`rxvt-unicode`] (also called [`urxvt`]) on Linux. Uses
//! [`ConPTY`] on Windows and [`gnome-terminal`] on GNOME. Renders [`ReGIS`] and
//! [`Sixel`] graphics. Handles [`X10`] mouse events. Colors per [`ITU-T Rec. T.416`]
//! standard. Powered by [`tokio`] runtime. Parses with [`vte`] crate.
//!
//! ## Edge cases: Qualified paths (must NOT be split)
//!
//! Reads from tokio::io::stdin() in a loop. Uses vte::Parser for parsing.
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
//! [`Alacritty`]: https://alacritty.org/
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ConPTY`]: https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
//! [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
//! [`CSI`]: crate::CsiSequence
//! [`DCS`]: https://vt100.net/docs/vt510-rm/chapter4.html#S4.3.4
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
//! [`DSR` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#DSR
//! [`DSR`]: crate::DsrSequence
//! [`ESC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#ESC
//! [`ESC`]: crate::EscSequence
//! [`gnome-terminal`]: https://en.wikipedia.org/wiki/GNOME_Terminal
//! [`ITU-T Rec. T.416`]: https://www.itu.int/rec/T-REC-T.416-199303-I
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`ReGIS`]: https://en.wikipedia.org/wiki/ReGIS
//! [`rxvt-unicode`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR
//! [`SGR`]: crate::SgrCode
//! [`Sixel`]: https://en.wikipedia.org/wiki/Sixel
//! [`tokio`]: tokio
//! [`urxvt`]: https://en.wikipedia.org/wiki/Rxvt-unicode
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VT-100` spec]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-220`]: https://en.wikipedia.org/wiki/VT220
//! [`vte`]: https://docs.rs/vte
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm

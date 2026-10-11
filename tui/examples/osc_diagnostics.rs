// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Interactive [`OSC`] (Operating System Command) diagnostics and testing suite.
//!
//! This example exercises both outbound sequence generation and inbound response parsing
//! across the bidirectional [`OSC`] pipeline:
//!
//! - **Outbound Requests**: Emits [`OSC`] commands via [`OscSender`] to query terminal
//!   colors, set window titles, format clickable hyperlinks, write to clipboard, and
//!   report build progress.
//! - **Inbound Responses**: Reads and decodes asynchronous terminal responses via
//!   [`InputDevice`] into [`InputEvent::TerminalColor`] variants.
//! - **Platform Awareness**: Detects the active [`TERMINAL_LIB_BACKEND`] and displays
//!   capabilities and platform constraints.
//!
//! ## Platform Support & Constraints
//!
//! | Capability                         | Linux ([`DirectToAnsi`]) | macOS / Windows ([`Crossterm`]) |
//! | :--------------------------------- | :----------------------- | :------------------------------ |
//! | **Outbound [`OSC`] Generation**    | Yes: Supported           | Yes: Supported                  |
//! | **Inbound Terminal Color Parsing** | Yes: Framed & Decoded    | No: Misparsed by Crossterm      |
//! | **Window Titles (`OSC 0/2`)**      | Yes: Supported           | Yes: Supported                  |
//! | **Hyperlinks (`OSC 8`)**           | Yes: Supported           | Yes: Supported                  |
//! | **Clipboard (`OSC 52`)**           | Yes: Supported           | Yes: Supported                  |
//! | **Progress Updates (`OSC 9;4`)**   | Yes: Supported           | Yes: Supported                  |
//!
//! > ⚠️ **macOS / Windows Note**: Under Crossterm, terminal query responses (such as
//! > background color reports) are not framed by Crossterm's event reader and will not
//! > produce [`InputEvent::TerminalColor`]. Running on Linux with [`DirectToAnsi`]
//! > exercises the full bidirectional loop.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example osc_diagnostics
//! ```
//!
//! ### Controls
//!
//! | Key             | Operation           | Description                                                                             |
//! | :-------------- | :------------------ | :-------------------------------------------------------------------------------------- |
//! | `Shift+1` / `!` | Background Color    | Query terminal background color ([`TerminalColorRole::Background`], `OSC 11`)           |
//! | `Shift+2` / `@` | Foreground Color    | Query terminal foreground color ([`TerminalColorRole::Foreground`], `OSC 10`)           |
//! | `Shift+3` / `#` | Cursor Color        | Query terminal cursor color ([`TerminalColorRole::Cursor`], `OSC 12`)                   |
//! | `Shift+4` / `$` | All Color Roles     | Query all 7 supported terminal color roles (`OSC 10..19`)                               |
//! | `Shift+5` / `%` | Terminal Title      | Set terminal title and tab header ([`OscSender::send_set_title_and_tab`], `OSC 0`)      |
//! | `Shift+6` / `^` | Hyperlink           | Emit clickable terminal hyperlink ([`OscSender::send_set_hyperlink`], `OSC 8`)          |
//! | `Shift+7` / `&` | Clipboard           | Copy text to system clipboard ([`OscSender::send_set_system_clipboard`], `OSC 52`)      |
//! | `Shift+8` / `*` | Progress Bar        | Cycle simulated build progress percentage ([`OscSender::send_set_progress`], `OSC 9;4`) |
//! | `Shift+9` / `(` | Run All Diagnostics | Sequentially execute all diagnostics in automated sequence                              |
//! | `Shift+0` / `)` | Clear Event Log     | Clear event history log and received color query swatches                               |
//! | `q` / `Ctrl+C`  | Quit                | Exit the diagnostics dashboard                                                          |
//!
//! [`Crossterm`]: crate::TerminalLibBackend::Crossterm
//! [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
//! [`OSC`]: crate::OscSequence
//! [`OscSender::send_set_hyperlink`]: crate::OscSender::send_set_hyperlink
//! [`OscSender::send_set_progress`]: crate::OscSender::send_set_progress
//! [`OscSender::send_set_system_clipboard`]: crate::OscSender::send_set_system_clipboard
//! [`OscSender::send_set_title_and_tab`]: crate::OscSender::send_set_title_and_tab
//! [`OscSender`]: crate::OscSender
//! [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
//! [`TerminalColorRole::Background`]: crate::TerminalColorRole::Background
//! [`TerminalColorRole::Cursor`]: crate::TerminalColorRole::Cursor
//! [`TerminalColorRole::Foreground`]: crate::TerminalColorRole::Foreground

use crate::{app::{MAX_EVENT_LOG_ENTRIES, OscDiagnosticsApp},
            box_drawing::{Placement, detect_width, make_box_line},
            terminal_output::{clear_screen_and_home, cursor_to, flush, print_text}};
use r3bl_tui::{InputDevice, InputEvent, Key, KeyPress, KeyState, ModifierKeysMask,
               NarrowingCastToU16, OscSender, OutputDevice, PaintMode, RgbValue,
               TERMINAL_LIB_BACKEND, TerminalColorReport, TerminalColorRole,
               TerminalLibBackend, TerminalModeController, WideningCastToUsize,
               ansi_output, assert_terminal_is_interactive, get_size, ok, pc,
               set_mimalloc_in_main, try_initialize_logging_global, vp_col, vp_row};
use std::{collections::{HashMap, VecDeque},
          time::Instant};
use strip_ansi_escapes::strip_str;
use tracing_core::LevelFilter;
use ui::render;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    // Initialize logging to `/tmp/r3bl_tui/log.txt`.
    let _log_guard = try_initialize_logging_global(LevelFilter::DEBUG).ok();
    tracing::debug!("Starting OSC Diagnostics Dashboard");

    // Initialize I/O devices.
    let output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::default();

    // Start raw mode and full screen TUI.
    let _raw_mode_guard = output_device.enter_raw_mode()?;
    let _fullscreen_tui_mode_guard = output_device.setup_full_screen_tui()?;

    let mut app = OscDiagnosticsApp::new();
    app.add_log("Dashboard initialized. Ready for OSC testing.");

    // Initial render.
    render(&app, &output_device);

    // Main event loop.
    while !app.should_quit {
        if let Some(event) = input_device.next().await {
            match event {
                InputEvent::Keyboard(key_press) => {
                    app.handle_key_press(key_press, &output_device)?;
                }
                InputEvent::TerminalColor(report) => {
                    app.handle_terminal_color_report(report);
                }
                _ => {}
            }

            render(&app, &output_device);
        }
    }

    if matches!(output_device.paint_mode, PaintMode::Real) {
        output_device.flush()?;
    }

    ok!()
}

mod app {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Maximum entries retained in the visible event log.
    pub const MAX_EVENT_LOG_ENTRIES: usize = 12;

    /// Diagnostic application state.
    #[derive(Debug)]
    pub struct OscDiagnosticsApp {
        /// Active color reports received from the terminal emulator.
        pub color_reports: HashMap<TerminalColorRole, (RgbValue, Instant)>,

        /// History of log entries shown in the event log.
        pub event_log: VecDeque<String>,

        /// Last raw [`OSC`] sequence emitted to the terminal display.
        ///
        /// [`OSC`]: crate::core::ansi::osc::OscSequence
        pub last_emitted_osc: Option<String>,

        /// Current simulated progress value (0, 25, 50, 75, 100, or cleared).
        pub current_progress_step: usize,

        /// Whether the application should exit.
        pub should_quit: bool,
    }

    impl Default for OscDiagnosticsApp {
        fn default() -> Self { Self::new() }
    }

    impl OscDiagnosticsApp {
        #[must_use]
        pub fn new() -> Self {
            Self {
                color_reports: HashMap::new(),
                event_log: VecDeque::new(),
                last_emitted_osc: None,
                current_progress_step: 0,
                should_quit: false,
            }
        }

        pub fn add_log(&mut self, message: impl Into<String>) {
            if self.event_log.len() >= MAX_EVENT_LOG_ENTRIES {
                self.event_log.pop_front();
            }
            self.event_log.push_back(message.into());
        }

        pub fn handle_terminal_color_report(&mut self, report: TerminalColorReport) {
            tracing::debug!(?report, "Received terminal color reply");
            let hex = format!(
                "#{:02x}{:02x}{:02x}",
                report.color.red, report.color.green, report.color.blue
            );
            let role_name = format!("{:?}", report.role);
            self.add_log(format!(
                "📥 Received color reply: {role_name} -> {hex} (R:{}, G:{}, B:{})",
                report.color.red, report.color.green, report.color.blue
            ));
            self.color_reports
                .insert(report.role, (report.color, Instant::now()));
        }

        #[allow(clippy::too_many_lines)]
        pub fn handle_key_press(
            &mut self,
            key: KeyPress,
            output_device: &OutputDevice,
        ) -> miette::Result<()> {
            tracing::debug!(?key, "Processing key press");
            match key {
                // Quit on `q` or `Ctrl+C`.
                KeyPress::Plain {
                    key: Key::Character('q' | 'Q'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('c'),
                    mask:
                        ModifierKeysMask {
                            ctrl_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.should_quit = true;
                }

                // Shift+0 or `)`: Clear log and color queries.
                KeyPress::Plain {
                    key: Key::Character(')'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('0' | ')'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.color_reports.clear();
                    self.event_log.clear();
                    self.add_log("🧹 Event log cleared.");
                }

                // Shift+1 or `!`: Query Background color.
                KeyPress::Plain {
                    key: Key::Character('!'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('1' | '!'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.query_color(output_device, TerminalColorRole::Background)?;
                }

                // Shift+2 or `@`: Query Foreground color.
                KeyPress::Plain {
                    key: Key::Character('@'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('2' | '@'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.query_color(output_device, TerminalColorRole::Foreground)?;
                }

                // Shift+3 or `#`: Query Cursor color.
                KeyPress::Plain {
                    key: Key::Character('#'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('3' | '#'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.query_color(output_device, TerminalColorRole::Cursor)?;
                }

                // Shift+4 or `$`: Query all supported color roles.
                KeyPress::Plain {
                    key: Key::Character('$'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('4' | '$'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.query_all_colors(output_device)?;
                }

                // Shift+5 or `%`: Set terminal window title.
                KeyPress::Plain {
                    key: Key::Character('%'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('5' | '%'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.set_title(output_device, "R3BL OSC Diagnostics - Active")?;
                }

                // Shift+6 or `^`: Emit clickable hyperlink.
                KeyPress::Plain {
                    key: Key::Character('^'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('6' | '^'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.emit_hyperlink(output_device)?;
                }

                // Shift+7 or `&`: Write to system clipboard via OSC 52.
                KeyPress::Plain {
                    key: Key::Character('&'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('7' | '&'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.copy_to_clipboard(
                        output_device,
                        "Hello from R3BL OSC Diagnostics!",
                    )?;
                }

                // Shift+8 or `*`: Cycle build progress (OSC 9;4).
                KeyPress::Plain {
                    key: Key::Character('*'),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('8' | '*'),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.cycle_progress(output_device)?;
                }

                // Shift+9 or `(`: Run all diagnostics.
                KeyPress::Plain {
                    key: Key::Character('('),
                }
                | KeyPress::WithModifiers {
                    key: Key::Character('9' | '('),
                    mask:
                        ModifierKeysMask {
                            shift_key_state: KeyState::Pressed,
                            ..
                        },
                } => {
                    self.run_all_diagnostics(output_device)?;
                }

                other => {
                    tracing::debug!(?other, "Unhandled key event");
                    if matches!(TERMINAL_LIB_BACKEND, TerminalLibBackend::Crossterm) {
                        self.add_log(format!("⚡ Crossterm raw key: {other:?}"));
                    }
                }
            }

            ok!()
        }

        pub fn query_color(
            &mut self,
            output_device: &OutputDevice,
            role: TerminalColorRole,
        ) -> miette::Result<()> {
            tracing::debug!(?role, osc_code = %role, "Sending terminal color query");
            let mut sender = OscSender::new(output_device);
            sender.send_color_query(role)?;
            self.last_emitted_osc = Some(format!("ESC ] {role} ; ? ST"));
            self.add_log(format!("📤 Emitted query: {role:?} (OSC {role})"));
            ok!()
        }

        pub fn query_all_colors(
            &mut self,
            output_device: &OutputDevice,
        ) -> miette::Result<()> {
            let roles = [
                TerminalColorRole::Background,
                TerminalColorRole::Foreground,
                TerminalColorRole::Cursor,
                TerminalColorRole::MouseForeground,
                TerminalColorRole::MouseBackground,
                TerminalColorRole::Highlight,
                TerminalColorRole::HighlightForeground,
            ];

            for role in roles {
                self.query_color(output_device, role)?;
            }

            ok!()
        }

        pub fn set_title(
            &mut self,
            output_device: &OutputDevice,
            title: &str,
        ) -> miette::Result<()> {
            let mut sender = OscSender::new(output_device);
            sender.send_set_title_and_tab(title)?;
            self.last_emitted_osc = Some(format!("ESC ] 0 ; {title} ST"));
            self.add_log(format!("📤 Set terminal title: \"{title}\""));
            ok!()
        }

        pub fn emit_hyperlink(
            &mut self,
            output_device: &OutputDevice,
        ) -> miette::Result<()> {
            let mut sender = OscSender::new(output_device);
            let uri = "https://r3bl.com";
            sender.send_set_hyperlink(uri, None::<&str>)?;
            sender.send_clear_hyperlink()?;
            self.last_emitted_osc =
                Some(format!("ESC ] 8 ; ; {uri} ST ... ESC ] 8 ; ; ST"));
            self.add_log(format!("📤 Emitted OSC 8 hyperlink: {uri}"));
            ok!()
        }

        pub fn copy_to_clipboard(
            &mut self,
            output_device: &OutputDevice,
            text: &str,
        ) -> miette::Result<()> {
            let mut sender = OscSender::new(output_device);
            sender.send_set_system_clipboard(text)?;
            self.last_emitted_osc = Some("ESC ] 52 ; c ; <base64> ST".to_string());
            self.add_log(format!("📤 Copied to clipboard via OSC 52: \"{text}\""));
            ok!()
        }

        pub fn cycle_progress(
            &mut self,
            output_device: &OutputDevice,
        ) -> miette::Result<()> {
            let steps = [
                pc!(25).ok(),
                pc!(50).ok(),
                pc!(75).ok(),
                pc!(100).ok(),
                None, // Clear progress
            ];

            let current = steps[self.current_progress_step % steps.len()];
            self.current_progress_step = self.current_progress_step.wrapping_add(1);

            let mut sender = OscSender::new(output_device);
            if let Some(pct) = current {
                sender.send_set_progress(pct)?;
                self.last_emitted_osc = Some(format!("ESC ] 9 ; 4 ; 1 ; {} ST", *pct));
                self.add_log(format!("📤 Emitted progress: {pct:?}"));
            } else {
                sender.send_clear_progress()?;
                self.last_emitted_osc = Some("ESC ] 9 ; 4 ; 0 ; 0 ST".to_string());
                self.add_log("📤 Emitted progress cleared");
            }

            ok!()
        }

        pub fn run_all_diagnostics(
            &mut self,
            output_device: &OutputDevice,
        ) -> miette::Result<()> {
            self.add_log("🚀 Running all OSC diagnostics...");
            self.set_title(output_device, "R3BL OSC Diagnostics - Full Suite")?;
            self.emit_hyperlink(output_device)?;
            self.copy_to_clipboard(output_device, "R3BL OSC Diagnostics Automated Test")?;
            self.cycle_progress(output_device)?;
            self.query_all_colors(output_device)?;
            ok!()
        }
    }
}

/// Unicode box-drawing utilities for rendering rectangular terminal dashboard panels.
mod box_drawing {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Fallback visual column width of all diagnostic dashboard boxes.
    pub const MAX_WIDTH: u16 = 73;

    /// Detects current terminal column width via [`get_size()`], falling back to
    /// [`MAX_WIDTH`].
    #[must_use]
    pub fn detect_width() -> u16 {
        match get_size() {
            Ok(size) if size.col_width.as_u16() > 0 => size.col_width.as_u16(),
            _ => MAX_WIDTH,
        }
    }

    /// Placement of a border or content line within a rectangular box.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Placement {
        /// Top border with optional title header (`┌─ {title} ─...─┐` or `┌─...─┐`).
        Top,
        /// Content row enclosed between vertical borders (`│ {content} │`).
        Middle,
        /// Bottom enclosing border (`└─...─┘`).
        Bottom,
    }

    /// Formats a complete single-line box component for the given [`Placement`].
    #[must_use]
    pub fn make_box_line(placement: Placement, content: &str, max_width: u16) -> String {
        let max_w = max_width.as_usize_widening();
        match placement {
            Placement::Top => {
                if content.is_empty() {
                    let dashes = "─".repeat(max_w.saturating_sub(2));
                    format!("┌{dashes}┐")
                } else {
                    let cw = visual_width(content);
                    let dashes_count = max_w.saturating_sub(cw + 5);
                    let dashes = "─".repeat(dashes_count);
                    format!("┌─ {content} {dashes}┐")
                }
            }
            Placement::Middle => {
                let inner_w = max_w.saturating_sub(3);
                let cw = visual_width(content);
                if cw <= inner_w {
                    let padding = " ".repeat(inner_w - cw);
                    format!("│ {content}{padding}│")
                } else {
                    let clean = strip_str(content);
                    let mut truncated = String::new();
                    let mut width = 0;
                    for ch in clean.chars() {
                        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if width + ch_w > inner_w {
                            break;
                        }
                        truncated.push(ch);
                        width += ch_w;
                    }
                    let padding = " ".repeat(inner_w - width);
                    format!("│ {truncated}{padding}│")
                }
            }
            Placement::Bottom => {
                let dashes = "─".repeat(max_w.saturating_sub(2));
                format!("└{dashes}┘")
            }
        }
    }

    /// Calculates the visible terminal column width of a string, ignoring [`ANSI`] escape
    /// codes.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    #[must_use]
    pub fn visual_width(s: &str) -> usize {
        let clean = strip_str(s);
        UnicodeWidthStr::width(clean.as_str())
    }
}

/// Direct terminal [`ANSI`] escape code output helpers for low-level screen manipulation.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
mod terminal_output {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Clears the screen and positions cursor at home (0,0).
    pub fn clear_screen_and_home(output: &OutputDevice) {
        output.write(|out| {
            let _unused =
                out.write_all(ansi_output::screen_clearing::clear_screen().as_bytes());
            let _unused = out.write_all(
                ansi_output::cursor_movement::cursor_position(
                    vp_row(0).into(),
                    vp_col(0).into(),
                )
                .as_bytes(),
            );
            let _unused = out.flush();
        });
    }

    /// Moves cursor to specified terminal position (1-based row, 1-based col).
    pub fn cursor_to(output: &OutputDevice, term_row: u16, term_col: u16) {
        output.write(|out| {
            let _unused = out.write_all(
                ansi_output::cursor_movement::cursor_position(
                    vp_row(term_row.saturating_sub(1)).into(),
                    vp_col(term_col.saturating_sub(1)).into(),
                )
                .as_bytes(),
            );
        });
    }

    /// Writes text at current cursor position.
    pub fn print_text(output: &OutputDevice, text: &str) {
        output.write(|out| {
            let _unused = out.write_all(text.as_bytes());
        });
    }

    /// Flushes output to ensure all bytes are transmitted to the terminal.
    pub fn flush(output: &OutputDevice) {
        output.write(|out| {
            let _unused = out.flush();
        });
    }
}

/// Centralized catalog of user-interface text constants and dynamic string formatters.
mod ui_strings {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Header dashboard title.
    #[must_use]
    pub fn dashboard_title() -> &'static str { "OSC Diagnostics Dashboard" }

    /// Header dashboard subtitle.
    #[must_use]
    pub fn dashboard_subtitle() -> &'static str {
        "Interactive OSC Sequence Generator & Inbound Terminal Response Reader"
    }

    /// Backend status panel title.
    #[must_use]
    pub fn backend_status_title() -> &'static str { "Backend Status" }

    /// Active backend description when running under [`DirectToAnsi`].
    ///
    /// [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
    #[must_use]
    pub fn direct_to_ansi_active() -> &'static str {
        "Active: DirectToAnsi (Linux pure Rust async I/O)"
    }

    /// Bidirectional capability note when running under [`DirectToAnsi`].
    ///
    /// [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
    #[must_use]
    pub fn direct_to_ansi_bidirectional() -> &'static str {
        "Bidirectional OSC: Outbound generation + Inbound color replies"
    }

    /// Framing status when running under [`DirectToAnsi`].
    ///
    /// [`DirectToAnsi`]: crate::TerminalLibBackend::DirectToAnsi
    #[must_use]
    pub fn direct_to_ansi_status() -> &'static str {
        "Status: Full bidirectional framing active"
    }

    /// Active backend description when running under [`Crossterm`].
    ///
    /// [`Crossterm`]: crate::TerminalLibBackend::Crossterm
    #[must_use]
    pub fn crossterm_active() -> &'static str {
        "Active: Crossterm (macOS / Windows compatibility backend)"
    }

    /// Warning displayed when running under [`Crossterm`].
    ///
    /// [`Crossterm`]: crate::TerminalLibBackend::Crossterm
    #[must_use]
    pub fn crossterm_warning() -> &'static str {
        "⚠️ Warning: Inbound color replies are not framed by Crossterm"
    }

    /// Outbound capability note when running under [`Crossterm`].
    ///
    /// [`Crossterm`]: crate::TerminalLibBackend::Crossterm
    #[must_use]
    pub fn crossterm_note() -> &'static str {
        "Replies leak as raw keystrokes below (Linux frames into clean events)"
    }

    /// Controls panel title.
    #[must_use]
    pub fn controls_title() -> &'static str { "Controls" }

    /// Minimum Column 2 starting offset to ensure spacing between columns.
    pub const CONTROLS_MIN_COL2_OFFSET: usize = 41;

    /// Calculates the Column 2 starting offset based on the available terminal width,
    /// ensuring it never falls below [`CONTROLS_MIN_COL2_OFFSET`].
    #[must_use]
    pub fn col2_offset_for_width(width: u16) -> usize {
        let inner_w = width.as_usize_widening().saturating_sub(3);
        (inner_w / 2).max(CONTROLS_MIN_COL2_OFFSET)
    }

    /// Formats two controls into a single two-column string, padding the first column
    /// so the second column begins cleanly at the specified offset.
    #[must_use]
    pub fn format_two_columns(col1: &str, col2: &str, col2_offset: usize) -> String {
        if col2.is_empty() {
            col1.to_string()
        } else {
            let col1_width = box_drawing::visual_width(col1);
            let padding_len = col2_offset.saturating_sub(col1_width);
            let padding = " ".repeat(padding_len);
            format!("{col1}{padding}{col2}")
        }
    }

    /// First row of keyboard controls.
    #[must_use]
    pub fn controls_line_1(width: u16) -> String {
        format_two_columns(
            "[Shift+1] Query Bg Color (OSC 11)",
            "[Shift+5] Set Title (OSC 0)",
            col2_offset_for_width(width),
        )
    }

    /// Second row of keyboard controls.
    #[must_use]
    pub fn controls_line_2(width: u16) -> String {
        format_two_columns(
            "[Shift+2] Query Fg Color (OSC 10)",
            "[Shift+6] Hyperlink (OSC 8)",
            col2_offset_for_width(width),
        )
    }

    /// Third row of keyboard controls.
    #[must_use]
    pub fn controls_line_3(width: u16) -> String {
        format_two_columns(
            "[Shift+3] Query Cursor Color (OSC 12)",
            "[Shift+7] Clipboard (OSC 52)",
            col2_offset_for_width(width),
        )
    }

    /// Fourth row of keyboard controls.
    #[must_use]
    pub fn controls_line_4(width: u16) -> String {
        format_two_columns(
            "[Shift+4] Query All Colors (OSC 10..19)",
            "[Shift+8] Progress (OSC 9;4)",
            col2_offset_for_width(width),
        )
    }

    /// Fifth row of keyboard controls.
    #[must_use]
    pub fn controls_line_5(width: u16) -> String {
        format_two_columns(
            "[Shift+9] Run All",
            "[Shift+0] Clear Log",
            col2_offset_for_width(width),
        )
    }

    /// Sixth row of keyboard controls.
    #[must_use]
    pub fn controls_line_6(width: u16) -> String {
        format_two_columns("[Q / Ctrl+C] Quit", "", col2_offset_for_width(width))
    }

    /// Last emitted [`OSC`] panel title.
    ///
    /// [`OSC`]: crate::OscSequence
    #[must_use]
    pub fn last_emitted_title() -> &'static str { "Last Emitted OSC Sequence" }

    /// Fallback message when no [`OSC`] sequence has been emitted yet.
    ///
    /// [`OSC`]: crate::OscSequence
    #[must_use]
    pub fn last_emitted_fallback() -> &'static str {
        "None (press [Shift+1]-[Shift+9] to emit)"
    }

    /// Terminal color queries panel title.
    #[must_use]
    pub fn color_queries_title() -> &'static str {
        "Terminal Color Queries Received (OSC 10..19)"
    }

    /// Event history log panel title.
    #[must_use]
    pub fn event_log_title() -> &'static str { "Event History Log" }

    /// Formats a single terminal color query status line with optional RGB swatch.
    #[must_use]
    pub fn color_query_line(
        role: TerminalColorRole,
        report: Option<&(RgbValue, Instant)>,
    ) -> String {
        let label = format!("{role:?} (OSC {role})");
        if let Some((rgb, _time)) = report {
            let swatch =
                format!("\x1b[48;2;{};{};{}m  \x1b[0m", rgb.red, rgb.green, rgb.blue);
            let hex = format!("#{:02x}{:02x}{:02x}", rgb.red, rgb.green, rgb.blue);
            format!(
                "{label:<22} -> {hex} (R:{:>3}, G:{:>3}, B:{:>3})    swatch: {swatch}",
                rgb.red, rgb.green, rgb.blue
            )
        } else {
            format!("{label:<22} -> (not queried or pending reply)")
        }
    }
}

/// Terminal UI rendering subsystem for presenting the diagnostics dashboard.
mod ui {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Renders the complete diagnostics UI to the terminal display.
    pub fn render(app: &OscDiagnosticsApp, output: &OutputDevice) {
        let width = detect_width();
        clear_screen_and_home(output);
        render_header(output, width);
        render_backend_status(output, width);
        render_controls(output, width);
        render_last_emitted_osc(app, output, width);
        render_color_queries(app, output, width);
        render_event_log(app, output, width);
        flush(output);
    }

    fn render_header(output: &OutputDevice, width: u16) {
        cursor_to(output, 1, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::dashboard_title(), width),
        );
        cursor_to(output, 2, 1);
        print_text(
            output,
            &make_box_line(Placement::Middle, ui_strings::dashboard_subtitle(), width),
        );
        cursor_to(output, 3, 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }

    fn render_backend_status(output: &OutputDevice, width: u16) {
        cursor_to(output, 4, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::backend_status_title(), width),
        );
        cursor_to(output, 5, 1);
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::DirectToAnsi => {
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::direct_to_ansi_active(),
                        width,
                    ),
                );
                cursor_to(output, 6, 1);
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::direct_to_ansi_bidirectional(),
                        width,
                    ),
                );
                cursor_to(output, 7, 1);
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::direct_to_ansi_status(),
                        width,
                    ),
                );
            }
            TerminalLibBackend::Crossterm => {
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::crossterm_active(),
                        width,
                    ),
                );
                cursor_to(output, 6, 1);
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::crossterm_warning(),
                        width,
                    ),
                );
                cursor_to(output, 7, 1);
                print_text(
                    output,
                    &make_box_line(
                        Placement::Middle,
                        ui_strings::crossterm_note(),
                        width,
                    ),
                );
            }
        }
        cursor_to(output, 8, 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }

    fn render_controls(output: &OutputDevice, width: u16) {
        cursor_to(output, 9, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::controls_title(), width),
        );
        cursor_to(output, 10, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_1(width),
                width,
            ),
        );
        cursor_to(output, 11, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_2(width),
                width,
            ),
        );
        cursor_to(output, 12, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_3(width),
                width,
            ),
        );
        cursor_to(output, 13, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_4(width),
                width,
            ),
        );
        cursor_to(output, 14, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_5(width),
                width,
            ),
        );
        cursor_to(output, 15, 1);
        print_text(
            output,
            &make_box_line(
                Placement::Middle,
                &ui_strings::controls_line_6(width),
                width,
            ),
        );
        cursor_to(output, 16, 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }

    fn render_last_emitted_osc(
        app: &OscDiagnosticsApp,
        output: &OutputDevice,
        width: u16,
    ) {
        cursor_to(output, 17, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::last_emitted_title(), width),
        );
        cursor_to(output, 18, 1);
        let last_osc_str = app
            .last_emitted_osc
            .as_deref()
            .unwrap_or(ui_strings::last_emitted_fallback());
        print_text(
            output,
            &make_box_line(Placement::Middle, last_osc_str, width),
        );
        cursor_to(output, 19, 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }

    fn render_color_queries(app: &OscDiagnosticsApp, output: &OutputDevice, width: u16) {
        cursor_to(output, 20, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::color_queries_title(), width),
        );

        let display_roles = [
            TerminalColorRole::Background,
            TerminalColorRole::Foreground,
            TerminalColorRole::Cursor,
            TerminalColorRole::Highlight,
        ];

        for (i, role) in display_roles.iter().enumerate() {
            let row = 21 + i.as_u16_narrowing();
            cursor_to(output, row, 1);
            let line_content =
                ui_strings::color_query_line(*role, app.color_reports.get(role));
            print_text(
                output,
                &make_box_line(Placement::Middle, &line_content, width),
            );
        }

        cursor_to(output, 25, 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }

    fn render_event_log(app: &OscDiagnosticsApp, output: &OutputDevice, width: u16) {
        cursor_to(output, 26, 1);
        print_text(
            output,
            &make_box_line(Placement::Top, ui_strings::event_log_title(), width),
        );

        for i in 0..MAX_EVENT_LOG_ENTRIES {
            let row = 27 + i.as_u16_narrowing();
            cursor_to(output, row, 1);
            let line_content = app.event_log.get(i).map_or("", String::as_str);
            print_text(
                output,
                &make_box_line(Placement::Middle, line_content, width),
            );
        }

        cursor_to(output, 27 + MAX_EVENT_LOG_ENTRIES.as_u16_narrowing(), 1);
        print_text(output, &make_box_line(Placement::Bottom, "", width));
    }
}

#[cfg(test)]
mod tests {
    use super::{app::OscDiagnosticsApp,
                box_drawing::{MAX_WIDTH, Placement, make_box_line, visual_width}};
    use r3bl_tui::{Key, KeyPress, OutputDevice, OutputDeviceExt, RgbValue,
                   TerminalColorRole, WideningCastToUsize};
    use std::time::Instant;

    #[test]
    fn test_make_box_line_widths() {
        let expected_w = MAX_WIDTH.as_usize_widening();

        let top = make_box_line(Placement::Top, "OSC Diagnostics Dashboard", MAX_WIDTH);
        assert_eq!(visual_width(&top), expected_w);

        let middle = make_box_line(Placement::Middle, "🧹 Event log cleared.", MAX_WIDTH);
        assert_eq!(visual_width(&middle), expected_w);

        let empty_middle = make_box_line(Placement::Middle, "", MAX_WIDTH);
        assert_eq!(visual_width(&empty_middle), expected_w);

        let bottom = make_box_line(Placement::Bottom, "", MAX_WIDTH);
        assert_eq!(visual_width(&bottom), expected_w);
    }

    #[test]
    fn test_clear_event_log_and_colors() {
        let mut app = OscDiagnosticsApp::new();
        app.color_reports.insert(
            TerminalColorRole::Background,
            (
                RgbValue {
                    red: 1,
                    green: 2,
                    blue: 3,
                },
                Instant::now(),
            ),
        );
        app.event_log.push_back("test".to_string());

        let (out, _) = OutputDevice::new_mock();
        let key = KeyPress::Plain {
            key: Key::Character(')'),
        };
        app.handle_key_press(key, &out).unwrap();
        assert!(app.color_reports.is_empty());
        assert_eq!(app.event_log.len(), 1);
        assert_eq!(app.event_log[0], "🧹 Event log cleared.");
    }

    #[test]
    fn test_ui_strings() {
        use super::ui_strings;

        assert_eq!(ui_strings::dashboard_title(), "OSC Diagnostics Dashboard");
        assert!(!ui_strings::dashboard_subtitle().is_empty());
        assert!(!ui_strings::backend_status_title().is_empty());
        assert!(!ui_strings::controls_title().is_empty());

        let pending = ui_strings::color_query_line(TerminalColorRole::Background, None);
        assert!(pending.contains("Background"));
        assert!(pending.contains("not queried or pending reply"));

        let report = (
            RgbValue {
                red: 12,
                green: 34,
                blue: 56,
            },
            Instant::now(),
        );
        let populated =
            ui_strings::color_query_line(TerminalColorRole::Foreground, Some(&report));
        assert!(populated.contains("Foreground"));
        assert!(populated.contains("#0c2238"));
        assert!(populated.contains("R: 12, G: 34, B: 56"));
    }

    #[test]
    fn test_format_two_columns() {
        use super::ui_strings;
        let formatted = ui_strings::format_two_columns("[A] First", "[B] Second", 20);
        assert_eq!(formatted.chars().nth(20), Some('['));
        assert_eq!(&formatted[..9], "[A] First");
        assert_eq!(&formatted[20..], "[B] Second");
    }

    #[test]
    fn test_col2_offset_for_width() {
        use super::{box_drawing::MAX_WIDTH, ui_strings};
        // At MAX_WIDTH (73), inner_w = 70. 70 / 2 = 35, clamped to
        // CONTROLS_MIN_COL2_OFFSET (41).
        assert_eq!(ui_strings::col2_offset_for_width(MAX_WIDTH), 41);
        // At width 120, inner_w = 117. 117 / 2 = 58.
        assert_eq!(ui_strings::col2_offset_for_width(120), 58);
    }

    #[test]
    fn test_controls_column_alignment() {
        use super::{box_drawing::MAX_WIDTH, ui_strings};
        let lines_with_col2 = [
            ui_strings::controls_line_1(MAX_WIDTH),
            ui_strings::controls_line_2(MAX_WIDTH),
            ui_strings::controls_line_3(MAX_WIDTH),
            ui_strings::controls_line_4(MAX_WIDTH),
            ui_strings::controls_line_5(MAX_WIDTH),
        ];
        // Verify lines 1-5 have Column 2 starting at index 41 ('[').
        for (i, line) in lines_with_col2.iter().enumerate() {
            let col2_char = line.chars().nth(41);
            assert_eq!(
                col2_char,
                Some('['),
                "Line {} does not have Column 2 bracket at index 41: {:?}",
                i + 1,
                line
            );
        }

        // Verify line 6 is Quit.
        let line_6 = ui_strings::controls_line_6(MAX_WIDTH);
        assert!(line_6.contains("[Q / Ctrl+C] Quit"));

        // Also verify wide terminal (width 120 -> col2 offset 58).
        let wide_lines = [
            ui_strings::controls_line_1(120),
            ui_strings::controls_line_2(120),
            ui_strings::controls_line_3(120),
            ui_strings::controls_line_4(120),
            ui_strings::controls_line_5(120),
        ];
        for (i, line) in wide_lines.iter().enumerate() {
            let col2_char = line.chars().nth(58);
            assert_eq!(
                col2_char,
                Some('['),
                "Line {} does not have Column 2 bracket at index 58: {:?}",
                i + 1,
                line
            );
        }
    }

    #[test]
    fn test_key_handling_shift_numbers() {
        let mut app = OscDiagnosticsApp::new();
        let (out, _) = OutputDevice::new_mock();

        // Test Shift+1 via plain '!'.
        let key_exclamation = KeyPress::Plain {
            key: Key::Character('!'),
        };
        app.handle_key_press(key_exclamation, &out).unwrap();
        assert!(app.event_log.iter().any(|msg| msg.contains("Background")));

        // Test Shift+1 via WithModifiers.
        let mut mask = r3bl_tui::ModifierKeysMask::default();
        mask.shift_key_state = r3bl_tui::KeyState::Pressed;
        let key_shift_1 = KeyPress::WithModifiers {
            key: Key::Character('1'),
            mask,
        };
        app.handle_key_press(key_shift_1, &out).unwrap();
        assert!(app.event_log.iter().any(|msg| msg.contains("Background")));

        // Test Shift+4 via '$'.
        let key_dollar = KeyPress::Plain {
            key: Key::Character('$'),
        };
        app.handle_key_press(key_dollar, &out).unwrap();
        assert!(app.event_log.iter().any(|msg| msg.contains("Highlight")));

        // Test Shift+5 via '%'.
        let key_pct = KeyPress::Plain {
            key: Key::Character('%'),
        };
        app.handle_key_press(key_pct, &out).unwrap();
        assert!(
            app.event_log
                .iter()
                .any(|msg| msg.contains("terminal title"))
        );

        // Test Shift+9 via '('.
        let key_paren = KeyPress::Plain {
            key: Key::Character('('),
        };
        app.handle_key_press(key_paren, &out).unwrap();
        assert!(
            app.event_log
                .iter()
                .any(|msg| msg.contains("Running all OSC diagnostics"))
        );
    }

    #[test]
    fn test_detect_width_returns_valid_value() {
        use super::box_drawing::{MAX_WIDTH, detect_width};
        let w = detect_width();
        assert!(w >= MAX_WIDTH || w > 0);
    }

    #[test]
    fn test_plain_number_and_letter_keys_do_not_trigger_ops() {
        let mut app = OscDiagnosticsApp::new();
        let (out, _) = OutputDevice::new_mock();

        // Plain '1' (as leaked by Crossterm when terminal sends reply).
        let key_plain_1 = KeyPress::Plain {
            key: Key::Character('1'),
        };
        app.handle_key_press(key_plain_1, &out).unwrap();
        assert!(
            !app.event_log
                .iter()
                .any(|msg| msg.contains("Emitted query"))
        );

        // Plain `a` (as leaked by Crossterm in hex color responses like `#0a0a0a`).
        let key_plain_a = KeyPress::Plain {
            key: Key::Character('a'),
        };
        app.handle_key_press(key_plain_a, &out).unwrap();
        assert!(
            !app.event_log
                .iter()
                .any(|msg| msg.contains("Running all OSC diagnostics"))
        );

        // Plain 'c' (as leaked by Crossterm in hex color responses).
        let key_plain_c = KeyPress::Plain {
            key: Key::Character('c'),
        };
        app.handle_key_press(key_plain_c, &out).unwrap();
        assert!(
            !app.event_log
                .iter()
                .any(|msg| msg.contains("Event log cleared"))
        );
    }
}

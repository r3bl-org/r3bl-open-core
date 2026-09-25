// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{OutputDevice, RawModeGuard, TERMINAL_LIB_BACKEND, TerminalLibBackend,
            ansi_output, ok};
use crossterm::{QueueableCommand,
                cursor::{Hide, Show},
                event::{DisableBracketedPaste, DisableMouseCapture,
                        EnableBracketedPaste, EnableMouseCapture,
                        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
                        PushKeyboardEnhancementFlags},
                terminal::{Clear, ClearType, DisableLineWrap, EnableLineWrap,
                           EnterAlternateScreen, LeaveAlternateScreen}};
use miette::IntoDiagnostic;

/// Provides an ergonomic API to explicitly control global terminal states (modes):
/// - Raw mode vs Cooked mode.
/// - Alternate screen vs Main screen.
/// - Visible cursor vs Hidden cursor.
/// - Mouse events on vs off.
/// - Bracketed paste on vs off.
///
/// These modes affect the interaction with:
/// - The terminal emulator (e.g., `/dev/pts/X`), also called pseudo-terminals (PTYs).
/// - Linux kernel virtual console (`/dev/tty`, like `Ctrl+Alt+F[1..4]`), also called
///   physical/virtual kernel TTYs.
pub trait TerminalModeController {
    /// Enables terminal raw mode for direct control over input/output.
    ///
    /// Raw mode disables line buffering and special character processing, allowing the
    /// application to receive keystrokes immediately and handle all terminal control
    /// sequences directly.
    ///
    /// This method returns a [`RawModeGuard`]. When this guard is dropped, it will
    /// automatically disable raw mode and restore normal terminal behavior ([`RAII`]).
    /// This way we don't need a corresponding `exit_raw_mode()`.
    ///
    /// # Errors
    /// Returns an error if the platform's raw mode API fails.
    ///
    /// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
    fn enter_raw_mode(&self) -> miette::Result<RawModeGuard>;

    /// Switches to alternate screen buffer for full-screen applications.
    ///
    /// When enabled, the terminal saves the current screen content and switches to an
    /// alternate buffer. This is used by full-screen applications (vim, less, etc.) to
    /// preserve shell history and avoid cluttering the original screen.
    ///
    /// Remember to call [`TerminalModeController::exit_alternate_screen`] before
    /// returning to normal shell operation.
    ///
    /// Maps to [`CSI`] `?1049h` [`ANSI`] sequence ([`DEC`] Private Mode Set).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn enter_alternate_screen(&self) -> miette::Result<()>;

    /// Exits alternate screen buffer and restores original screen content.
    ///
    /// Restores the screen content that was saved when
    /// [`TerminalModeController::enter_alternate_screen`] was called. Should always be
    /// called before returning to normal shell operation.
    ///
    /// Maps to [`CSI`] `?1049l` [`ANSI`] sequence ([`DEC`] Private Mode Reset).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn exit_alternate_screen(&self) -> miette::Result<()>;

    /// Hide cursor (make it invisible).
    ///
    /// Maps to [`CSI`] `?25l` [`ANSI`] sequence ([`DEC`] Private Mode Reset).
    ///
    /// Useful for animations or rendering where cursor visibility would be distracting.
    /// Remember to call [`TerminalModeController::show_cursor`] before normal operation
    /// resumes.
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn hide_cursor(&self) -> miette::Result<()>;

    /// Show cursor (make it visible).
    ///
    /// Maps to [`CSI`] `?25h` [`ANSI`] sequence ([`DEC`] Private Mode Set).
    ///
    /// Restores cursor visibility after it has been hidden with
    /// [`TerminalModeController::hide_cursor`].
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn show_cursor(&self) -> miette::Result<()>;

    /// Enables mouse event tracking (clicks, movement, scroll).
    ///
    /// When enabled, the terminal reports mouse events to the application. This includes
    /// mouse clicks, movements, and scroll wheel events.
    ///
    /// Remember to call [`TerminalModeController::disable_mouse_tracking`] when tracking
    /// is no longer needed.
    ///
    /// Maps to [`CSI`] `?1000h` [`ANSI`] sequence ([`DEC`] Private Mode Set for mouse
    /// tracking).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn enable_mouse_tracking(&self) -> miette::Result<()>;

    /// Disables mouse event tracking.
    ///
    /// Restores normal mouse behavior where the terminal no longer reports mouse events
    /// to the application. Called to restore normal operation after mouse tracking is no
    /// longer needed following a call to
    /// [`TerminalModeController::enable_mouse_tracking`].
    ///
    /// Maps to [`CSI`] `?1000l` [`ANSI`] sequence ([`DEC`] Private Mode Reset).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn disable_mouse_tracking(&self) -> miette::Result<()>;

    /// Enables bracketed paste mode for distinguishing pasted text.
    ///
    /// When enabled, text pasted from the clipboard is wrapped with special escape
    /// sequences, allowing the application to distinguish pasted content from keyboard
    /// input. This prevents pasted content from being misinterpreted as commands.
    ///
    /// Remember to call [`TerminalModeController::disable_bracketed_paste`] when
    /// clipboard detection is no longer needed.
    ///
    /// Maps to [`CSI`] `?2004h` [`ANSI`] sequence ([`DEC`] Private Mode Set for bracketed
    /// paste).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn enable_bracketed_paste(&self) -> miette::Result<()>;

    /// Disables bracketed paste mode.
    ///
    /// Restores normal paste behavior where the terminal doesn't wrap pasted text with
    /// special escape sequences. Called when clipboard detection is no longer needed
    /// following a call to [`TerminalModeController::enable_bracketed_paste`].
    ///
    /// Maps to [`CSI`] `?2004l` [`ANSI`] sequence ([`DEC`] Private Mode Reset).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn disable_bracketed_paste(&self) -> miette::Result<()>;

    /// Enables progressive keyboard enhancement ([`Kitty`] keyboard protocol).
    ///
    /// When enabled, the terminal uses `CSI u` escape sequences to encode keys
    /// unambiguously, allowing modifiers such as `Alt+[`, `Shift+Enter`, `Ctrl+Tab`,
    /// and `Alt+Escape` to be distinguished without ambiguity or collision.
    ///
    /// Remember to call [`TerminalModeController::disable_keyboard_enhancement`] when
    /// keyboard enhancement is no longer needed.
    ///
    /// Maps to [`CSI`] `>1u` [`ANSI`] sequence (Push flags: 1 =
    /// `DISAMBIGUATE_ESCAPE_CODES`).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
    fn enable_keyboard_enhancement(&self) -> miette::Result<()>;

    /// Disables progressive keyboard enhancement ([`Kitty`] keyboard protocol).
    ///
    /// Restores standard keyboard reporting. Called when keyboard enhancement is no
    /// longer needed following a call to
    /// [`TerminalModeController::enable_keyboard_enhancement`].
    ///
    /// Maps to [`CSI`] `<1u` [`ANSI`] sequence (Pop 1 level of flags).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
    fn disable_keyboard_enhancement(&self) -> miette::Result<()>;

    /// Enables line wrapping at the right margin.
    ///
    /// Maps to [`CSI`] `?7h` [`ANSI`] sequence ([`DEC`] Private Mode Set for autowrap).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn enable_line_wrap(&self) -> miette::Result<()>;

    /// Disables line wrapping at the right margin.
    ///
    /// Maps to [`CSI`] `?7l` [`ANSI`] sequence ([`DEC`] Private Mode Reset for autowrap).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    fn disable_line_wrap(&self) -> miette::Result<()>;

    /// Clears the entire terminal screen.
    ///
    /// Maps to [`CSI`] `2J` [`ANSI`] sequence (Erase in Display: entire display).
    ///
    /// # Errors
    /// Returns an error if the underlying I/O fails.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    fn clear_screen(&self) -> miette::Result<()>;
}

impl TerminalModeController for OutputDevice {
    fn enter_raw_mode(&self) -> miette::Result<RawModeGuard> { RawModeGuard::new() }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn enter_alternate_screen(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(EnterAlternateScreen).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::enter_alternate_screen();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Teardown method: Poison-safe to prevent [Double Panic Abort] during drop.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn exit_alternate_screen(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(LeaveAlternateScreen).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::exit_alternate_screen();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn hide_cursor(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(Hide).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::cursor_visibility::hide_cursor();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Teardown method: Poison-safe to prevent [Double Panic Abort] during drop.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn show_cursor(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(Show).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::cursor_visibility::show_cursor();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn enable_mouse_tracking(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(EnableMouseCapture).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::enable_mouse_tracking();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Teardown method: Poison-safe to prevent [Double Panic Abort] during drop.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn disable_mouse_tracking(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(DisableMouseCapture).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::disable_mouse_tracking();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn enable_bracketed_paste(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(EnableBracketedPaste).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::enable_bracketed_paste();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Teardown method: Poison-safe to prevent [Double Panic Abort] during drop.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn disable_bracketed_paste(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(DisableBracketedPaste).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::disable_bracketed_paste();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn enable_keyboard_enhancement(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    let result = writer
                        .queue(PushKeyboardEnhancementFlags(
                            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
                        ))
                        .and_then(std::io::Write::flush);
                    if let Err(e) = result {
                        // On platforms or terminals that do not support progressive
                        // keyboard enhancement (such as the
                        // legacy Windows console API), gracefully degrade.
                        if e.kind() == std::io::ErrorKind::Unsupported
                            || e.to_string().contains("legacy Windows API")
                        {
                            return ok!();
                        }
                        return Err(e).into_diagnostic();
                    }
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::enable_keyboard_enhancement();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Teardown method: Poison-safe to prevent [Double Panic Abort] during drop.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn disable_keyboard_enhancement(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    let result = writer
                        .queue(PopKeyboardEnhancementFlags)
                        .and_then(std::io::Write::flush);
                    if let Err(e) = result {
                        // On platforms or terminals that do not support progressive
                        // keyboard enhancement (such as the
                        // legacy Windows console API), gracefully degrade.
                        if e.kind() == std::io::ErrorKind::Unsupported
                            || e.to_string().contains("legacy Windows API")
                        {
                            return ok!();
                        }
                        return Err(e).into_diagnostic();
                    }
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi =
                        ansi_output::terminal_modes::disable_keyboard_enhancement();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn enable_line_wrap(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(EnableLineWrap).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::enable_line_wrap();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn disable_line_wrap(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(DisableLineWrap).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::terminal_modes::disable_line_wrap();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }

    /// Setup method: Fail-fast if the terminal is poisoned.
    fn clear_screen(&self) -> miette::Result<()> {
        self.write(|writer| {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer.queue(Clear(ClearType::All)).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = ansi_output::screen_clearing::clear_screen();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words O_NONBLOCK

use crate::{RawModeGuard, SafeRawTerminal, SendRawTerminal,
            StdMutex, TERMINAL_LIB_BACKEND, TerminalLibBackend, col, ok, row};
use crossterm::{QueueableCommand,
                cursor::{Hide, Show},
                event::{DisableBracketedPaste, DisableMouseCapture,
                        EnableBracketedPaste, EnableMouseCapture},
                terminal::{EnterAlternateScreen, LeaveAlternateScreen}};
use miette::IntoDiagnostic;
use std::{io::{Stdout, Write, stdout},
          sync::Arc};

pub type LockedOutputDevice<'a> = &'a mut dyn Write;

/// Whether to execute paint operations against the real terminal or in mock mode
/// for testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    /// Send execution commands to the real, active physical terminal backend.
    Real,
    /// Record execution commands in memory without outputting to the screen (used
    /// for testing).
    Mock,
}

/// This struct represents an output device that can be used to write to the terminal.
/// - It is safe to clone.
/// - To write to it, use the [`Self::write()`] method.
/// - It utilizes [`StdMutex`]. See its [architectural rationale] for details.
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for details.
///
/// [`StdMutex`]: crate::StdMutex
/// [architectural rationale]:
///     crate::StdMutex#architectural-rationale-for-paniconspecificlocknesting-specific
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct OutputDevice {
    pub resource: SafeRawTerminal,
    pub paint_mode: PaintMode,
}

impl Default for OutputDevice {
    fn default() -> Self { Self::new_stdout() }
}

impl OutputDevice {
    /// Creates a new output device wrapping [`stdout`].
    ///
    /// The standard output is wrapped in [`FullBufferWaitingStdout`] to safely handle
    /// [`WouldBlock`] errors that occur when [`stdin`] is set to non-blocking mode.
    ///
    /// [`stdin`]: std::io::stdin
    /// [`stdout`]: std::io::stdout
    /// [`WouldBlock`]: std::io::ErrorKind::WouldBlock
    #[must_use]
    pub fn new_stdout() -> Self {
        Self {
            resource: Arc::new(StdMutex::new(FullBufferWaitingStdout(stdout()))),
            paint_mode: PaintMode::Real,
        }
    }
}

/// A wrapper around [`stdout`] that politely waits and retries when the OS terminal
/// buffer is full.
///
/// # The Mental Model
///
/// 1. **The Problem:** We made [`stdin`] non-blocking so our event loop doesn't freeze.
///    On Linux, [`stdout`] uses the exact same underlying file description, so [`stdout`]
///    accidentally became non-blocking too.
/// 2. **The Symptom:** If we try to write a ton of text to the terminal all at once (like
///    a huge UI render or a massive [`cat`] command), the operating system's terminal
///    buffer gets full. Normally, the OS would pause our thread and wait for space. But
///    because it's non-blocking now, the OS panics and throws a "buffer full, try again
///    later" error ([`WouldBlock`]).
/// 3. **The Fix:** We built this wrapper to catch that "try again later" error. Instead
///    of crashing, it literally tells the thread to wait ([`std::thread::yield_now()`])
///    and loops to try writing the text again. We are essentially faking normal terminal
///    behavior so the rest of the app doesn't have to know that [`stdout`] is acting
///    weird under the hood.
///
/// # Blocking vs. Busy-Waiting vs. Yielding
///
/// Why use [`std::thread::yield_now()`] instead of just looping (busy-waiting) or letting
/// the OS handle it (blocking)?
///
/// - **Blocking (Going to Sleep):** This is the default behavior of [`stdout`]. When the
///   buffer is full, the OS takes the thread off the CPU and puts it to sleep until space
///   frees up. CPU usage is 0%. We lost this behavior when [`stdin`] became non-blocking.
/// - **Busy-Waiting (The Bad Way):** If we caught [`WouldBlock`] and just looped (`loop {
///   match ... }`) without yielding, our thread would stay on the CPU asking the OS "Is
///   it ready?" millions of times a second. This burns 100% of a CPU core doing nothing
///   useful.
/// - **Yielding (Polite Waiting):** By calling [`yield_now()`], the thread immediately
///   pauses its execution and gives up its timeslice to the OS scheduler. The thread is
///   not asleep, but it goes to the back of the line. In the fraction of a second while
///   other programs run, the terminal emulator clears the OS buffer. When our thread gets
///   its turn again, the buffer has space. This is highly CPU-efficient active polling.
///
/// # Cross-References
/// - Root cause: [Why We Need Non-Blocking Read]
/// - Where it originates: In  [`MioPollWorker`], see [`original_stdin_flags`] field and
///   [`drop()`] method.
///
/// [`cat`]: https://en.wikipedia.org/wiki/Cat_(Unix)
/// [`drop()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker#method.drop
/// [`MioPollWorker`]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
/// [`original_stdin_flags`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker::original_stdin_flags
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
/// [`WouldBlock`]: std::io::ErrorKind::WouldBlock
/// [`yield_now()`]: std::thread::yield_now
/// [Why We Need Non-Blocking Read]:
///     crate::tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::handler_stdin::consume_stdin_input_with_sender#why-we-need-non-blocking-read
#[derive(Debug)]
pub struct FullBufferWaitingStdout(pub Stdout);

impl Write for FullBufferWaitingStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            match self.0.write(buf) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::yield_now();
                }
                other => return other,
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        loop {
            match self.0.flush() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::yield_now();
                }
                other => return other,
            }
        }
    }
}

/// Mimics the public API of [`ScopedMutex`] due to the requirement of passing in closures
/// and no longer providing direct access to the underlying mutex.
///
/// [`ScopedMutex`]: crate::ScopedMutex
impl OutputDevice {
    /// Provides read-only access to the output device via a closure.
    ///
    /// # Panics
    ///
    /// - Panics if the internal mutex is poisoned (Fail-fast).
    /// - Panics if a recursive lock is detected on the same instance.
    pub fn read<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&dyn Write) -> R,
    {
        self.resource.read(|writer| fun(writer))
    }

    /// Provides read-write access to the output device via a closure.
    ///
    /// # Panics
    ///
    /// - Panics if the internal mutex is poisoned (Fail-fast).
    /// - Panics if a recursive lock is detected on the same instance.
    pub fn write<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut SendRawTerminal) -> R,
    {
        self.resource.write(|writer| fun(writer))
    }

    /// Provides raw access to the internal mutex, returning the
    /// [`std::sync::LockResult`].
    ///
    /// This is a **poison-safe** alternative specifically designed for **cleanup paths**.
    ///
    /// This method **bypasses** the shared ledger to ensure that terminal restoration can
    /// proceed even in complex failure states.
    pub fn lock_raw<'this, F, R>(&'this self, fun: F) -> R
    where
        F: FnOnce(
            std::sync::LockResult<std::sync::MutexGuard<'this, SendRawTerminal>>,
        ) -> R,
    {
        self.resource.lock_raw(fun)
    }

    /// Provides raw, poison-safe access to the internal mutex. It automatically
    /// recovers from potential poison errors by calling `into_inner()` on the
    /// poison error, and passes a mutable reference to the protected data to
    /// the closure.
    ///
    /// Like [`Self::lock_raw()`], this method **bypasses** recursion detection
    /// to ensure that cleanup or terminal restoration can proceed even in complex
    /// failure states or panic/drop paths.
    pub fn lock_raw_poison_safe<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(&mut SendRawTerminal) -> R,
    {
        self.resource.lock_raw_poison_safe(fun)
    }

    /// Flushes the internal writer (e.g. stdout) ensuring all buffered data is written.
    ///
    /// # Errors
    /// Returns an error if the underlying I/O write fails.
    pub fn flush(&self) -> miette::Result<()> {
        self.lock_raw_poison_safe(|writer| writer.flush().into_diagnostic())
    }

    /// Sets up the full-screen TUI environment.
    ///
    /// This includes enabling bracketed paste, mouse tracking, entering the alternate
    /// screen, hiding the cursor, and clearing the screen.
    ///
    /// # Errors
    /// Returns an error if any terminal mode cannot be set or I/O fails.
    pub fn setup_full_screen_tui(&self) -> miette::Result<FullScreenTuiModeGuard> {
        self.enable_bracketed_paste()?;
        self.enable_mouse_tracking()?;
        self.enter_alternate_screen()?;
        self.hide_cursor()?;

        self.write(|writer| -> miette::Result<()> {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    writer
                        .queue(crossterm::cursor::MoveTo(0, 0))
                        .into_diagnostic()?;
                    writer
                        .queue(crossterm::terminal::Clear(
                            crossterm::terminal::ClearType::All,
                        ))
                        .into_diagnostic()?;
                }
                TerminalLibBackend::DirectToAnsi => {
                    let ansi = crate::ansi_output::cursor_movement::cursor_position(row(0), col(0));
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;

                    let ansi2 = crate::ansi_output::screen_clearing::clear_screen();
                    writer.write_all(ansi2.as_bytes()).into_diagnostic()?;
                }
            }
            ok!()
        })?;

        if matches!(self.paint_mode, PaintMode::Real) {
            self.flush()?;
        }

        Ok(FullScreenTuiModeGuard {
            output_device: self.clone(),
        })
    }

    /// Tears down the full-screen TUI environment.
    /// This restores the cursor, exits the alternate screen, and disables mouse/paste
    /// tracking.
    ///
    /// # Errors
    /// Returns an error if any terminal mode cannot be reset or I/O fails.
    pub fn teardown_full_screen_tui(&self) -> miette::Result<()> {
        self.disable_bracketed_paste()?;
        self.disable_mouse_tracking()?;
        self.exit_alternate_screen()?;
        self.show_cursor()?;

        if matches!(self.paint_mode, PaintMode::Real) {
            self.flush()?;
        }
        ok!()
    }
}

/// An [`RAII`] guard that tears down the TUI environment when dropped.
///
/// This is returned by [`OutputDevice::setup_full_screen_tui()`] and ensures that the
/// terminal is properly restored (cursor shown, alternate screen exited, mouse and
/// bracketed paste tracking disabled) even if a panic occurs or the future returns early,
/// avoiding a [Double Panic Abort].
///
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [Double Panic Abort]: crate#the-double-panic-abort-risk
#[must_use = "The full screen TUI mode guard must be held as long as the TUI is active."]
#[allow(missing_debug_implementations)]
pub struct FullScreenTuiModeGuard {
    output_device: OutputDevice,
}

impl Drop for FullScreenTuiModeGuard {
    /// We prioritize Resilience over Integrity here to prevent a [Double Panic Abort].
    /// The teardown methods underneath are poison-safe.
    ///
    /// [Double Panic Abort]: crate#the-double-panic-abort-risk
    fn drop(&mut self) { drop(self.output_device.teardown_full_screen_tui()); }
}

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
                    let ansi = crate::ansi_output::terminal_modes::enter_alternate_screen();
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
                    let ansi = crate::ansi_output::terminal_modes::exit_alternate_screen();
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
                    let ansi = crate::ansi_output::cursor_visibility::hide_cursor();
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
                    let ansi = crate::ansi_output::cursor_visibility::show_cursor();
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
                    let ansi = crate::ansi_output::terminal_modes::enable_mouse_tracking();
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
                    let ansi = crate::ansi_output::terminal_modes::disable_mouse_tracking();
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
                    let ansi = crate::ansi_output::terminal_modes::enable_bracketed_paste();
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
                    let ansi = crate::ansi_output::terminal_modes::disable_bracketed_paste();
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;
                    writer.flush().into_diagnostic()?;
                }
            }
            ok!()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_output_device() {
        let output_device = OutputDevice::new_stdout();
        output_device.write(|writer| {
            // We don't care about the result of this operation.
            drop(writer.write_all(b"Hello, world!\n"));
        });

        assert_eq!(output_device.paint_mode, PaintMode::Real);
    }

    #[test]
    fn test_stdout_output_device_is_not_mock() {
        let device = OutputDevice::new_stdout();
        assert_eq!(device.paint_mode, PaintMode::Real);
    }

    #[test]
    fn test_output_device_poison_resilience() {
        let resource: SafeRawTerminal = Arc::new(StdMutex::new(Vec::new()));
        let device = OutputDevice {
            resource: Arc::clone(&resource),
            paint_mode: PaintMode::Mock,
        };

        // 1. Poison the mutex.
        let _unused = std::thread::spawn(move || {
            resource.write(|_| {
                panic!("Intentional panic to poison OutputDevice resource");
            });
        })
        .join();

        // 2. Verify it is poisoned.
        let is_poisoned = device.resource.lock_raw(|result| result.is_err());
        assert!(is_poisoned);

        // 3. Verify write() panics (Fail-fast).
        let result = std::panic::catch_unwind(|| {
            device.write(|writer| {
                drop(writer.write_all(b"should panic"));
            });
        });
        assert!(result.is_err());

        // 4. Verify lock_raw() does NOT panic and returns the dirty state.
        device.lock_raw(|result| {
            let mut guard = match result {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            drop(guard.write_all(b"still works"));
        });

        // 5. Verify lock_raw_poison_safe() does NOT panic and returns the dirty state.
        device.lock_raw_poison_safe(|writer| {
            drop(writer.write_all(b" still works"));
        });

        // 6. Verify data was written to the dirty state.
        device.lock_raw_poison_safe(|writer| {
            // Can't easily check content of dyn Write, but we can verify it doesn't
            // panic.
            drop(writer.flush());
        });
    }
}

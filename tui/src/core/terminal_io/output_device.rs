// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{BackpressureStdout, FullScreenTuiModeGuard, SafeRawTerminal,
            SendRawTerminal, StdMutex, TERMINAL_LIB_BACKEND, TerminalLibBackend,
            TerminalModeController, ansi_output, ok, vp_col, vp_row};
use crossterm::QueueableCommand;
use miette::IntoDiagnostic;
use std::{io::{Write, stdout},
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
    /// The standard output is wrapped in [`BackpressureStdout`] to safely handle
    /// [`WouldBlock`] errors that occur when [`stdin`] is set to non-blocking mode.
    ///
    /// [`BackpressureStdout`]: BackpressureStdout
    /// [`stdin`]: std::io::stdin
    /// [`stdout`]: std::io::stdout
    /// [`WouldBlock`]: std::io::ErrorKind::WouldBlock
    #[must_use]
    pub fn new_stdout() -> Self {
        Self {
            resource: Arc::new(StdMutex::new(BackpressureStdout(stdout()))),
            paint_mode: PaintMode::Real,
        }
    }
}

/// Mimics the public API of [`ScopedMutex`] due to the requirement of passing in
/// closures and no longer providing direct access to the underlying mutex.
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
    /// This is a **poison-safe** alternative specifically designed for **cleanup
    /// paths**.
    ///
    /// This method **bypasses** the shared ledger to ensure that terminal restoration
    /// can proceed even in complex failure states.
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

    /// Flushes the internal writer (e.g. stdout) ensuring all buffered data is
    /// written.
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
                    let ansi = ansi_output::cursor_movement::cursor_position(
                        vp_row(0).into(),
                        vp_col(0).into(),
                    );
                    writer.write_all(ansi.as_bytes()).into_diagnostic()?;

                    let ansi2 = ansi_output::screen_clearing::clear_screen();
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

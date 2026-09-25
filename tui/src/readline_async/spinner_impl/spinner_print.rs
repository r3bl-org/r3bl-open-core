// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CommonResult, LockedOutputDevice, OutputDevice, SharedWriter, SpinnerStyle,
            TERMINAL_LIB_BACKEND, TermCol, TermRowDelta, TerminalLibBackend,
            ansi_output, ok, queue_commands, queue_commands_no_lock};
use crossterm::{cursor::{Hide, MoveToColumn, MoveToNextLine, MoveToPreviousLine, Show},
                style::Print,
                terminal::{Clear, ClearType}};
use miette::IntoDiagnostic;

// Allocate specified number of lines in the terminal (ahead of the current cursor
// position) for the spinner.
#[allow(clippy::needless_pass_by_value)]
fn clear_lines_for_spinner(
    output_device: OutputDevice,
    num_lines_to_clear: u16,
) -> CommonResult {
    if num_lines_to_clear == 0 {
        return ok!();
    }

    output_device.write(|writer| {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                queue_commands_no_lock!(writer, Hide, Clear(ClearType::CurrentLine));

                // Clear subsequent lines by moving down and clearing. This loop runs
                // (num_lines_to_clear - 1) times.
                for _ in 1..num_lines_to_clear {
                    queue_commands_no_lock!(
                        writer,
                        MoveToNextLine(1),
                        Clear(ClearType::CurrentLine)
                    );
                }

                // Move cursor back to the start of the first line that was cleared. If
                // num_lines_to_clear is 1, no downward movement occurred, so no upward
                // movement is needed. Otherwise, move up by
                // (num_lines_to_clear - 1) lines.
                if num_lines_to_clear > 1 {
                    queue_commands_no_lock!(
                        writer,
                        MoveToPreviousLine(num_lines_to_clear - 1)
                    );
                }
                queue_commands_no_lock!(writer, MoveToColumn(0));
            }
            TerminalLibBackend::DirectToAnsi => {
                let mut buf = String::new();
                buf.push_str(ansi_output::cursor_visibility::hide_cursor());
                buf.push_str(ansi_output::screen_clearing::clear_current_line());

                for _ in 1..num_lines_to_clear {
                    buf.push_str(&ansi_output::cursor_movement::cursor_next_line(
                        TermRowDelta::ONE,
                    ));
                    buf.push_str(ansi_output::screen_clearing::clear_current_line());
                }

                if let Some(delta) = num_lines_to_clear
                    .checked_sub(1)
                    .and_then(TermRowDelta::new)
                {
                    buf.push_str(&ansi_output::cursor_movement::cursor_previous_line(
                        delta,
                    ));
                }
                buf.push_str(&ansi_output::cursor_movement::cursor_to_column(
                    TermCol::ONE,
                ));
                writer.write_all(buf.as_bytes()).into_diagnostic()?;
            }
        }

        writer.flush().into_diagnostic()?;

        Ok::<(), miette::Report>(())
    })?;

    ok!()
}

/// This function only does something `Spinner` is used by itself, and not within a
/// [`crate::ReadlineAsyncContext`], ie, when `maybe_shared_writer` is `None`.
///
/// # Errors
///
/// Returns an error if clearing lines fails due to I/O errors.
pub fn print_start_if_standalone(
    output_device: OutputDevice,
    maybe_shared_writer: &Option<SharedWriter>,
) -> CommonResult {
    if maybe_shared_writer.is_none() {
        clear_lines_for_spinner(output_device, 2)?;
    }
    ok!()
}

/// This gets called repeatedly to print the spinner with the intedeterminate progress
/// message.
///
/// # Errors
///
/// Returns an error if printing or flushing the output fails.
#[allow(clippy::needless_pass_by_value)]
pub fn print_tick_interval_msg(
    _style: &SpinnerStyle,
    output: &str,
    output_device: OutputDevice,
) -> CommonResult {
    // Print the output. And make sure to terminate w/ a newline, so that the
    // output is printed for ReadlineAsyncContext.
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            queue_commands!(
                output_device,
                // Move the cursor to the beginning of the current line.
                MoveToColumn(0),
                // Clear everything from the cursor position to the end of the screen.
                Clear(ClearType::CurrentLine),
                // Print the spinner output. The \n is important for ReadlineAsyncContext
                // to pick it up.
                Print(format!("{output}\n")), /* \n is needed to ReadlineAsyncContext */
                // Move the cursor up one line, to where the spinner message was just
                // printed.
                MoveToPreviousLine(1),
                // Move the cursor to the beginning of that line again, ready for the
                // next tick.
                MoveToColumn(0)
            );
        }
        TerminalLibBackend::DirectToAnsi => {
            output_device.write(|writer| -> miette::Result<()> {
                let mut buf = String::new();
                buf.push_str(&ansi_output::cursor_movement::cursor_to_column(
                    TermCol::ONE,
                ));
                buf.push_str(ansi_output::screen_clearing::clear_current_line());
                buf.push_str(output);
                buf.push('\n');
                buf.push_str(&ansi_output::cursor_movement::cursor_previous_line(
                    TermRowDelta::ONE,
                ));
                buf.push_str(&ansi_output::cursor_movement::cursor_to_column(
                    TermCol::ONE,
                ));
                writer.write_all(buf.as_bytes()).into_diagnostic()?;
                ok!()
            })?;
        }
    }

    output_device.write(|writer| {
        writer.flush().into_diagnostic()?;
        Ok::<(), miette::Report>(())
    })?;

    ok!()
}

/// This gets called when the spinner is done, to print the final message.
///
/// # Errors
///
/// Returns an error if printing or flushing the output fails.
#[allow(clippy::needless_pass_by_value)]
pub fn print_tick_final_msg(
    _style: &SpinnerStyle,
    output: &str,
    output_device: OutputDevice,
    maybe_shared_writer: &Option<SharedWriter>,
) -> CommonResult {
    output_device.write(|writer| {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                queue_commands_no_lock!(
                    writer,
                    // Ensure cursor is at the beginning of the spinner line. This is
                    // usually true if called after
                    // print_tick_interval_msg, but it's safer
                    // to be explicit.
                    MoveToColumn(0),
                    // Clear the current line (where the spinner interval message was).
                    Clear(ClearType::CurrentLine),
                    // Print the final output on this cleared line. The \n will move the
                    // cursor to the beginning of the next line.
                    Print(format!("{output}\n")),
                    // Now, from the current cursor position (start of the line after the
                    // final message), clear downwards. This is to
                    // clean up any other concurrent output
                    // that might have appeared below the spinner.
                    Clear(ClearType::FromCursorDown)
                );
            }
            TerminalLibBackend::DirectToAnsi => {
                let mut buf = String::new();
                buf.push_str(&ansi_output::cursor_movement::cursor_to_column(
                    TermCol::ONE,
                ));
                buf.push_str(ansi_output::screen_clearing::clear_current_line());
                buf.push_str(output);
                buf.push('\n');
                buf.push_str(ansi_output::screen_clearing::clear_to_end_of_screen());
                writer.write_all(buf.as_bytes()).into_diagnostic()?;
            }
        }

        // Only run this if the spinner is not running in a `ReadlineAsyncContext`
        // context.
        if maybe_shared_writer.is_none() {
            // We don't care about the result of this operation.
            drop(print_end_if_standalone(writer));
        }

        writer.flush().into_diagnostic()?;

        Ok::<(), miette::Report>(())
    })?;

    ok!()
}

/// This function only does something `Spinner` is used by itself, and not within a
/// [`crate::ReadlineAsyncContext`], ie, when `maybe_shared_writer` is `None`.
///
/// This receives the `writer` that is already locked by the caller, so that there is no
/// "out of sequence" issues with the output that is printed, that might result from
/// having to wait to acquire a lock.
#[allow(clippy::missing_errors_doc)]
fn print_end_if_standalone(writer: LockedOutputDevice<'_>) -> CommonResult {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            queue_commands_no_lock!(
                writer,
                // Move the cursor to the beginning of the current line.
                MoveToColumn(0),
                // Clear everything from the cursor position to the end of the screen.
                Clear(ClearType::CurrentLine),
                // Show the cursor again.
                Show
            );
        }
        TerminalLibBackend::DirectToAnsi => {
            let mut buf = String::new();
            buf.push_str(&ansi_output::cursor_movement::cursor_to_column(
                TermCol::ONE,
            ));
            buf.push_str(ansi_output::screen_clearing::clear_current_line());
            buf.push_str(ansi_output::cursor_visibility::show_cursor());
            writer.write_all(buf.as_bytes()).into_diagnostic()?;
        }
    }

    ok!()
}

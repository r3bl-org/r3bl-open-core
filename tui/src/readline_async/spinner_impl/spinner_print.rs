/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use crossterm::{cursor::{Hide, MoveToColumn, MoveToNextLine, MoveToPreviousLine, Show},
                style::Print,
                terminal::{Clear, ClearType}};
use miette::IntoDiagnostic as _;

use crate::{lock_output_device_as_mut,
            ok,
            queue_commands,
            queue_commands_no_lock,
            CommonResult,
            LockedOutputDevice,
            OutputDevice,
            SharedWriter,
            SpinnerStyle};

// Allocate specified number of lines in the terminal (ahead of the current cursor
// position) for the spinner.
fn clear_lines_for_spinner(
    output_device: OutputDevice,
    num_lines_to_clear: u16,
) -> CommonResult<()> {
    if num_lines_to_clear == 0 {
        return ok!();
    }

    let writer = lock_output_device_as_mut!(output_device);

    queue_commands_no_lock!(writer, Hide, Clear(ClearType::CurrentLine));

    // Clear subsequent lines by moving down and clearing. This loop runs
    // (num_lines_to_clear - 1) times.
    for _ in 1..num_lines_to_clear {
        queue_commands_no_lock!(writer, MoveToNextLine(1), Clear(ClearType::CurrentLine));
    }

    // Move cursor back to the start of the first line that was cleared. If
    // num_lines_to_clear is 1, no downward movement occurred, so no upward movement
    // is needed. Otherwise, move up by (num_lines_to_clear - 1) lines.
    if num_lines_to_clear > 1 {
        queue_commands_no_lock!(writer, MoveToPreviousLine(num_lines_to_clear - 1));
    }
    queue_commands_no_lock!(writer, MoveToColumn(0));

    writer.flush().into_diagnostic()?;

    ok!()
}

/// This function only does something `Spinner` is used by itself, and not within a
/// [`crate::ReadlineAsyncContext`], ie, when `maybe_shared_writer` is `None`.
pub fn print_start_if_standalone(
    output_device: OutputDevice,
    maybe_shared_writer: Option<SharedWriter>,
) -> CommonResult<()> {
    if maybe_shared_writer.is_none() {
        clear_lines_for_spinner(output_device, 2)?;
    }
    ok!()
}

/// This gets called repeatedly to print the spinner with the intedeterminate progress
/// message.
pub fn print_tick_interval_msg(
    _style: &SpinnerStyle,
    output: &str,
    output_device: OutputDevice,
) -> CommonResult<()> {
    // Print the output. And make sure to terminate w/ a newline, so that the
    // output is printed for ReadlineAsync.
    queue_commands!(
        output_device,
        // Move the cursor to the beginning of the current line.
        MoveToColumn(0),
        // Clear everything from the cursor position to the end of the screen.
        Clear(ClearType::CurrentLine),
        // Print the spinner output. The \n is important for ReadlineAsync to pick it
        // up.
        Print(format!("{output}\n")), /* \n is needed to ReadlineAsync */
        // Move the cursor up one line, to where the spinner message was just printed.
        MoveToPreviousLine(1),
        // Move the cursor to the beginning of that line again, ready for the next
        // tick.
        MoveToColumn(0)
    );

    lock_output_device_as_mut!(output_device)
        .flush()
        .into_diagnostic()?;

    ok!()
}

/// This gets called when the spinner is done, to print the final message.
pub fn print_tick_final_msg(
    _style: &SpinnerStyle,
    output: &str,
    output_device: OutputDevice,
    maybe_shared_writer: Option<SharedWriter>,
) -> CommonResult<()> {
    let writer = lock_output_device_as_mut!(output_device);

    queue_commands_no_lock!(
        writer,
        // Ensure cursor is at the beginning of the spinner line. This is usually true
        // if called after print_tick_interval_msg, but it's safer to be explicit.
        MoveToColumn(0),
        // Clear the current line (where the spinner interval message was).
        Clear(ClearType::CurrentLine),
        // Print the final output on this cleared line. The \n will move the cursor to
        // the beginning of the next line.
        Print(format!("{output}\n")),
        // Now, from the current cursor position (start of the line after the final
        // message), clear downwards. This is to clean up any other concurrent output
        // that might have appeared below the spinner.
        Clear(ClearType::FromCursorDown)
    );

    // Only run this if the spinner is not running in a `ReadlineAsync` context.
    if maybe_shared_writer.is_none() {
        // We don't care about the result of this operation.
        print_end_if_standalone(writer).ok();
    }

    writer.flush().into_diagnostic()?;

    ok!()
}

/// This function only does something `Spinner` is used by itself, and not within a
/// [`crate::ReadlineAsyncContext`], ie, when `maybe_shared_writer` is `None`.
///
/// This receives the `writer` that is already locked by the caller, so that there is no
/// "out of sequence" issues with the output that is printed, that might result from
/// having to wait to acquire a lock.
fn print_end_if_standalone(writer: LockedOutputDevice<'_>) -> CommonResult<()> {
    queue_commands_no_lock!(
        writer,
        // Move the cursor to the beginning of the current line.
        MoveToColumn(0),
        // Clear everything from the cursor position to the end of the screen.
        Clear(ClearType::CurrentLine),
        // Show the cursor again.
        Show
    );

    ok!()
}

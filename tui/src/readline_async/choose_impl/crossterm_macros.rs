/*
 *   Copyright (c) 2025 R3BL LLC
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

/// This is a macro to queue commands to the output device. It locks the output device
/// before queuing the commands, and unlocks it after. This is good for one and done
/// commands that you want to queue and execute in one go. If you have complex
/// interactions with the output device, you should use [queue_commands_no_lock!] instead,
/// when you have to explicitly lock the output device and hold the lock for the span of
/// operations that you wish to perform; this avoids undefined behavior (in terms of
/// output order).
#[macro_export]
macro_rules! queue_commands {
    ($output_device:expr $(, $command:expr)* $(,)?) => {{
        use miette::IntoDiagnostic as _;
        $(
            ::crossterm::QueueableCommand::queue(
                $crate::lock_output_device_as_mut!($output_device),
                $command
            ).into_diagnostic()?;
        )*
    }}
}

/// This is similar to [queue_commands!], but it does not lock the output device. The use
/// case for this macro is when you have an already locked output device and you want to
/// queue commands without locking it again. This is important if you don't want to get
/// into issues with output generated in the wrong order, due to lock contention leading
/// to undefined behavior (in terms of output order).
#[macro_export]
macro_rules! queue_commands_no_lock {
    ($writer:expr $(, $command:expr)* $(,)?) => {{
        use miette::IntoDiagnostic as _;
        $(
            ::crossterm::QueueableCommand::queue(
                $writer,
                $command
            ).into_diagnostic()?;
        )*
    }}
}

/// This is a macro to execute commands to the output device immediately. It locks the output
/// device before executing the commands, and unlocks it after. This is good for one and
/// done commands that you want to execute in one go. If you have complex interactions with
/// the output device, you should use [execute_commands_no_lock!] instead, when you have to
/// explicitly lock the output device and hold the lock for the span of operations that you
/// wish to perform; this avoids undefined behavior (in terms of output order).
#[macro_export]
macro_rules! execute_commands {
    ($output_device:expr $(, $command:expr)* $(,)?) => {{
        use miette::IntoDiagnostic as _;
        $(
            ::crossterm::QueueableCommand::queue(
                $crate::lock_output_device_as_mut!($output_device),
                $command
            ).into_diagnostic()?;
        )*
        $crate::lock_output_device_as_mut!($output_device).flush().into_diagnostic()?;
    }}
}

/// This is similar to [execute_commands!], but it does not lock the output device. The use
/// case for this macro is when you have an already locked output device and you want to
/// execute commands without locking it again. This is important if you don't want to get
/// into issues with output generated in the wrong order, due to lock contention leading
/// to undefined behavior (in terms of output order).
#[macro_export]
macro_rules! execute_commands_no_lock {
    ($writer:expr $(, $command:expr)* $(,)?) => {{
        use miette::IntoDiagnostic as _;
        $(
            ::crossterm::QueueableCommand::queue(
                $writer,
                $command
            ).into_diagnostic()?;
        )*
        $writer.flush().into_diagnostic()?;
    }}
}

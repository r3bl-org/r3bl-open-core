// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// This is a macro to queue commands to the output device. It locks the output device
/// before queuing the commands, and unlocks it after. This is good for one and done
/// commands that you want to queue and execute in one go. If you have complex
/// interactions with the output device, you should use [`crate::queue_commands_no_lock`!]
/// instead, when you have to explicitly lock the output device and hold the lock for the
/// span of operations that you wish to perform; this avoids undefined behavior (in terms
/// of output order).
#[macro_export]
macro_rules! queue_commands {
    ($output_device:expr $(, $command:expr)* $(,)?) => {{
        $output_device.write(|writer| {
            $(
                ::crossterm::QueueableCommand::queue(
                    writer,
                    $command
                )?;
            )*
            Ok::<(), ::std::io::Error>(())
        })?
    }}
}

/// This is similar to [`queue_commands`!], but it does not lock the output device. The
/// use case for this macro is when you have an already locked output device and you want
/// to queue commands without locking it again. This is important if you don't want to get
/// into issues with output generated in the wrong order, due to lock contention leading
/// to undefined behavior (in terms of output order).
#[macro_export]
macro_rules! queue_commands_no_lock {
    ($writer:expr $(, $command:expr)* $(,)?) => {{
        $(
            ::crossterm::QueueableCommand::queue(
                $writer,
                $command
            )?;
        )*
    }}
}

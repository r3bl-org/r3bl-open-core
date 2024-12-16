/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::io::{self, Write};

use crate::ok;

pub type Text = Vec<u8>;

/// Cloneable object that implements [`Write`] and allows for sending data to the terminal
/// without messing up its associated `Readline` instance (in the `r3bl_terminal_async`
/// crate).
///
/// # Create a new instance by creating a `Readline` instance
///
/// - A [`crate::SharedWriter`] instance is obtained by calling `Readline::new()` (in the
///   `r3bl_terminal_async` crate).
/// - It also returns a `Readline` instance associated with the writer.
///
/// # Nothing is output without terminating with a newline, unless you call [SharedWriter::flush()]
///
/// This is the nature of buffered writing in POSIX. It isn't really specific to this
/// crate.
///
/// Data written to a [`crate::SharedWriter`] is only output when a line feed (`'\n'`) has
/// been written and either is executing on the associated `Readline` instance both in
/// `readline.rs`:
/// - `Readline::readline()`.
/// - `manage_shared_writer_output::flush_internal()`.
///
/// If you want to output data without a newline, you can call [`SharedWriter::flush()`].
#[derive(Debug)]
pub struct SharedWriter {
    /// Holds the data to be written to the terminal.
    pub buffer: Text,

    /// Sender end of the channel, the receiver end is in `Readline` (in the
    /// `r3bl_terminal_async` crate), which does the actual printing to `stdout`.
    pub line_state_control_channel_sender:
        tokio::sync::mpsc::Sender<LineStateControlSignal>,

    /// This is set to `true` when this struct is cloned. Only the first instance of this
    /// struct will report errors when [`std::io::Write::write()`] fails, due to the
    /// receiver end of the channel being closed.
    pub silent_error: bool,

    /// Unique identifier for the `SharedWriter` instance.
    pub uuid: uuid::Uuid,
}

impl PartialEq for SharedWriter {
    fn eq(&self, other: &Self) -> bool { self.uuid == other.uuid }
}

/// Signals that can be sent to the `line` channel, which is monitored by the task.
#[derive(Debug, Clone)]
pub enum LineStateControlSignal {
    Line(Text),
    Flush,
    Pause,
    Resume,
    SpinnerActive(tokio::sync::broadcast::Sender<()>),
    SpinnerInactive,
}

impl SharedWriter {
    /// Creates a new instance of `SharedWriter` with an empty buffer and a
    /// [`tokio::sync::mpsc::Sender`] end of the channel.
    pub fn new(line_sender: tokio::sync::mpsc::Sender<LineStateControlSignal>) -> Self {
        Self {
            buffer: Default::default(),
            line_state_control_channel_sender: line_sender,
            silent_error: false,
            uuid: uuid::Uuid::new_v4(),
        }
    }
}

/// Custom [Clone] implementation for [`SharedWriter`]. This ensures that each new
/// instance gets its own buffer to write data into. And a [Clone] of the
/// [Self::line_state_control_channel_sender], so all the [`LineStateControlSignal`]s end
/// up in the same `line` [tokio::sync::mpsc::channel] that lives in the `Readline`
/// instance (in the `r3bl_terminal_async` crate).
impl Clone for SharedWriter {
    fn clone(&self) -> Self {
        Self {
            buffer: Default::default(),
            line_state_control_channel_sender: self
                .line_state_control_channel_sender
                .clone(),
            silent_error: true,
            uuid: self.uuid,
        }
    }
}

impl Write for SharedWriter {
    fn write(&mut self, payload: &[u8]) -> io::Result<usize> {
        let self_buffer = &mut self.buffer;

        // Append the payload to self_buffer.
        self_buffer.extend_from_slice(payload);

        // If self_buffer ends with a newline, send it to the line_sender.
        if self_buffer.ends_with(b"\n") {
            match self
                .line_state_control_channel_sender
                .try_send(LineStateControlSignal::Line(self_buffer.clone()))
            {
                Ok(_) => {
                    self_buffer.clear();
                }
                Err(_) => {
                    if !self.silent_error {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "SharedWriter Receiver has closed",
                        ));
                    }
                }
            }
        };

        Ok(payload.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        match self
            .line_state_control_channel_sender
            .try_send(LineStateControlSignal::Line(self.buffer.clone()))
        {
            Ok(_) => {
                self.buffer.clear();
            }
            Err(_) => {
                if !self.silent_error {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "SharedWriter Receiver has closed",
                    ));
                }
            }
        }

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write() {
        let (line_sender, _) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter::new(line_sender);
        shared_writer.write_all(b"Hello, World!").unwrap();
        assert_eq!(shared_writer.buffer, b"Hello, World!");
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_write_flush() {
        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter::new(line_sender);

        shared_writer.write_all(b"Hello, World!").unwrap();
        shared_writer.flush().unwrap();
        assert_eq!(shared_writer.buffer, b"");

        let it = line_receiver.recv().await.unwrap();
        if let LineStateControlSignal::Line(bytes) = it {
            assert_eq!(bytes, b"Hello, World!".to_vec());
        } else {
            panic!("Expected LineStateControlSignal::Line, got something else");
        }
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_writeln_no_flush() {
        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter::new(line_sender);
        shared_writer.write_all(b"Hello, World!\n").unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let it = line_receiver.recv().await.unwrap();
        if let LineStateControlSignal::Line(bytes) = it {
            assert_eq!(bytes, b"Hello, World!\n".to_vec());
        } else {
            panic!("Expected LineStateControlSignal::Line, got something else");
        }
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_clone_silent_error() {
        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter::new(line_sender);
        assert!(!shared_writer.silent_error);

        let mut cloned_writer = shared_writer.clone();
        assert!(cloned_writer.silent_error);

        cloned_writer.write_all(b"Hello, World!\n").unwrap();
        assert!(cloned_writer.buffer.is_empty());

        // Will not produce error.
        line_receiver.close();
        cloned_writer.write_all(b"Hello, World!\n").unwrap();

        // Will produce error.
        assert!(shared_writer.write_all(b"error\n").is_err());
    }
}

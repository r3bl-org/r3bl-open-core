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

use crate::{LineControlSignal, Text};
use std::io::{self, Write};

/// Cloneable object that implements [`Write`] and allows for sending data
/// to the terminal without messing up the [`crate::Readline`].
///
/// A `SharedWriter` instance is obtained by calling [`crate::Readline::new()`], which
/// also returns a [`crate::Readline`] instance associated with the writer.
///
/// Data written to a `SharedWriter` is only output when a line feed (`'\n'`) has been
/// written and either [`crate::Readline::readline()`] or
/// [`crate::pause_and_resume_support::flush_internal()`] is executing on the associated
/// `Readline` instance.
pub struct SharedWriter {
    /// Holds the data to be written to the terminal.
    pub buffer: Text,

    /// Sender end of the channel, the receiver end is in [`crate::Readline`], which does
    /// the actual printing to `stdout`.
    pub line_sender: tokio::sync::mpsc::Sender<LineControlSignal>,
}

/// Custom [Clone] implementation for [`SharedWriter`]. This ensures that each new
/// instance gets its own buffer to write data into. And a [Clone] of the
/// [Self::line_sender], so all the [`LineControlSignal`]s end up in the same `line`
/// [tokio::sync::mpsc::channel] that lives in the [`crate::Readline`] instance.
impl Clone for SharedWriter {
    fn clone(&self) -> Self {
        Self {
            buffer: Vec::new(),
            line_sender: self.line_sender.clone(),
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
                .line_sender
                .try_send(LineControlSignal::Line(self_buffer.clone()))
            {
                Ok(_) => {
                    self_buffer.clear();
                }
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "SharedWriter Receiver has closed",
                    ));
                }
            }
        };

        Ok(payload.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write() {
        let (line_sender, _) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter {
            buffer: Vec::new(),
            line_sender,
        };
        shared_writer.write_all(b"Hello, World!").unwrap();
        assert_eq!(shared_writer.buffer, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_writeln() {
        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter {
            buffer: Vec::new(),
            line_sender,
        };
        shared_writer.write_all(b"Hello, World!\n").unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let it = line_receiver.recv().await.unwrap();
        assert_eq!(it, LineControlSignal::Line(b"Hello, World!\n".to_vec()));
    }
}

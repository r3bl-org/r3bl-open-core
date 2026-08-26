// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words POLLOUT EINTR EBADF

use super::BackpressureStdout;
use std::io::Write;

impl Write for BackpressureStdout {
    /// Writes a buffer of bytes into standard output, waiting for buffer capacity if the
    /// OS buffer fills up ([`ErrorKind::WouldBlock`]).
    ///
    /// [`ErrorKind::WouldBlock`]: std::io::ErrorKind::WouldBlock
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            match self.0.write(buf) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.wait_until_writable()?;
                }
                other => return other,
            }
        }
    }

    /// Flushes the standard output stream, waiting for buffer capacity if the OS
    /// buffer fills up ([`ErrorKind::WouldBlock`]).
    ///
    /// [`ErrorKind::WouldBlock`]: std::io::ErrorKind::WouldBlock
    fn flush(&mut self) -> std::io::Result<()> {
        loop {
            match self.0.flush() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.wait_until_writable()?;
                }
                other => return other,
            }
        }
    }
}

impl BackpressureStdout {
    /// Blocks the calling thread until [`std::io::Stdout`] has buffer capacity available
    /// to accept write data ([`POLLOUT`]).
    ///
    /// - Returns `Ok(())` when write capacity is ready, or when awakened by a POSIX
    ///   signal ([`EINTR`]) so the caller can retry.
    /// - Returns `Err` immediately on fatal OS descriptor errors (such as [`EBADF`]).
    ///
    /// [`EBADF`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`EINTR`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`POLLOUT`]: https://man7.org/linux/man-pages/man2/poll.2.html
    fn wait_until_writable(&self) -> std::io::Result<()> {
        let mut poll_fd = [rustix::event::PollFd::new(
            &self.0,
            rustix::event::PollFlags::OUT,
        )];

        match rustix::event::poll(&mut poll_fd, None) {
            Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
            Err(fatal_errno) => Err(std::io::Error::from(fatal_errno)),
        }
    }
}

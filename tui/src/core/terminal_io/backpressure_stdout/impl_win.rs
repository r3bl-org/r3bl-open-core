// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::BackpressureStdout;
use std::io::Write;

impl Write for BackpressureStdout {
    /// Writes a buffer of bytes into standard output, yielding the thread timeslice
    /// via [`yield_now()`] if the OS buffer fills up
    /// ([`std::io::ErrorKind::WouldBlock`]).
    ///
    /// [`yield_now()`]: std::thread::yield_now
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

    /// Flushes the standard output stream, yielding the thread timeslice via
    /// [`yield_now()`] if the OS buffer fills up ([`std::io::ErrorKind::WouldBlock`]).
    ///
    /// [`yield_now()`]: std::thread::yield_now
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

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words errno EPERM

use crate::EIO;
use std::io::BufRead;

/// Extension trait for [`BufRead`] to handle [`EIO`] errors as [`EOF`] when reading lines
/// from a [`PTY`] controller.
///
/// [`EIO`] (`errno` `5`) is primarily a Linux-specific signal for [`PTY`] closure. This
/// trait provides a safe way to normalize this error into an [`EOF`] (`Ok(0)`) across all
/// platforms.
///
/// For a detailed explanation of why this is necessary, see the [POSIX `EOF` vs Linux
/// `EIO`] section in [`PtyPair`].
///
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
/// [`PTY`]: crate::pty_pair#what-is-a-pty
/// [`PtyPair`]: crate::PtyPair
/// [POSIX `EOF` vs Linux `EIO`]: crate::PtyPair#posix-eof-vs-linux-eio
pub trait BufReadExt: BufRead {
    /// Reads a line of text, normalizing [`EIO`] errors to [`EOF`] (`Ok(0)`).
    ///
    /// # Errors
    ///
    /// Returns any I/O error encountered during reading, except for [`EIO`], which is
    /// mapped to `Ok(0)` (same as [`EOF`]).
    ///
    /// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
    /// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
    fn read_line_eio_to_eof(&mut self, buf: &mut String) -> std::io::Result<usize> {
        match self.read_line(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.raw_os_error() == Some(EIO) => Ok(0),
            Err(e) => Err(e),
        }
    }
}

/// Provide blanket implementation for [`BufRead`].
impl<R: BufRead> BufReadExt for R {}

#[cfg(test)]
mod tests {
    use super::{super::MockReaderErrOnly, *};
    use std::io::{self, Cursor};

    #[test]
    fn test_read_line_eio_to_eof_passes_through_normal_read() {
        let mut reader = Cursor::new("hello\nworld\n");
        let mut buf = String::new();
        assert_eq!(reader.read_line_eio_to_eof(&mut buf).unwrap(), 6);
        assert_eq!(buf, "hello\n");
    }

    #[test]
    fn test_read_line_eio_to_eof_normalizes_eio_to_eof() {
        let mut reader = MockReaderErrOnly {
            read_impl_fn: || Err(io::Error::from_raw_os_error(EIO)),
        };
        let result = reader.read_line_eio_to_eof(&mut String::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_read_line_eio_to_eof_passes_through_other_os_error() {
        let mut reader = MockReaderErrOnly {
            read_impl_fn: || {
                let err = io::Error::from_raw_os_error(1);
                Err::<usize, _>(err)
            },
        };
        let result = reader.read_line_eio_to_eof(&mut String::new());
        assert!(result.is_err());
        let raw_os_err = result.unwrap_err().raw_os_error();
        assert_eq!(raw_os_err.unwrap(), 1);
    }

    #[test]
    fn test_read_line_eio_to_eof_passes_through_non_os_error() {
        let mut reader = MockReaderErrOnly {
            read_impl_fn: || {
                // Do NOT use io::ErrorKind::Interrupted here, since std lib will just
                // keep retrying and this test will hang.
                let err = io::Error::other("error");
                Err::<usize, _>(err)
            },
        };
        let result = reader.read_line_eio_to_eof(&mut String::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::io::Read as _;

/// A mock implementation of [`std::io::Read`] and [`std::io::BufRead`] that uses a
/// closure to provide return values.
///
/// It is specifically designed to test error handling and normalization logic (like
/// converting Linux-specific [`EIO`] errors to [`EOF`]).
///
/// **Constraint**: This mock is designed strictly to return errors. Any successful read
/// path triggered via [`std::io::BufRead`] will result in a panic via
/// [`unreachable!()`].
///
/// [`EIO`]: https://man7.org/linux/man-pages/man3/errno.3.html
/// [`EOF`]: https://en.wikipedia.org/wiki/End-of-file
#[derive(Debug)]
pub struct MockReaderErrOnly<F: FnMut() -> std::io::Result<usize>> {
    /// The closure that provides the implementation for the [`std::io::Read::read`]
    /// method.
    pub read_impl_fn: F,
}

impl<F> std::io::Read for MockReaderErrOnly<F>
where
    F: FnMut() -> std::io::Result<usize>,
{
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { (self.read_impl_fn)() }
}

impl<F> std::io::BufRead for MockReaderErrOnly<F>
where
    F: FnMut() -> std::io::Result<usize>,
{
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let Err(err) = self.read(&mut []) else {
            unreachable!();
        };
        Err(err)
    }

    fn consume(&mut self, _: usize) { unreachable!() }
}

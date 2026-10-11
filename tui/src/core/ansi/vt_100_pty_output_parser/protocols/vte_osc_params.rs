// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Newtype wrapper and iterator providing ergonomic parameter extraction for
//! [`OSC`] sequences parsed by [`VTE`].
//!
//! When [`VTE`] encounters an [`OSC`] sequence it invokes
//! [`vte::Perform::osc_dispatch(&[&[u8]], bool)`][vte_osc_dispatch].
//!
//! For this sequence:
//! - `ESC ] 8 ; id=1 ; https://example.com ST`
//!
//! We get:
//! ```text
//!     params.0 = [
//!        [b'8'],                       // index 0: OSC Command Code
//!        [b'i', b'd', b'=', ...],      // index 1: Argument 1 (metadata)
//!        [b'h', b't', b't', b'p', ...] // index 2: Argument 2 (URI)
//!    ]
//! ```
//!
//! Unlike [`CSI`] parameters which are numeric integers managed by [`vte::Params`] and
//! extended via [`ParamsExt`], [`OSC`] parameters are arbitrary byte payloads
//! representing strings, URIs, or binary data. [`VTE`] supplies them as a raw nested
//! slice `&[&[u8]]`:
//! - Parameter `0` is the command code (e.g. `b"0"`, `b"8"`).
//! - Parameters `1..` are the command arguments.
//!
//! [`VteOscParams`] wraps this raw slice in a zero-cost, type-safe newtype, providing
//! domain methods to cleanly extract the command code and sequentially consume
//! arguments as [`UTF-8`] strings or raw byte slices via [`OscArgsIter`].
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::VteOscParams;
//!
//! let raw: &[&[u8]] = &[b"8", b"id=link1", b"https://example.com"];
//! let params = VteOscParams::from(raw);
//!
//! assert_eq!(params.code(), Some("8"));
//! assert_eq!(params.args_count(), 2);
//!
//! let mut args = params.into_iter();
//! assert_eq!(args.next_str(), Some("id=link1"));
//! assert_eq!(args.next_str(), Some("https://example.com"));
//! assert_eq!(args.next_str(), None);
//! ```
//!
//! [`CSI`]: crate::core::ansi::vt_100_pty_output_parser::protocols::CsiSequence
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`ParamsExt`]: crate::ParamsExt
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VTE`]: mod@vte
//! [vte_osc_dispatch]: vte::Perform::osc_dispatch

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// VteOscParams
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Transparent newtype wrapper over the raw nested slice `&[&[u8]]` supplied by [`VTE`]
/// `osc_dispatch` callback.
///
/// Encapsulates the entire parameter list of an [`OSC`] sequence, where position `0` is
/// the command code and positions `1..` are the command arguments.
///
/// [`OSC`]: crate::core::ansi::osc::OscSequence
/// [`VTE`]: mod@vte
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VteOscParams<'a>(&'a [&'a [u8]]);

impl<'a> From<&'a [&'a [u8]]> for VteOscParams<'a> {
    fn from(value: &'a [&'a [u8]]) -> Self { VteOscParams(value) }
}

impl<'a> VteOscParams<'a> {
    /// Returns the command code (the 0th parameter) decoded as a [`UTF-8`] string (e.g.
    /// `"0"`, `"8"`). Returns [`None`] if the parameter slice is empty or if
    /// parameter `0` is not valid [`UTF-8`].
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn code(&self) -> Option<&'a str> {
        let code_bytes = self.0.first()?;
        std::str::from_utf8(code_bytes).ok()
    }

    /// Returns the raw bytes of the command code (the 0th parameter).
    /// Returns [`None`] if the parameter slice is empty.
    #[must_use]
    pub fn code_raw(&self) -> Option<&'a [u8]> { self.0.first().copied() }

    /// Returns the argument at 0-based index `n` *after* the command code, decoded as
    /// [`UTF-8`]. Returns [`None`] if index `n` is out of bounds or if the argument
    /// is not valid [`UTF-8`].
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn arg_str(&self, n: usize) -> Option<&'a str> {
        let bytes = self.arg_raw(n)?;
        std::str::from_utf8(bytes).ok()
    }

    /// Returns the raw bytes of the argument at 0-based index `n` *after* the command
    /// code. Returns [`None`] if index `n` is out of bounds.
    #[must_use]
    pub fn arg_raw(&self, n: usize) -> Option<&'a [u8]> {
        self.0.get(n.checked_add(1)?).copied()
    }

    /// Returns the total number of arguments (excluding the command code at position
    /// `0`).
    #[must_use]
    pub fn args_count(&self) -> usize { self.0.len().saturating_sub(1) }
}

/// Enables natural `for arg in params` iteration over argument byte slices.
impl<'a> IntoIterator for VteOscParams<'a> {
    type Item = &'a [u8];
    type IntoIter = OscArgsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        // Skip the command code (first item in the nested slice).
        let args_slice = match self.0 {
            [] => &[],
            [_, rest @ ..] => rest,
        };

        OscArgsIter {
            inner: args_slice.iter(),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// OscArgsIter
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Iterator over [`OSC`] command arguments (excluding the command code at position `0`).
///
/// Yields each argument as a borrowed byte slice `&'a [u8]`, with convenience methods
/// like [`next_str`][Self::next_str] for sequential [`UTF-8`] string decoding.
///
/// [`OSC`]: crate::core::ansi::osc::OscSequence
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[derive(Debug, Clone)]
pub struct OscArgsIter<'a> {
    inner: std::slice::Iter<'a, &'a [u8]>,
}

impl<'a> Iterator for OscArgsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> { self.inner.next().copied() }

    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

/// Forward exact size guarantees from the underlying slice so callers can check
/// the remaining argument count via `.len()`.
impl ExactSizeIterator for OscArgsIter<'_> {}

impl<'a> OscArgsIter<'a> {
    /// Yields the next argument decoded as a [`UTF-8`] string slice (`&'a str`).
    ///
    /// Returns [`None`] if:
    /// - There are no more arguments remaining.
    /// - The next argument contains invalid [`UTF-8`] bytes.
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn next_str(&mut self) -> Option<&'a str> {
        let bytes = self.next()?;
        std::str::from_utf8(bytes).ok()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Unit Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_params() {
        let raw: &[&[u8]] = &[];
        let empty: VteOscParams = raw.into();
        assert_eq!(empty.code(), None);
        assert_eq!(empty.code_raw(), None);
        assert_eq!(empty.args_count(), 0);
        assert_eq!(empty.arg_str(0), None);
        assert_eq!(empty.arg_raw(0), None);

        let mut iter = empty.into_iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_str(), None);
    }

    #[test]
    fn test_code_only_params() {
        let raw: &[&[u8]] = &[b"0"];
        let code_only: VteOscParams = raw.into();
        assert_eq!(code_only.code(), Some("0"));
        assert_eq!(code_only.code_raw(), Some(&b"0"[..]));
        assert_eq!(code_only.args_count(), 0);
        assert_eq!(code_only.arg_str(0), None);
        assert_eq!(code_only.arg_raw(0), None);

        let mut iter = code_only.into_iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_single_argument() {
        let raw: &[&[u8]] = &[b"2", b"Terminal Title"];
        let title_seq: VteOscParams = raw.into();
        assert_eq!(title_seq.code(), Some("2"));
        assert_eq!(title_seq.args_count(), 1);
        assert_eq!(title_seq.arg_str(0), Some("Terminal Title"));
        assert_eq!(title_seq.arg_raw(0), Some(&b"Terminal Title"[..]));
        assert_eq!(title_seq.arg_str(1), None);

        let mut iter = title_seq.into_iter();
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next_str(), Some("Terminal Title"));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next_str(), None);
    }

    #[test]
    fn test_multiple_arguments_hyperlink() {
        let raw: &[&[u8]] = &[b"8", b"id=link1", b"https://example.com"];
        let hyperlink_seq: VteOscParams = raw.into();
        assert_eq!(hyperlink_seq.code(), Some("8"));
        assert_eq!(hyperlink_seq.args_count(), 2);
        assert_eq!(hyperlink_seq.arg_str(0), Some("id=link1"));
        assert_eq!(hyperlink_seq.arg_str(1), Some("https://example.com"));
        assert_eq!(hyperlink_seq.arg_str(2), None);

        let mut iter = hyperlink_seq.into_iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next_str(), Some("id=link1"));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next_str(), Some("https://example.com"));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next_str(), None);
    }

    #[test]
    fn test_into_iterator() {
        let raw: &[&[u8]] = &[b"8", b"id=link1", b"https://example.com"];
        let params: VteOscParams = raw.into();
        let collected: Vec<&str> = params
            .into_iter()
            .filter_map(|bytes| std::str::from_utf8(bytes).ok())
            .collect();
        assert_eq!(collected, vec!["id=link1", "https://example.com"]);
    }

    #[test]
    fn test_invalid_utf8() {
        let raw: &[&[u8]] = &[b"\xFF", b"\xFE"];
        let invalid: VteOscParams = raw.into();
        assert_eq!(invalid.code(), None);
        assert_eq!(invalid.code_raw(), Some(&b"\xFF"[..]));
        assert_eq!(invalid.args_count(), 1);
        assert_eq!(invalid.arg_str(0), None);
        assert_eq!(invalid.arg_raw(0), Some(&b"\xFE"[..]));

        let mut iter = invalid.into_iter();
        assert_eq!(iter.next_str(), None);
    }
}

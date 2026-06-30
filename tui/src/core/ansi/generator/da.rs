// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Attributes ([`DA`]) sequence builders.
//!
//! This module provides types and builders for handling [`DA`] requests and responses in
//! terminal emulation.
//!
//! ## [`DA`] Codes and Constants
//!
//! [`DA`] sequences are used for bidirectional communication between terminals and
//! applications:
//! - **Requests** (INCOMING): Applications send [`CSI`] sequences to request device
//!   attributes
//! - **Responses** (OUTGOING): Terminal emulator sends back [`ESC`] sequences with the
//!   requested device features
//!
//! ### Request Format (from application)
//! - `CSI c` or `CSI 0 c` - Request Primary Device Attributes (DA1)
//!
//! ### Response Format (from terminal)
//! - `ESC [ ? 62 ; 22 c` - VT220 with [`ANSI`] color support
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CSI`]: crate::CsiSequence
//! [`DA`]: crate::DaSequence
//! [`ESC`]: crate::EscSequence

use crate::{BufTextStorage, FastStringify, generate_impl_display_for_fast_stringify, DA1_VT220_COLOR_RESPONSE_STR};
use std::fmt::{self};

/// Builds Device Attributes (DA) response sequences.
///
/// This sequence is sent TO the [`PTY`] in response to a DA query (e.g. `CSI c`).
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaSequence {
    /// Primary Device Attributes response.
    ///
    /// Indicates a `VT220`-family terminal with `ANSI` color support (`ESC [ ? 62 ; 22
    /// c`).
    ///
    /// - The `VT200` series (specifically `VT220`) is a strict superset of `VT100`. By
    ///   responding with `62` (`VT220`-family) and the parameter `22` ([`ANSI`] color
    ///   support), we are essentially telling the child application: "I fully support the
    ///   `VT100` specification, but I also support modern extensions like [`ANSI`] color
    ///   and advanced control sequences."
    /// - This is a standard industry practice. Almost all modern terminal emulators (like
    ///   [`WezTerm`], [`Alacritty`], [`GNOME Terminal`], etc.) identify themselves as
    ///   `VT220`, `VT320`, or `VT420` for exactly this reason: to unlock colors and
    ///   modern features in child apps while remaining backwards compatible with the
    ///   `VT100` standard.
    /// - Note - In our codebase we use the `VT100` in our type & module names because
    ///   it's the universally recognized name for the technology and protocol. It
    ///   encompasses `VT220` with color extensions, etc. It's very similar to how we
    ///   still use the term [`TTY`] (which stands for Teletypewriter) even though we
    ///   haven't used mechanical teletypewriters with ink and paper in over 40 years.
    ///
    /// [`Alacritty`]: https://alacritty.org/
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`GNOME Terminal`]: https://help.gnome.org/users/gnome-terminal/stable/
    /// [`TTY`]: https://en.wikipedia.org/wiki/Tty_(Unix)
    /// [`WezTerm`]: https://wezfurlong.org/wezterm/
    PrimaryDeviceAttributes,
}

mod impl_da {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl FastStringify for DaSequence {
        fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
            match self {
                DaSequence::PrimaryDeviceAttributes => {
                    acc.push_str(DA1_VT220_COLOR_RESPONSE_STR);
                }
            }
            Ok(())
        }

        fn write_buf_to_fmt(
            &self,
            acc: &BufTextStorage,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result {
            f.write_str(&acc.clone())
        }
    }
}

generate_impl_display_for_fast_stringify!(DaSequence);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_da_primary_device_attributes() {
        let sequence = DaSequence::PrimaryDeviceAttributes;
        assert_eq!(sequence.to_string(), DA1_VT220_COLOR_RESPONSE_STR);
    }

    #[test]
    fn test_write_to_buf_produces_correct_ansi_sequence() {
        let sequence = DaSequence::PrimaryDeviceAttributes;
        let mut acc = BufTextStorage::new();

        sequence.write_to_buf(&mut acc).unwrap();
        assert_eq!(acc.clone(), DA1_VT220_COLOR_RESPONSE_STR);
    }
}

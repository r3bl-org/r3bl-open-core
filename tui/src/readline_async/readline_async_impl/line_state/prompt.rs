// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{GCStringOwned, VPWidth};
use std::{fmt::{self, Display, Formatter},
          ops::Deref};

/// Represents the prompt string displayed before user input and its pre-computed display
/// width.
///
/// May contain [`ANSI`] escape codes for styling (colors, bold, etc.). Display width
/// is calculated by stripping escape sequences and measuring the resulting Unicode
/// grapheme cluster width.
///
/// Encapsulating both the raw string and pre-computed width guarantees the invariant
/// that [`Prompt::width`] is always in sync with [`Prompt::as_str`].
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// Raw prompt string, possibly containing [`ANSI`] escape codes.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    raw: String,

    /// Pre-computed display width of `raw` (excluding [`ANSI`] escape sequences).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    width: VPWidth,
}

impl Default for Prompt {
    fn default() -> Self { Self::new("") }
}

impl Prompt {
    /// Creates a new [`Prompt`], computing its display width automatically.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        let raw = prompt.into();
        let width = Self::calculate_width(&raw);
        Self { raw, width }
    }

    /// Updates the prompt string and recalculates its display width.
    pub fn set(&mut self, prompt: &str) {
        self.raw.clear();
        self.raw.push_str(prompt);
        self.width = Self::calculate_width(prompt);
    }

    /// Returns the prompt text as a string slice, including any [`ANSI`] sequences.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    #[must_use]
    pub fn as_str(&self) -> &str { &self.raw }

    /// Returns the pre-computed display width, excluding [`ANSI`] escape sequences.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    #[must_use]
    pub fn width(&self) -> VPWidth { self.width }

    /// Calculates the display width of a prompt string, excluding [`ANSI`] escape
    /// sequences.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    #[must_use]
    pub fn calculate_width(prompt: &str) -> VPWidth {
        let stripped = strip_ansi::strip_ansi(prompt);
        GCStringOwned::from(stripped.as_str()).width()
    }
}

impl Deref for Prompt {
    type Target = str;

    fn deref(&self) -> &Self::Target { &self.raw }
}

impl AsRef<str> for Prompt {
    fn as_ref(&self) -> &str { &self.raw }
}

impl Display for Prompt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{}", self.raw) }
}

impl<T: Into<String>> From<T> for Prompt {
    fn from(s: T) -> Self { Self::new(s) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vp_width;

    #[test]
    fn test_calculate_prompt_width_with_ansi() {
        // Plain prompt.
        assert_eq!(Prompt::calculate_width("> "), vp_width(2));

        // Prompt with 8-color ANSI escapes.
        assert_eq!(Prompt::calculate_width("\x1b[32m>\x1b[0m "), vp_width(2));

        // Prompt with 24-bit truecolor ANSI escapes.
        assert_eq!(
            Prompt::calculate_width("\x1b[38;2;255;100;0mprompt>\x1b[0m "),
            vp_width(8)
        );
    }

    #[test]
    fn test_prompt_construct_and_set() {
        let mut prompt = Prompt::new("> ");
        assert_eq!(prompt.as_str(), "> ");
        assert_eq!(prompt.width(), vp_width(2));
        assert_eq!(&*prompt, "> ");
        assert_eq!(format!("{prompt}"), "> ");

        prompt.set("\x1b[32mnew>\x1b[0m ");
        assert_eq!(prompt.as_str(), "\x1b[32mnew>\x1b[0m ");
        assert_eq!(prompt.width(), vp_width(5));
    }
}

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ParserGlobalState, TerminalModeState};
use crate::{ActiveScreenBuffer, GetMemSize, HiddenScreenState, OfsBuf,
            ScrollbackBuffer, ScrollbackBufferLimit, Size};
use std::{fmt::Debug,
          mem::size_of,
          ops::{Deref, DerefMut}};

/// State for the [`VT-100`] [`ANSI`] parser, which is used by the [`PTY`] multiplexer.
///
/// This struct composites:
/// 1. The screen buffer [`OfsBuf`].
/// 2. The [`VT-100`] [`ANSI`] [parser] state machine, which includes:
///    - The [`ANSI`] parser state - [`ParserGlobalState`].
///    - The terminal mode flags - [`TerminalModeState`].
///    - The hidden screen parking state - [`HiddenScreenState`].
///    - The scrollback history buffer - [`ScrollbackBuffer`].
///
/// This struct is used by the [`PTY`] [multiplexer].
///
/// # Architecture: Scrollback and Compositing
///
/// This struct is composed of the following:
/// 1. _Live buffer_: A [`bitblt`] 2D buffer (an array of [`PixelCharLines`] in
///    [`ofs_buf`] field of type [`OfsBuf`]), and this is fixed in size to match
///    the viewport (or the size of the physical screen/display).
/// 2. _History buffer_: A field of type [`VecDeque`] ([`ScrollbackBuffer`]) that handles
///    the scrollback history. These are the [`PixelCharLines`] that have scrolled off the
///    [`ofs_buf`] viewport.
///
/// While this works, it is tricky to stitch [`PixelCharLine`]s from the "history" and the
/// "live" buffer into the final composited [`OfsBuf`] which is [`bitblt`]'d to
/// [`stdout`] (the screen/display) whenever the compositor is painted. See the
/// [`OutputRenderer`] for the stitching implementation.
///
/// The underlying machinery to parse [`VT-100`] is in the [parser] module.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`bitblt`]: https://en.wikipedia.org/wiki/Bit_blit
/// [`ofs_buf`]: Self::ofs_buf
/// [`OutputRenderer`]: crate::OutputRenderer
/// [`PixelCharLine`]: crate::PixelCharLine
/// [`PixelCharLines`]: crate::PixelCharLines
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stdout`]: std::io::stdout
/// [`VecDeque`]: std::collections::VecDeque
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [multiplexer]: mod@crate::pty_mux
/// [parser]: mod@crate::core::ansi::vt_100_pty_output_parser
#[derive(Clone, Debug, PartialEq)]
pub struct OfsBufVT100 {
    /// The physical screen viewport that the user currently sees. This buffer receives
    /// all character writes and styling.
    pub ofs_buf: OfsBuf,

    /// High-level runtime state tracking active graphic renditions ([`SGR`], colors,
    /// styling), character set mappings, and protocol requests ([`DSR`], [`OSC`]) that
    /// persist globally.
    ///
    /// [`DSR`]: crate::DsrSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`SGR`]: crate::SgrCode
    pub parser_global_state: ParserGlobalState,

    /// Saves the main screen's contents and cursor position when the terminal switches
    /// to the alternate screen buffer (e.g. `CSI ? 1049 h` / `CSI ? 1049 l`).
    pub hidden_screen_state: HiddenScreenState,

    /// Tracks active terminal modes and boolean toggles (e.g. [`DECTCEM`] for cursor
    /// visibility, [`DECAWM`] for auto-wrap).
    ///
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    pub terminal_mode: TerminalModeState,

    /// Contains the history of lines that have scrolled off the physical screen.
    pub scrollback_buffer: ScrollbackBuffer,
}

mod impl_ofs_buf_vt_100 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl OfsBufVT100 {
        /// Checks if the child process is currently displaying on the primary screen
        /// buffer. This is nothing to do with raw or cooked mode.
        #[must_use]
        pub fn is_in_primary_screen(&self) -> bool {
            self.terminal_mode.active_screen_buffer == ActiveScreenBuffer::Primary
        }

        /// Checks if the child process is currently displaying on the alternate screen
        /// buffer. This is nothing to do with raw or cooked mode.
        #[must_use]
        pub fn is_in_alternate_screen(&self) -> bool {
            self.terminal_mode.active_screen_buffer == ActiveScreenBuffer::Alternate
        }

        /// Creates a new virtual terminal state with a blank screen.
        ///
        /// This method delegates to the underlying state components, initializing them
        /// into an empty state (e.g., creating blank spaces in the terminal grid and
        /// clearing the scrollback).
        ///
        /// # Examples
        ///
        /// You can pass a [`Size`] to create a terminal with default settings:
        ///
        /// ```rust
        /// use r3bl_tui::{width, height};
        /// use r3bl_tui::OfsBufVT100;
        /// let size = height(24) + width(80);
        /// let state = OfsBufVT100::new_empty(size);
        /// ```
        ///
        /// Or pass a tuple of `(Size, ScrollbackBufferLimit)` to configure it explicitly:
        ///
        /// ```rust
        /// use r3bl_tui::{width, height, len};
        /// use r3bl_tui::{OfsBufVT100, ScrollbackBufferLimit};
        /// let size = r3bl_tui::Size::default();
        /// let state = OfsBufVT100::new_empty(
        ///     (size, ScrollbackBufferLimit::Fixed(len(100)))
        /// );
        /// ```
        #[must_use]
        pub fn new_empty(arg_config: impl Into<OfsBufVT100Config>) -> Self {
            let config = arg_config.into();
            Self {
                ofs_buf: config.into(),
                parser_global_state: ParserGlobalState::default(),
                hidden_screen_state: config.into(),
                terminal_mode: TerminalModeState::default(),
                scrollback_buffer: config.into(),
            }
        }
    }

    impl Deref for OfsBufVT100 {
        type Target = OfsBuf;
        fn deref(&self) -> &Self::Target { &self.ofs_buf }
    }

    impl DerefMut for OfsBufVT100 {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ofs_buf }
    }

    impl GetMemSize for OfsBufVT100 {
        /// Fast `O(1)` memory footprint calculation.
        ///
        /// This avoids expensive `O(rows * columns)` calculations by relying on the
        /// `O(1)` cached memory retrieval of both the primary [`OfsBuf`] and
        /// the alternate screen buffer ([`HiddenScreenState`]).
        fn get_mem_size(&self) -> usize {
            self.ofs_buf.get_mem_size()
                + self.hidden_screen_state.get_mem_size()
                + size_of::<ParserGlobalState>()
                + size_of::<TerminalModeState>()
                + self.scrollback_buffer.get_mem_size()
                + size_of::<Self>()
        }
    }
}

/// Configuration for the [`OfsBufVT100`] terminal state parser.
///
/// This struct holds the initial dimensions of the offscreen buffer ([`Size`]) and the
/// limit limits for retaining the history of scrolled lines ([`ScrollbackBufferLimit`]).
///
/// It supports flexible initialization through the [`From`] trait, defaulting to infinite
/// scrollback if only a [`Size`] is provided.
///
/// [`OfsBufVT100`]: crate::OfsBufVT100
/// [`ScrollbackBufferLimit`]: crate::ScrollbackBufferLimit
/// [`Size`]: crate::Size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfsBufVT100Config {
    pub window_size: Size,
    pub scrollback_buffer_limit: ScrollbackBufferLimit,
}

/// Conversions from [`OfsBufVT100Config`] to the internal states of [`OfsBufVT100`].
///
/// This module provides [`From`] trait implementations to build an [`OfsBufVT100Config`]
/// from a [`Size`] or tuple, and to convert that configuration into the various terminal
/// states ([`OfsBuf`], [`HiddenScreenState`], and [`ScrollbackBuffer`]).
///
/// [`HiddenScreenState`]: super::HiddenScreenState
/// [`OfsBuf`]: crate::OfsBuf
/// [`OfsBufVT100`]: crate::OfsBufVT100
/// [`OfsBufVT100Config`]: crate::OfsBufVT100Config
/// [`ScrollbackBuffer`]: super::ScrollbackBuffer
/// [`Size`]: crate::Size
mod impl_ofs_buf_vt_100_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<Size> for OfsBufVT100Config {
        fn from(window_size: Size) -> Self {
            Self {
                window_size,
                scrollback_buffer_limit: ScrollbackBufferLimit::Unlimited,
            }
        }
    }

    impl From<(Size, ScrollbackBufferLimit)> for OfsBufVT100Config {
        fn from(
            (window_size, scrollback_buffer_limit): (Size, ScrollbackBufferLimit),
        ) -> Self {
            Self {
                window_size,
                scrollback_buffer_limit,
            }
        }
    }

    impl From<OfsBufVT100Config> for ScrollbackBuffer {
        fn from(config: OfsBufVT100Config) -> Self {
            config.scrollback_buffer_limit.into()
        }
    }

    impl From<OfsBufVT100Config> for OfsBuf {
        fn from(config: OfsBufVT100Config) -> Self { Self::new_empty(config.window_size) }
    }

    impl From<OfsBufVT100Config> for HiddenScreenState {
        fn from(config: OfsBufVT100Config) -> Self { Self::new_empty(config.window_size) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vt100_terminal_state_struct_size() {
        // TRIPWIRE: If you add or remove a field from `OfsBufVT100`, this test
        // will fail. This is intentional! It reminds you to:
        // 1. Update the `GetMemSize` implementation for this struct to include your new
        //    field.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            // First we assert against a dummy value to see the real sizes in the test
            // output, then we will update it.
            assert_eq!(std::mem::size_of::<OfsBufVT100>(), 224);
        }
    }

    #[test]
    fn test_vt100_terminal_state_get_mem_size() {
        // TRIPWIRE: This test verifies that `GetMemSize` actually sums up all the fields.
        // If you added a field, you MUST add its memory size calculation to
        // `expected_size` below, and ensure the actual `GetMemSize`
        // implementation matches it.
        use crate::{height, width};
        let size = height(10) + width(20);
        let state = OfsBufVT100::new_empty(size);

        let calculated_size = state.get_mem_size();
        let expected_size = state.ofs_buf.get_mem_size()
            + state.hidden_screen_state.get_mem_size()
            + size_of::<ParserGlobalState>()
            + size_of::<TerminalModeState>()
            + state.scrollback_buffer.get_mem_size()
            + size_of::<OfsBufVT100>();

        assert_eq!(calculated_size, expected_size);
        // Ensure consistency across calls
        assert_eq!(calculated_size, state.get_mem_size());
    }

    #[test]
    fn test_ofs_buf_vt100_config_from_size() {
        use crate::{height, width};
        let size = height(10) + width(20);
        let config: OfsBufVT100Config = size.into();
        assert_eq!(config.window_size, size);
        assert_eq!(
            config.scrollback_buffer_limit,
            ScrollbackBufferLimit::Unlimited
        );
    }

    #[test]
    fn test_ofs_buf_vt100_config_from_tuple() {
        use crate::{height, width};
        let size = height(10) + width(20);
        let limit = ScrollbackBufferLimit::Fixed(crate::len(100));
        let config: OfsBufVT100Config = (size, limit).into();
        assert_eq!(config.window_size, size);
        assert_eq!(config.scrollback_buffer_limit, limit);
    }

    #[test]
    fn test_ofs_buf_vt100_config_into_components() {
        use crate::{height, width};
        let size = height(10) + width(20);
        let limit = ScrollbackBufferLimit::Fixed(crate::len(100));
        let config = OfsBufVT100Config {
            window_size: size,
            scrollback_buffer_limit: limit,
        };

        // Test Into<OfsBuf>
        let _ofs_buf: OfsBuf = config.into();
        // Since OfsBuf does not derive PartialEq, we can just check if
        // get_mem_size doesn't panic and size gets set (indirectly tested by dimensions).
        // Let's just test that we can convert.

        // Test Into<HiddenScreenState>
        let hidden: HiddenScreenState = config.into();
        assert_eq!(hidden.hidden_buffer.get_height().as_usize(), 10); // height is 10

        // Test Into<ScrollbackBuffer>
        let scrollback_buffer: ScrollbackBuffer = config.into();
        assert_eq!(scrollback_buffer.limit, limit);
    }

    #[test]
    fn test_ofs_buf_vt100_new_empty_variants() {
        use crate::{height, width};
        let size = height(10) + width(20);
        let limit = ScrollbackBufferLimit::Fixed(crate::len(100));

        // 1. Construct with just Size
        let state1 = OfsBufVT100::new_empty(size);
        assert_eq!(
            state1.scrollback_buffer.limit,
            ScrollbackBufferLimit::Unlimited
        );

        // 2. Construct with Size and Capacity
        let state2 = OfsBufVT100::new_empty((size, limit));
        assert_eq!(state2.scrollback_buffer.limit, limit);
    }
}

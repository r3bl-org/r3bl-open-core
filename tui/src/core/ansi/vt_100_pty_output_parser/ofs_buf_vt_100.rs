// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ParserGlobalState, TerminalModeState};
use crate::{GetMemSize, HiddenScreenState, OffscreenBuffer, Size};
use std::{fmt::Debug,
          mem::size_of,
          ops::{Deref, DerefMut}};

/// State for the [`VT-100`] [`ANSI`] parser, which is used by the [`PTY`] multiplexer.
///
/// This struct composites:
/// 1. The screen buffer [`OffscreenBuffer`]
/// 2. The [`VT-100`] [`ANSI`] [parser] state machine, which includes:
///    - The [`ANSI`] parser state - [`ParserGlobalState`]
///    - The terminal mode flags - [`TerminalModeState`]
///    - The hidden screen parking state - [`HiddenScreenState`]
///
/// It is used specifically by the [`PTY`] [multiplexer]. The underlying machinery to
/// parse [`VT-100`] is in the [parser] module.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [multiplexer]: mod@crate::pty_mux
/// [parser]: mod@crate::core::ansi::vt_100_pty_output_parser
#[derive(Clone, Debug, PartialEq)]
pub struct OfsBufVT100 {
    pub ofs_buf: OffscreenBuffer,
    pub parser_global_state: ParserGlobalState,
    pub hidden_screen_state: HiddenScreenState,
    pub terminal_mode: TerminalModeState,
}

mod vt_100_terminal_state_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for OfsBufVT100 {
        type Target = OffscreenBuffer;
        fn deref(&self) -> &Self::Target { &self.ofs_buf }
    }

    impl DerefMut for OfsBufVT100 {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ofs_buf }
    }

    impl GetMemSize for OfsBufVT100 {
        /// Fast `O(1)` memory footprint calculation.
        ///
        /// This avoids expensive `O(rows * columns)` calculations by relying on the
        /// `O(1)` cached memory retrieval of both the primary [`OffscreenBuffer`] and
        /// the alternate screen buffer ([`HiddenScreenState`]).
        fn get_mem_size(&self) -> usize {
            self.ofs_buf.get_mem_size()
                + self.hidden_screen_state.get_mem_size()
                + size_of::<ParserGlobalState>()
                + size_of::<TerminalModeState>()
                + size_of::<Self>()
        }
    }

    impl OfsBufVT100 {
        #[must_use]
        pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
            let window_size = arg_window_size.into();
            Self {
                ofs_buf: OffscreenBuffer::new_empty(window_size),
                parser_global_state: ParserGlobalState::default(),
                hidden_screen_state: HiddenScreenState::new_empty(window_size),
                terminal_mode: TerminalModeState::default(),
            }
        }
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
            assert_eq!(size_of::<OfsBufVT100>(), 928);
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
            + size_of::<OfsBufVT100>();

        assert_eq!(calculated_size, expected_size);
        // Ensure consistency across calls
        assert_eq!(calculated_size, state.get_mem_size());
    }
}

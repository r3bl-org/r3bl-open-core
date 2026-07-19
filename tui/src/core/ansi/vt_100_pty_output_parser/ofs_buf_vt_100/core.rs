// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::OfsBufVT100Config;
use crate::{Flat2DArray, GetMemSize, GrowableBuffer, OfsBuf, ParserGlobalState,
            PixelChar, TerminalModeState};
use std::{fmt::Debug, mem::size_of};

pub type PrimaryBuffer = OfsBuf<GrowableBuffer>;
pub type AlternateBuffer = OfsBuf<Flat2DArray<PixelChar>>;

/// State for the [`VT-100`] [`ANSI`] parser, which is used by the [`PTY`] multiplexer
/// [`pty_mux`].
///
/// This struct composites:
///
/// 1. Primary Screen Buffer - [`PrimaryBuffer`]
///   - Implements the "primary continuous 2D buffer" architecture natively provided by
///     [`CanvasStorage`]'s [Canvas and Viewport concept].
///   - Used by standard CLI programs (like `ls`, `cat`, `grep`, `head`, `tail`, `cargo
///     build`, etc.) where scrollback history must be preserved.
///   - Access it using [`OfsBufVT100::get_primary_buffer()`] and
///     [`OfsBufVT100::primary_buffer_mut()`].
/// 2. Alternate Screen Buffer - [`OfsBuf<Flat2DArray>`]
///   - A fixed-size buffer for full-screen [`pty_mux`] applications (the size matches the
///     [virtual terminal] emulator window size).
///   - Used by TUI programs (like `vim`, `less`, `top`, `htop`, `hx`, etc.) to provide a
///     full-screen experience without polluting the scrollback history.
///   - Access it using [`OfsBufVT100::get_alternate_buffer()`] and
///     [`OfsBufVT100::get_alternate_buffer_mut()`].
/// 3. [`VT-100`] [`ANSI`] [parser] State Machine
///    - The [`ANSI`] parser state - [`ParserGlobalState`]. Access it using
///      [`OfsBufVT100::get_parser_global_state()`].
///    - The terminal mode flags - [`TerminalModeState`]. Access it using
///      [`OfsBufVT100::get_terminal_mode()`].
///
/// The underlying machinery to parse [`VT-100`] is in the [parser] module.
///
/// # Routing and Active Buffer Selection
///
/// The [`VT-100`] parser can switch between the primary screen buffer (supported by
/// [`GrowableBuffer`]) and the alternate screen buffer (supported by [`Flat2DArray`]).
///
/// Because Rust treats these as two entirely distinct and separate types at compile time,
/// they cannot be used interchangeably without an abstraction:
/// 1. [`PrimaryBuffer`]
/// 2. [`AlternateBuffer`]
///
/// To solve this and avoid writing `match` blocks for every single screen operation, this
/// module relies on the [`OfsBufOpsVT100`] trait (which the two types above implement).
///
/// - The coordinator struct ([`OfsBufVT100`]) provides exactly two helper methods which
///   contain the only `match` blocks in the system, each returning a trait object:
///   - [`get_active_screen_buffer()`] -> `&dyn OfsBufOpsVT100`
///   - [`get_active_screen_buffer_mut()`] -> `&mut dyn OfsBufOpsVT100`
/// - The rest of the parser code simply queries these helpers to obtain the trait object,
///   then calls the operations directly on it. The Rust runtime then dynamically
///   dispatches the calls to the correct concrete buffer implementation, avoiding
///   duplicate matching logic.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`bitblt`]: https://en.wikipedia.org/wiki/Bit_blit
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`CanvasStorage`]: crate::CanvasStorage
/// [`get_active_screen_buffer()`]: OfsBufVT100::get_active_screen_buffer
/// [`get_active_screen_buffer_mut()`]: OfsBufVT100::get_active_screen_buffer_mut
/// [`OfsBufOpsVT100`]: crate::OfsBufOpsVT100
/// [`OutputRenderer`]: crate::OutputRenderer
/// [`PixelCharLine`]: crate::PixelCharLine
/// [`pty_mux`]: crate::pty_mux
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`stdout`]: std::io::stdout
/// [`VecDeque`]: std::collections::VecDeque
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
/// [multiplexer]: mod@crate::pty_mux
/// [parser]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [virtual terminal]: crate::pty_mux#virtual-terminal-architecture
#[derive(Clone, Debug, PartialEq)]
pub struct OfsBufVT100 {
    pub(super) primary_buffer: PrimaryBuffer,
    pub(super) alternate_buffer: AlternateBuffer,
    pub(super) parser_global_state: ParserGlobalState,
    pub(super) terminal_mode: TerminalModeState,
}

impl OfsBufVT100 {
    /// Creates a new virtual terminal state with a blank screen.
    ///
    /// This method delegates to the underlying state components, initializing them into
    /// an empty state (e.g., creating blank spaces in the terminal grid and clearing the
    /// scrollback).
    ///
    /// # Examples
    ///
    /// You can pass a [`VPSize`] to create a terminal with default settings:
    ///
    /// ```rust
    /// use r3bl_tui::OfsBufVT100;
    /// use r3bl_tui::{vp_height, vp_width};
    /// let size = vp_height(24) + vp_width(80);
    /// let state = OfsBufVT100::new_empty(size);
    /// ```
    ///
    /// Or pass a tuple of `(`[`VPSize`]`, `[`StorageLineLimit`]`)` to configure it
    /// explicitly:
    ///
    /// ```rust
    /// use r3bl_tui::{OfsBufVT100, StorageLineLimit, VPSize};
    /// let size = VPSize::default();
    /// let state = OfsBufVT100::new_empty(
    ///     (size, StorageLineLimit::Fixed(100))
    /// );
    /// ```
    ///
    /// [`StorageLineLimit`]: crate::StorageLineLimit
    /// [`VPSize`]: crate::VPSize
    #[must_use]
    pub fn new_empty(arg_config: impl Into<OfsBufVT100Config>) -> Self {
        let config: OfsBufVT100Config = arg_config.into();
        let size = config.window_size;
        let storage_line_limit = config.storage_line_limit;
        Self {
            terminal_mode: TerminalModeState::default(),
            parser_global_state: ParserGlobalState::default(),
            primary_buffer: OfsBuf::new(GrowableBuffer::new_empty(
                size,
                storage_line_limit,
                PixelChar::Spacer,
            )),
            alternate_buffer: OfsBuf::new(Flat2DArray::new_empty(
                size,
                PixelChar::Spacer,
            )),
        }
    }
}

impl GetMemSize for OfsBufVT100 {
    /// Fast `O(1)` memory footprint calculation.
    fn get_mem_size(&self) -> usize {
        self.primary_buffer.get_mem_size()
            + self.alternate_buffer.get_mem_size()
            + size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StorageLineLimit,
                test_fixture_growable_buffer_for_conformance_tests::TestGrowableBufferExt,
                vp_height, vp_width};

    #[test]
    fn test_vt100_terminal_state_struct_size() {
        // TRIPWIRE: If you add or remove a field from `OfsBufVT100`, this test will fail.
        // This is intentional! It reminds you to:
        // 1. Update the `GetMemSize` implementation for this struct to include your new
        //    field.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            // First we assert against a dummy value to see the real sizes in the test
            // output, then we will update it.
            assert_eq!(size_of::<OfsBufVT100>(), 216);
        }
    }

    #[test]
    fn test_vt100_terminal_state_get_mem_size() {
        // TRIPWIRE: This test verifies that `GetMemSize` actually sums up all the fields.
        // If you added a field, you MUST add its memory size calculation to
        // `expected_size` below, and ensure the actual `GetMemSize` implementation
        // matches it.
        let size = vp_height(10) + vp_width(20);
        let state = OfsBufVT100::new_empty(size);

        let calculated_size = state.get_mem_size();
        let expected_size = state.get_primary_buffer().get_mem_size()
            + state.get_alternate_buffer().get_mem_size()
            + size_of::<OfsBufVT100>();

        assert_eq!(calculated_size, expected_size);
        // Ensure consistency across calls
        assert_eq!(calculated_size, state.get_mem_size());
    }

    #[test]
    fn test_ofs_buf_vt100_config_into_components() {
        let size = vp_height(10) + vp_width(20);
        let limit = StorageLineLimit::Fixed(100usize);
        let config = OfsBufVT100Config {
            window_size: size,
            storage_line_limit: limit,
        };

        let ofs_buf =
            OfsBufVT100::new_empty((config.window_size, config.storage_line_limit));
        // Since OfsBuf does not derive PartialEq, we can just check if get_mem_size
        // doesn't panic and size gets set (indirectly tested by dimensions).
        assert_eq!(
            ofs_buf.get_primary_buffer().get_storage_line_limit(),
            StorageLineLimit::Fixed(100usize)
        );
    }

    #[test]
    fn test_ofs_buf_vt100_new_empty_variants() {
        let size = vp_height(10) + vp_width(20);
        let limit = StorageLineLimit::Fixed(100usize);

        // 1. Construct with just Size
        let state1 = OfsBufVT100::new_empty(size);
        assert_eq!(
            state1.get_primary_buffer().get_storage_line_limit(),
            StorageLineLimit::Unlimited
        );

        // 2. Construct with Size and Capacity
        let state2 = OfsBufVT100::new_empty((size, limit));
        assert_eq!(
            state2.get_primary_buffer().get_storage_line_limit(),
            StorageLineLimit::Fixed(100usize)
        );
    }
}

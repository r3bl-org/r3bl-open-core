// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayOverflowResult, ScrollbackBuffer};
use std::{fmt::Debug,
          ops::{Deref, DerefMut}};

/// Represents a vertical scroll amount backed by a [`usize`] value.
///
/// For details on how the screen coordinates map to the active and history buffers during
/// scrolling, see [Mental Model & Visual Layout] in [`OutputRenderer`].
///
/// # Why [`usize`]?
///
/// Unlike [`RowIndex`] and [`RowHeight`] which are backed by [`u16`] because terminal
/// screens never exceed 65,535 rows, this struct is backed by [`usize`] to safely track
/// history in scrollback buffers which can commonly hold 100,000+ lines.
///
/// ## Examples
///
/// ```rust
/// use r3bl_tui::{ScrollbackAmount, ArrayOverflowResult};
///
/// // Create from usize.
/// let amount_to_scroll: usize = 10;
/// let offset: ScrollbackAmount = amount_to_scroll.into();
///
/// // Type-safe addition with saturating bounds.
/// let new_offset = offset.saturating_add(5.into());
/// assert_eq!(*new_offset, 15);
///
/// // Type-safe bounds checking.
/// let history_len = 10;
/// assert_eq!(new_offset.overflows(history_len), ArrayOverflowResult::Overflowed);
/// assert_eq!(offset.overflows(15), ArrayOverflowResult::Within);
/// ```
///
/// [`ChUnit`]: crate::ChUnit
/// [`OutputRenderer`]: super::OutputRenderer
/// [`RowHeight`]: crate::RowHeight
/// [`RowIndex`]: crate::RowIndex
/// [`u16`]: crate::ChUnitPrimitiveType
/// [Mental Model & Visual Layout]: super::OutputRenderer::render_from_active_buffer
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct ScrollbackAmount {
    inner: usize,
}

mod impl_scrollback_amount {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<usize> for ScrollbackAmount {
        fn from(inner: usize) -> Self { Self { inner } }
    }

    impl Deref for ScrollbackAmount {
        type Target = usize;

        fn deref(&self) -> &Self::Target { &self.inner }
    }

    impl DerefMut for ScrollbackAmount {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
    }

    impl ScrollbackAmount {
        #[must_use]
        pub fn saturating_add(self, rhs: ScrollbackAmount) -> Self {
            Self {
                inner: self.inner.saturating_add(rhs.inner),
            }
        }

        #[must_use]
        pub fn saturating_sub(self, rhs: ScrollbackAmount) -> Self {
            Self {
                inner: self.inner.saturating_sub(rhs.inner),
            }
        }

        #[must_use]
        pub fn overflows(&self, length: usize) -> ArrayOverflowResult {
            if self.inner >= length {
                ArrayOverflowResult::Overflowed
            } else {
                ArrayOverflowResult::Within
            }
        }

        #[must_use]
        pub fn clamp_to_max(&self, max_length: usize) -> Self {
            Self {
                inner: self.inner.min(max_length),
            }
        }
    }
}

/// Represents the mapped destination of a physical screen row in the multiplexer
/// viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportRowMapping {
    /// The screen row maps to a line in the scrollback history buffer. The value is the
    /// index within the history buffer.
    History(usize),

    /// The screen row maps to a line in the live active buffer. The value is the index
    /// within the active buffer.
    Live(usize),
}

impl ViewportRowMapping {
    /// Calculates how a physical screen row index maps to the corresponding index in
    /// either the history buffer or the live active buffer, based on the scroll amount.
    ///
    /// When scrolling backwards (aka up), the new text enters the viewport from the top
    /// (not the bottom) of the screen. So, when the user scrolls up by `N` lines
    /// (`scrollback_amt = N`), the physical terminal screen is split horizontally into
    /// two zones as shown below:
    /// 1. `0..N` -> history zone
    /// 2. `N..M` -> live zone
    ///
    /// ```text
    /// How the viewport is split into zones (in the "physical" display/screen)
    ///
    /// 0 ╭───────────────────────╮
    /// 1 │ History Zone          │ ◄─ row_idx < N (maps to ViewportRowMapping::History)
    /// . │                       │
    /// . │                       │
    /// N ├───────────────────────┤
    /// . │ Live Zone             │ ◄─ row_idx >= N (maps to ViewportRowMapping::Live)
    /// . │                       │
    /// . │                       │
    /// M ╰───────────────────────╯
    ///
    /// Legend: N = scrollback_amt, M = pty_max_rows
    /// ```
    ///
    /// - The top rows (`0..N`) are filled with lines from the history buffer.
    /// - The bottom rows are filled with lines from the live active buffer.
    #[must_use]
    pub fn calculate(
        scrollback_amt: ScrollbackAmount,
        scrollback_buffer: &ScrollbackBuffer,
        row_idx: usize,
    ) -> Self {
        let history_len = scrollback_buffer.lines.len();

        // Clamp the scroll amount to the actual history length to prevent underflow
        // panics. This handles race conditions where the terminal state gets out of sync
        // with its data (e.g., the user scrolls up 1000 lines, then a background process
        // clears the terminal history down to 0 lines, but the scroll state hasn't been
        // reset yet).
        let safe_scrollback_amt = scrollback_amt.clamp_to_max(history_len).inner;

        if row_idx < safe_scrollback_amt {
            // 1. Find the "base index" in history (the absolute oldest visible line). If
            //    history has 100 lines and we scroll up by 3, the base index is 97.
            // 2. Add the current physical `row_idx` to step forward through history.
            ViewportRowMapping::History(history_len - safe_scrollback_amt + row_idx)
        } else {
            // Since the top `safe_scroll_amt` rows of the physical screen are stolen by
            // the history zone, the live zone is shifted down. We subtract that offset to
            // re-align the physical row back to index 0 of the live buffer.
            ViewportRowMapping::Live(row_idx - safe_scrollback_amt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelCharLine, ScrollbackBuffer, ScrollbackBufferLimit};

    #[test]
    fn test_map_viewport_row_with_scrolling() {
        // history_len = 100, scrollback_amt = 3
        let scrollback_amt: ScrollbackAmount = 3.into();

        let mut scrollback_buffer: ScrollbackBuffer =
            ScrollbackBufferLimit::Unlimited.into();
        for _ in 0..100 {
            scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0_u16));
        }

        // The first 3 rows of the screen (0, 1, 2) map to the last 3 rows of history.
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 0),
            ViewportRowMapping::History(97)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 1),
            ViewportRowMapping::History(98)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 2),
            ViewportRowMapping::History(99)
        );

        // Row 3 onwards maps to the live active buffer.
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 3),
            ViewportRowMapping::Live(0)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 4),
            ViewportRowMapping::Live(1)
        );
    }

    #[test]
    fn test_map_viewport_row_no_scrolling() {
        let scrollback_amt: ScrollbackAmount = 0.into();

        let mut scrollback_buffer: ScrollbackBuffer =
            ScrollbackBufferLimit::Unlimited.into();
        for _ in 0..100 {
            scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0_u16));
        }

        // All rows map directly to the live buffer.
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 0),
            ViewportRowMapping::Live(0)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 10),
            ViewportRowMapping::Live(10)
        );
    }

    #[test]
    fn test_map_viewport_row_scroll_exceeds_history() {
        // scrollback_amt = 10, but history_len = 5
        let scrollback_amt: ScrollbackAmount = 10.into();

        let mut scrollback_buffer: ScrollbackBuffer =
            ScrollbackBufferLimit::Unlimited.into();
        for _ in 0..5 {
            scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0_u16));
        }

        // Even though scrollback_amt is 10, it's clamped to history_len (5).
        // The first 5 rows (0..4) should map to history (0..4).
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 0),
            ViewportRowMapping::History(0)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 4),
            ViewportRowMapping::History(4)
        );

        // Row 5 onwards should map to live buffer, starting from 0.
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 5),
            ViewportRowMapping::Live(0)
        );
    }

    #[test]
    fn test_map_viewport_row_empty_history() {
        // scrollback_amt = 10, but history_len = 0
        let scrollback_amt: ScrollbackAmount = 10.into();

        let scrollback_buffer: ScrollbackBuffer = ScrollbackBufferLimit::Unlimited.into();

        // Clamped to 0. All rows map to Live.
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 0),
            ViewportRowMapping::Live(0)
        );
        assert_eq!(
            ViewportRowMapping::calculate(scrollback_amt, &scrollback_buffer, 5),
            ViewportRowMapping::Live(5)
        );
    }
}

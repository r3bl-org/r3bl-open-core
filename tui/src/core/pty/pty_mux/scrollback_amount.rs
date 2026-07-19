// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, CRow, CanvasToViewportExt, IndexOps, LengthOps,
            NumericConversions, NumericValue, StorageCoordinate, VPHeight, VPLength,
            VPRow, Viewport, ViewportToCanvasExt};
use std::{fmt::Debug,
          ops::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign}};

/// Represents a vertical scroll amount backed by a [`usize`] value.
///
/// For details on how the screen coordinates map to the active and history buffers during
/// scrolling, see [Mental Model & Visual Layout] in [`OutputRenderer`].
///
/// # Why [`usize`]?
///
/// Unlike [`VPRow`] and [`VPHeight`] which are backed by [`u16`] because terminal
/// screens never exceed 65,535 rows, this struct is backed by [`usize`] to safely track
/// history in scrollback buffers which can commonly hold 100,000+ lines.
///
/// ## Examples
///
/// ```rust
/// use r3bl_tui::{ScrollbackAmount, ArrayOverflowResult, ArrayBoundsCheck};
///
/// // Create from usize.
/// let amount_to_scroll: usize = 10;
/// let offset: ScrollbackAmount = amount_to_scroll.into();
///
/// // Type-safe addition with saturating bounds.
/// let new_offset = offset.saturating_add(5u16.into());
/// assert_eq!(*new_offset, 15);
///
/// // Type-safe bounds checking.
/// let history_len = 10usize;
/// assert_eq!(new_offset.overflows(history_len), ArrayOverflowResult::Overflowed);
/// assert_eq!(offset.overflows(15usize), ArrayOverflowResult::Within);
/// ```
///
/// [`ChUnit`]: crate::ChUnit
/// [`OutputRenderer`]: super::OutputRenderer
/// [`VPHeight`]: crate::VPHeight
/// [`VPRow`]: crate::VPRow
/// [Mental Model & Visual Layout]: super::OutputRenderer::render_from_active_buffer
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct ScrollbackAmount {
    inner: usize,
}

#[must_use]
pub fn scrollback_amount(amount: usize) -> ScrollbackAmount {
    ScrollbackAmount { inner: amount }
}

/// Coordinate mapping to and from Viewport <-> [`Canvas`] coordinates, taking into
/// account the vertical scrollback amount.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
impl ScrollbackAmount {
    /// Translates a row index in **Viewport Coordinates (Viewport-Relative)** to an
    /// absolute row index in **[`Canvas`] Coordinates (Canvas-Absolute)**, taking into
    /// account this vertical scrollback amount.
    ///
    /// **Stage 1: Live Bottom View (`scrollback_amt` = 0)**
    ///
    /// When `scrollback_amt` is 0, Viewport row 0 maps directly to the line
    /// immediately following `history_len`. See [`ViewportToCanvasExt::to_canvas`] for
    /// a visual diagram of how rows are laid out in the canvas.
    ///
    /// **Stage 2: Scrolled Back View (`scrollback_amt` = 2)**
    ///
    /// When `scrollback_amt` is greater than 0, the target lookup address shifts UP
    /// into history by `scrollback_amt` lines.
    ///
    /// ```text
    ///    Canvas Storage Buffer                                   Row Address Calculation
    ///   ┌─────────────────────────────────────────────────────┐
    ///  0│ (History Line 0)                                    │  ← Canvas Row 0
    ///  1│ (History Line 1)                                    │
    ///  2│ (History Line 2) ◄─── Target (Canvas Row 2)         │  ▲
    ///  3│ (History Line 3)      ▲                             │  │ history_len = 4
    ///   ├───────────────────────┼─────────────────────────────┤  ▼
    ///  4│ Viewport Row 0        │  Shift UP into history by   │  ▲
    ///  5│ Viewport Row 1        │  scrollback_amt = 2         │  │
    ///  6│ Viewport Row 2        │  from relative_row_index 0  │  │ Viewport Height = 4
    ///  7│ Viewport Row 3        │                             │  ▼
    ///   └───────────────────────┴─────────────────────────────┘  ← Viewport Bottom
    ///
    ///   Target Canvas Row = history_len (4) - scrollback_amt (2) + relative_row_index (0)
    ///                      = CRow(2)
    /// ```
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    /// [`CRow`]: crate::CRow
    /// [`Viewport`]: crate::Viewport
    /// [`ViewportToCanvasExt::to_canvas`]: crate::ViewportToCanvasExt::to_canvas
    /// [`VPRow`]: crate::VPRow
    #[must_use]
    pub fn to_c_row(&self, viewport: &Viewport, relative_row_index: VPRow) -> CRow {
        let history_len = viewport.get_history_len();
        let safe_scrollback_amt = self.clamp_to_max(history_len);
        let mut vp_copy = *viewport;
        vp_copy.set_origin_pos(|pos| pos.row_index -= *safe_scrollback_amt);
        vp_copy.to_canvas(relative_row_index)
    }

    /// Translates an absolute row index in **[`Canvas`] Coordinates (Canvas-Absolute)**
    /// to a relative row index in **Viewport Coordinates (Viewport-Relative)**,
    /// taking into account this vertical scrollback amount.
    ///
    /// This is the inverse of [`Self::to_c_row`]. See [`Self::to_c_row`]
    /// for a visual diagram of the scrolled viewport layout.
    ///
    /// Returns `Some(VPRow)` if the canvas row falls within the scrolled
    /// viewport view, or `None` if it is outside the visible window.
    ///
    /// [`Canvas`]: mod@crate::core::coordinates::canvas
    /// [`CRow`]: crate::CRow
    /// [`Viewport`]: crate::Viewport
    /// [`VPRow`]: crate::VPRow
    #[must_use]
    pub fn to_viewport_row(&self, viewport: &Viewport, c_row_idx: CRow) -> Option<VPRow> {
        let history_len = viewport.get_history_len();
        let safe_scrollback_amt = self.clamp_to_max(history_len);
        let mut vp_copy = *viewport;
        vp_copy.set_origin_pos(|pos| pos.row_index -= *safe_scrollback_amt);
        vp_copy.to_viewport(c_row_idx)
    }
}

/// Addition and subtraction operations for [`ScrollbackAmount`] with saturating bounds.
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
    pub fn clamp_to_max(&self, max_length: impl Into<ScrollbackAmount>) -> Self {
        *self.min(&max_length.into())
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_add(rhs.inner)
            .map(|inner| Self { inner })
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_sub(rhs.inner)
            .map(|inner| Self { inner })
    }
}

impl From<usize> for ScrollbackAmount {
    fn from(inner: usize) -> ScrollbackAmount { ScrollbackAmount { inner } }
}

impl From<u16> for ScrollbackAmount {
    fn from(inner: u16) -> ScrollbackAmount {
        ScrollbackAmount {
            inner: usize::from(inner),
        }
    }
}

impl From<VPHeight> for ScrollbackAmount {
    fn from(height: VPHeight) -> ScrollbackAmount {
        ScrollbackAmount {
            inner: height.as_usize(),
        }
    }
}

impl From<VPLength> for ScrollbackAmount {
    fn from(length: VPLength) -> ScrollbackAmount {
        ScrollbackAmount {
            inner: length.as_usize(),
        }
    }
}

impl Deref for ScrollbackAmount {
    type Target = usize;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for ScrollbackAmount {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl Add for ScrollbackAmount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output { self.saturating_add(rhs) }
}

impl Sub for ScrollbackAmount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output { self.saturating_sub(rhs) }
}

impl AddAssign for ScrollbackAmount {
    fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
}

impl SubAssign for ScrollbackAmount {
    fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
}

impl NumericConversions for ScrollbackAmount {
    fn as_usize(&self) -> usize { self.inner }
}

impl IndexOps for ScrollbackAmount {
    type LengthType = ScrollbackAmount;

    fn convert_to_length(&self) -> Self::LengthType {
        ScrollbackAmount {
            inner: self.inner.saturating_add(1),
        }
    }
}

impl LengthOps for ScrollbackAmount {
    type IndexType = ScrollbackAmount;

    fn convert_to_index(&self) -> Self::IndexType {
        ScrollbackAmount {
            inner: self.inner.saturating_sub(1),
        }
    }
}

impl NumericValue for ScrollbackAmount {}

impl StorageCoordinate for ScrollbackAmount {}

impl ArrayBoundsCheck<ScrollbackAmount> for ScrollbackAmount {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayOverflowResult, VPSize, Viewport, c_row, vp_height, vp_row,
                vp_width};

    fn create_test_viewport() -> Viewport {
        Viewport::from((
            crate::c_pos(0, 5),
            VPSize::new((vp_width(80), vp_height(24))),
        ))
    }

    #[test]
    fn test_scrollback_amount_constructors_and_deref() {
        let amount1 = scrollback_amount(10);
        assert_eq!(*amount1, 10);

        let mut amount2: ScrollbackAmount = 5u16.into();
        assert_eq!(*amount2, 5);

        *amount2 = 15;
        assert_eq!(*amount2, 15);
    }

    #[test]
    fn test_saturating_add_and_sub() {
        let a = scrollback_amount(10);
        let b = scrollback_amount(5);

        assert_eq!(*a.saturating_add(b), 15);
        assert_eq!(*a.saturating_sub(b), 5);
        assert_eq!(*b.saturating_sub(a), 0);
    }

    #[test]
    fn test_overflows_and_clamp() {
        let amount = scrollback_amount(10);

        assert_eq!(amount.overflows(10u16), ArrayOverflowResult::Overflowed);
        assert_eq!(amount.overflows(15u16), ArrayOverflowResult::Within);

        assert_eq!(*amount.clamp_to_max(5u16), 5);
        assert_eq!(*amount.clamp_to_max(15u16), 10);
    }

    #[test]
    fn test_to_canvas_row_and_to_viewport_row() {
        let vp = create_test_viewport(); // history_len = 5, height = 24

        // Live view (scrollback_amt = 0)
        let live = scrollback_amount(0);
        assert_eq!(live.to_c_row(&vp, vp_row(0)), c_row(5));
        assert_eq!(live.to_viewport_row(&vp, c_row(5)), Some(vp_row(0)));
        assert_eq!(live.to_viewport_row(&vp, c_row(4)), None);

        // Scrolled view (scrollback_amt = 2)
        let scrolled = scrollback_amount(2);
        assert_eq!(scrolled.to_c_row(&vp, vp_row(0)), c_row(3));
        assert_eq!(scrolled.to_viewport_row(&vp, c_row(3)), Some(vp_row(0)));
        assert_eq!(scrolled.to_viewport_row(&vp, c_row(2)), None);

        // Clamped scrollback (scrollback_amt = 10 > history_len of 5)
        let over_scrolled = scrollback_amount(10);
        assert_eq!(over_scrolled.to_c_row(&vp, vp_row(0)), c_row(0));
        assert_eq!(
            over_scrolled.to_viewport_row(&vp, c_row(0)),
            Some(vp_row(0))
        );
    }

    #[test]
    fn test_checked_add_and_sub() {
        let a = scrollback_amount(10);
        let b = scrollback_amount(5);

        assert_eq!(a.checked_add(b), Some(scrollback_amount(15)));
        assert_eq!(a.checked_sub(b), Some(scrollback_amount(5)));
        assert_eq!(b.checked_sub(a), None);
        assert_eq!(
            scrollback_amount(usize::MAX).checked_add(scrollback_amount(1)),
            None
        );
    }

    #[test]
    fn test_operators_and_assign() {
        let mut a = scrollback_amount(10);
        let b = scrollback_amount(5);

        assert_eq!(a + b, scrollback_amount(15));
        assert_eq!(a - b, scrollback_amount(5));

        a += b;
        assert_eq!(a, scrollback_amount(15));

        a -= scrollback_amount(10);
        assert_eq!(a, scrollback_amount(5));
    }

    #[test]
    fn test_numeric_value_and_index_length_ops() {
        use crate::{IndexOps, LengthOps, NumericValue};

        let zero = scrollback_amount(0);
        let non_zero = scrollback_amount(10);

        assert!(zero.is_zero());
        assert!(!non_zero.is_zero());

        assert_eq!(non_zero.convert_to_length(), scrollback_amount(11));
        assert_eq!(non_zero.convert_to_index(), scrollback_amount(9));
    }
}

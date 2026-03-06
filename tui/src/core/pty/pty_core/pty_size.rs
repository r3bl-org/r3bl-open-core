// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Size, height, width};

/// Default size value for row height, for tests.
pub const DEFAULT_ROW_HEIGHT: u16 = 24;

/// Default size value for col width, for tests.
pub const DEFAULT_COL_WIDTH: u16 = 80;

/// Default pixel width for [`portable_pty::PtySize`] conversions. We don't really use
/// this.
pub const DEFAULT_PIXEL_WIDTH: u16 = 0;

/// Default pixel height for [`portable_pty::PtySize`] conversions. We don't really use
/// this.
pub const DEFAULT_PIXEL_HEIGHT: u16 = 0;

/// Converts a [`Size`] to a [`portable_pty::PtySize`].
impl From<Size> for portable_pty::PtySize {
    fn from(it: Size) -> Self {
        Self {
            rows: it.row_height.as_u16(),
            cols: it.col_width.as_u16(),
            pixel_width: DEFAULT_PIXEL_WIDTH,
            pixel_height: DEFAULT_PIXEL_HEIGHT,
        }
    }
}

/// Marker struct for default conversion to [`portable_pty::PtySize`]. It contains the
/// default size values for [rows] and [columns], typically used in tests.
///
/// [columns]: DEFAULT_COL_WIDTH
/// [rows]: DEFAULT_ROW_HEIGHT
#[derive(Debug, Clone, Copy)]
pub struct DefaultPtySize;

impl From<DefaultPtySize> for portable_pty::PtySize {
    fn from(_: DefaultPtySize) -> Self {
        let size: Size = DefaultPtySize.into();
        size.into()
    }
}

impl From<DefaultPtySize> for Size {
    fn from(_: DefaultPtySize) -> Self {
        width(DEFAULT_COL_WIDTH) + height(DEFAULT_ROW_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pty_size_conversion() {
        let default_size: portable_pty::PtySize = DefaultPtySize.into();
        assert_eq!(default_size.rows, DEFAULT_ROW_HEIGHT);
        assert_eq!(default_size.cols, DEFAULT_COL_WIDTH);
    }
}

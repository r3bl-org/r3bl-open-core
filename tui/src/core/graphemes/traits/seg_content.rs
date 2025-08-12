// Copyright (c) 2024 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChUnit, ColIndex, ColWidth, Seg};

/// Core segment content reference for zero-copy access to grapheme cluster segments.
///
/// This struct provides a unified way to access segment content and metadata
/// without copying the underlying string data. The lifetime parameter `'a` represents
/// the lifetime of the borrowed string content, ensuring that the `SegContent`
/// cannot outlive the string it references.
#[derive(Debug, Clone, Copy)]
pub struct SegContent<'a> {
    /// The actual string content of the segment
    pub content: &'a str,
    /// The segment metadata
    pub seg: Seg,
}

impl SegContent<'_> {
    /// Get the string content of this segment
    #[must_use]
    pub fn as_str(&self) -> &str { self.content }

    /// Get the display width of this segment
    #[must_use]
    pub fn width(&self) -> ColWidth { self.seg.display_width }

    /// Get the starting column index of this segment
    #[must_use]
    pub fn start_col(&self) -> ColIndex { self.seg.start_display_col_index }

    /// Get a reference to the underlying segment metadata
    #[must_use]
    pub fn seg(&self) -> &Seg { &self.seg }

    /// Get the byte range of this segment within the original string
    #[must_use]
    pub fn byte_range(&self) -> std::ops::Range<ChUnit> {
        self.seg.start_byte_index..self.seg.end_byte_index
    }
}

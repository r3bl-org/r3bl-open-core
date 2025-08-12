// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cache for dialog border strings to avoid repeated allocation and `GCStringOwned`
//! creation.
//!
//! ## Performance Impact
//!
//! The flamegraph analysis shows 53M samples in `render_border_lines` →
//! `lolcat_from_style` → `GCStringOwned::new` for dialog borders. This cache eliminates
//! the repeated string allocation and unicode segmentation overhead by caching pre-built
//! border strings for common sizes.

use std::{hash::{Hash, Hasher},
          sync::LazyLock};

use crate::{BorderGlyphCharacter, ColWidth, InlineString, ch, usize};

/// Key for caching border strings.
#[derive(Clone, PartialEq, Eq, Debug)]
struct BorderCacheKey {
    line_type: BorderLineType,
    width: ColWidth,
}

impl Hash for BorderCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.line_type.hash(state);
        self.width.0.hash(state);
    }
}

/// Types of border lines that can be cached.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum BorderLineType {
    Top,
    Middle,
    Bottom,
    Separator,
}

/// Global cache for border strings.
///
/// Since dialog widths typically range from 20-200 columns and there are only
/// 4 line types, this cache will have at most ~800 entries in practice.
static BORDER_CACHE: LazyLock<crate::ThreadSafeLruCache<BorderCacheKey, InlineString>> =
    LazyLock::new(|| crate::new_threadsafe_lru_cache(1000));

/// Get a cached top border line or create and cache it.
pub fn get_top_border_line(bounds_width: ColWidth) -> InlineString {
    let key = BorderCacheKey {
        line_type: BorderLineType::Top,
        width: bounds_width,
    };

    if let Ok(mut cache) = BORDER_CACHE.lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let text = create_top_border_line(bounds_width);
        cache.insert(key, text.clone());
        text
    } else {
        // Fallback if cache lock fails
        create_top_border_line(bounds_width)
    }
}

/// Get a cached middle border line or create and cache it.
pub fn get_middle_border_line(bounds_width: ColWidth) -> InlineString {
    let key = BorderCacheKey {
        line_type: BorderLineType::Middle,
        width: bounds_width,
    };

    if let Ok(mut cache) = BORDER_CACHE.lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let text = create_middle_border_line(bounds_width);
        cache.insert(key, text.clone());
        text
    } else {
        // Fallback if cache lock fails
        create_middle_border_line(bounds_width)
    }
}

/// Get a cached bottom border line or create and cache it.
pub fn get_bottom_border_line(bounds_width: ColWidth) -> InlineString {
    let key = BorderCacheKey {
        line_type: BorderLineType::Bottom,
        width: bounds_width,
    };

    if let Ok(mut cache) = BORDER_CACHE.lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let text = create_bottom_border_line(bounds_width);
        cache.insert(key, text.clone());
        text
    } else {
        // Fallback if cache lock fails
        create_bottom_border_line(bounds_width)
    }
}

/// Get a cached separator line or create and cache it.
pub fn get_separator_line(bounds_width: ColWidth) -> InlineString {
    let key = BorderCacheKey {
        line_type: BorderLineType::Separator,
        width: bounds_width,
    };

    if let Ok(mut cache) = BORDER_CACHE.lock() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let text = create_separator_line(bounds_width);
        cache.insert(key, text.clone());
        text
    } else {
        // Fallback if cache lock fails
        create_separator_line(bounds_width)
    }
}

// Helper functions to create border strings

fn create_top_border_line(bounds_width: ColWidth) -> InlineString {
    let inner_width = usize(*bounds_width - ch(2));
    let inner_line = BorderGlyphCharacter::Horizontal
        .as_ref()
        .repeat(inner_width);

    InlineString::from(format!(
        "{}{}{}",
        BorderGlyphCharacter::TopLeft.as_ref(),
        inner_line,
        BorderGlyphCharacter::TopRight.as_ref()
    ))
}

fn create_middle_border_line(bounds_width: ColWidth) -> InlineString {
    let inner_width = usize(*bounds_width - ch(2));
    let inner_spaces = crate::glyphs::SPACER_GLYPH.repeat(inner_width);

    InlineString::from(format!(
        "{}{}{}",
        BorderGlyphCharacter::Vertical.as_ref(),
        inner_spaces,
        BorderGlyphCharacter::Vertical.as_ref()
    ))
}

fn create_bottom_border_line(bounds_width: ColWidth) -> InlineString {
    let inner_width = usize(*bounds_width - ch(2));
    let inner_line = BorderGlyphCharacter::Horizontal
        .as_ref()
        .repeat(inner_width);

    InlineString::from(format!(
        "{}{}{}",
        BorderGlyphCharacter::BottomLeft.as_ref(),
        inner_line,
        BorderGlyphCharacter::BottomRight.as_ref()
    ))
}

fn create_separator_line(bounds_width: ColWidth) -> InlineString {
    let inner_width = usize(*bounds_width - ch(2));
    let inner_line = BorderGlyphCharacter::Horizontal
        .as_ref()
        .repeat(inner_width);

    InlineString::from(format!(
        "{}{}{}",
        BorderGlyphCharacter::LineUpDownRight.as_ref(),
        inner_line,
        BorderGlyphCharacter::LineUpDownLeft.as_ref()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::width;

    #[test]
    fn test_border_cache_basic() {
        // Test top border
        let test_width = width(10);
        let border1 = get_top_border_line(test_width);
        let border2 = get_top_border_line(test_width);
        assert_eq!(border1, border2);

        // Test middle border
        let middle1 = get_middle_border_line(test_width);
        let middle2 = get_middle_border_line(test_width);
        assert_eq!(middle1, middle2);

        // Test different widths produce different results
        let border_narrow = get_top_border_line(width(5));
        let border_wide = get_top_border_line(width(20));
        assert_ne!(border_narrow, border_wide);
    }

    #[test]
    fn test_border_content() {
        let test_width = width(6);
        let top = get_top_border_line(test_width);
        assert!(top.contains(BorderGlyphCharacter::TopLeft.as_ref()));
        assert!(top.contains(BorderGlyphCharacter::TopRight.as_ref()));
        assert!(top.contains(BorderGlyphCharacter::Horizontal.as_ref()));

        let middle = get_middle_border_line(test_width);
        assert!(middle.contains(BorderGlyphCharacter::Vertical.as_ref()));
        assert!(middle.contains(crate::glyphs::SPACER_GLYPH));
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{GetMemSize, MemorySize, Flat2DArray, PixelChar, Pos, Size};

/// Internal offscreen buffer state for the alternate screen.
///
/// This represents the *internal state* of the engine, tracking whether the grid
/// buffers are currently swapped.
///
/// For the external VT100 mode requested by the child process, see
/// [`RequestedScreenMode`].
///
/// [`RequestedScreenMode`]: crate::RequestedScreenMode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveScreenBuffer {
    /// Alternate screen buffer is actively displayed.
    Alternate,

    /// Alternate screen buffer is inactive, primary screen is displayed.
    #[default]
    Primary,
}

/// Encapsulated state representing the alternate screen support and its independent
/// cursor.
///
/// This struct manages the hidden buffer (which is either the primary or alternate
/// screen depending on the active state) and the cursor position associated with it.
///
/// # Initialization
///
/// The hidden buffer is eagerly allocated at creation time ([`new_empty()`]). This
/// avoids the complexity and overhead of `Option`-wrapping the buffer, trading a small,
/// fixed memory cost for cleaner, faster O(1) access.
///
/// # [`SGR`] Style Inheritance & [`BCE`] (Background Color Erase) Compliance
///
/// Under [`VT-100`], [`xterm`], and standard [`ANSI`] specifications, graphic rendition
/// states (foreground/background colors, bold, italic) are globally shared and preserved
/// across screen buffer switches.
///
/// This struct implements this specification by separating the alternate buffer
/// ([`hidden_buffer`]) from the overall parser emulation state. When entering the
/// alternate screen:
/// - The alternate buffer inherits the active graphic style from the main buffer.
/// - The buffer is cleared utilizing [`create_empty_pixel_char()`] to ensure that erased
///   cells carry the active background color and attributes, fully complying with
///   Background Color Erase ([`BCE`]) rules.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`BCE`]: https://invisible-island.net/xterm/xterm.faq.html#what_is_bce
/// [`create_empty_pixel_char()`]: crate::OfsBufVT100::create_empty_pixel_char
/// [`hidden_buffer`]: HiddenScreenState::hidden_buffer
/// [`new_empty()`]: crate::HiddenScreenState::new_empty
/// [`SGR`]: crate::SgrCode
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenScreenState {
    /// The secondary buffer grid representing the alternate screen. Always allocated at
    /// buffer creation time to avoid having to use [`Option`] which adds needless
    /// complexity for a very small cost in terms of memory.
    pub hidden_buffer: Flat2DArray<PixelChar>,

    /// Saved cursor position for the hidden buffer.
    pub hidden_cursor_pos: Pos,

    /// Cached memory size of this struct to provide O(1) retrieval.
    pub cached_memory_size: MemorySize,
}

impl HiddenScreenState {
    #[must_use]
    pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
        let window_size = arg_window_size.into();
        let hidden_buffer = Flat2DArray::new_empty(window_size, PixelChar::Spacer); // window_size);

        let cached_memory_size = {
            let primary_buffer_mem = hidden_buffer.get_mem_size();
            let hidden_cursor_pos = size_of::<Pos>();
            MemorySize::new(primary_buffer_mem + hidden_cursor_pos)
        };

        Self {
            hidden_buffer,
            hidden_cursor_pos: Pos::default(),
            cached_memory_size,
        }
    }
}

impl GetMemSize for HiddenScreenState {
    /// Acts as a fast O(1) getter for the cached memory size to avoid O(N) traversal
    /// of the entire alternate screen buffer grid.
    fn get_mem_size(&self) -> usize { self.cached_memory_size.size().unwrap_or(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RequestedScreenMode;

    #[test]
    fn test_hidden_screen_state_struct_size() {
        assert_eq!(size_of::<ActiveScreenBuffer>(), 1);
        assert_eq!(size_of::<RequestedScreenMode>(), 1);

        // TRIPWIRE: If you add or remove a field from `HiddenScreenState`, this test will
        // fail. This is intentional! It reminds you to:
        // 1. Update the `GetMemSize` implementation for this struct to include your new
        //    field.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<HiddenScreenState>(), 48);
        }
    }

    #[test]
    fn test_hidden_screen_state_get_mem_size() {
        // TRIPWIRE: This test verifies that `GetMemSize` actually sums up all the fields.
        // If you added a field, you MUST add its memory size calculation to
        // `expected_size` below, and ensure the actual `GetMemSize`
        // implementation matches it.
        use crate::{height, width};
        let size = height(10) + width(20);
        let support = HiddenScreenState::new_empty(size);

        let calculated_size = support.get_mem_size();
        let expected_size = support.hidden_buffer.get_mem_size() + size_of::<Pos>();

        assert_eq!(calculated_size, expected_size);
        // Ensure consistency across calls
        assert_eq!(calculated_size, support.get_mem_size());
    }
}

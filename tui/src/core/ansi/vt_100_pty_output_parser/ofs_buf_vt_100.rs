// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{PtyResponseEvent, GetMemSize, MemorySize, OffscreenBuffer,
            PixelCharLines, Pos, Size, TermRow, TuiStyle, osc::OscEvent};
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

/// Encapsulated runtime state tracking active graphic renditions, terminal attributes,
/// and protocol requests for [`ANSI`] sequence parsing.
///
/// This struct maintains the global terminal states that persist across individual parser
/// sequences and must be inherited when transitioning between different screen buffers
/// (e.g., from the primary grid to the alternate grid).
///
/// It acts as the central holding area for:
/// - Graphic renditions ([`SGR`]) such as active foreground/background colors, text
///   styling (bold, italic, etc.), ensuring [`BCE`] (Background Color Erase) compliance.
/// - The active character set mapping (e.g., standard [`ASCII`] vs. [`DEC`] Line
///   Drawing).
/// - Cursor visibility states toggled via escape codes.
/// - Pending Device Status Report ([`DSR`]) requests that need to be flushed back to the
///   [`PTY`].
///
/// ## Primary and Alt Screen State Management
///
/// This struct manages global terminal settings. Why is the active cursor position
/// intentionally omitted here?
///
/// Under the [`VT-100`]/[`xterm`] spec, when switching to the alternate screen:
/// - **Global state shared between primary and alternate screens**: Graphic renditions
///   (colors, bold, etc.), active character sets (`ESC ( B`), and cursor visibility
///   settings persist across screen switches. They belong to the terminal's global state
///   machine, not to a specific text grid. That's why they are stored here in
///   [`ParserGlobalState`].
/// - **Screen (primary or alternate) specific state**: The active cursor position (`row`,
///   `col`) is not shared. The primary screen and alternate screen maintain entirely
///   independent cursors.
///   - To keep primary and alternate screen drawing logic simple, we use
///     [`std::mem::swap()`] in [`set_alt_screen_mode()`] to physically swap the screen's
///     underlying [`OffscreenBuffer`] data when switching to another screen.
///   - Because of this, the [`OffscreenBuffer`] and its [`OffscreenBuffer::cursor_pos`]
///     **always** represent the currently visible screen (regardless of whether it is
///     primary or alternate).
///   - The hidden screen's data and its specific cursor position are parked in
///     [`HiddenScreenState`] (in its [`hidden_buffer`] and [`hidden_cursor_pos`]) until
///     they are swapped back when the screen switches.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`BCE`]:
///     https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
/// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
/// [`DSR`]: crate::PtyResponseEvent
/// [`hidden_buffer`]: crate::HiddenScreenState::hidden_buffer
/// [`hidden_cursor_pos`]: crate::HiddenScreenState::hidden_cursor_pos
/// [`OffscreenBuffer::cursor_pos`]: crate::OffscreenBuffer::cursor_pos
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`set_alt_screen_mode()`]: crate::OfsBufVT100::set_alt_screen_mode
/// [`SGR`]: crate::SgrCode
/// [`std::mem::swap()`]: std::mem::swap
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParserGlobalState {
    /// Temporary cursor position storage for [`DECSC`]/[`DECRC`] escape sequences only.
    ///
    /// This field is ONLY used for `ESC 7` ([`DECSC`]) save and `ESC 8` ([`DECRC`])
    /// restore operations, as well as their [`CSI`] equivalents (`CSI s` and `CSI
    /// u`). It does NOT track the current cursor position - that's stored in
    /// [`OffscreenBuffer::cursor_pos`].
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement the [`DECSC`] (`ESC 7`) and
    /// [`DECRC`] (`ESC 8`) escape sequences for saving and restoring cursor
    /// position.
    ///
    /// ## Data Flow:
    /// ```text
    /// 1. Child process (e.g., vim) sends ESC 7 to save cursor
    ///                             ↓
    /// 2. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 7
    ///                             ↓
    /// 3. Saves current cursor_pos to buffer.parser_global_state.
    ///     `cursor_pos_for_esc_save_and_restore`.
    ///                             ↓
    /// 4. Later, child sends ESC 8 to restore cursor
    ///                             ↓
    /// 5. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 8
    ///                             ↓
    /// 6. Restores cursor_pos from buffer.parser_global_state.cursor_pos_for_esc_save_and_restore
    /// ```
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::core::ansi::vt_100_pty_output_parser::AnsiToOfsBufPerformer
    /// [`CSI`]: crate::CsiSequence
    /// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
    /// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
    pub cursor_pos_for_esc_save_and_restore: Option<Pos>,

    /// Active character set for [`ANSI`] escape sequence support.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement character set switching via the
    /// sequences:
    /// - `ESC ( B` for [`ASCII`].
    /// - `ESC ( 0` for [`DEC`] graphics.
    ///
    /// When in Graphics mode, characters like 'q' are translated to box-drawing
    /// characters like '─' during the [`print()`] operation.
    ///
    /// ## Character Set Usage:
    /// ```text
    /// ASCII Mode    : ESC ( B  : 'q' → 'q' (literal)
    /// Graphics Mode : ESC ( 0  : 'q' → '─' (horizontal line)
    /// ```
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`ESC`]: crate::EscSequence
    /// [`print()`]: vte::Perform::print
    pub character_set: CharacterSet,

    /// Auto-wrap mode ([`DECAWM`]) for [`ANSI`] escape sequence support.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to control line wrapping behavior when printing
    /// characters. This implements the [`VT-100`] [`DECAWM`] (Auto Wrap Mode)
    /// specification.
    ///
    /// ## [`DECAWM`] Control:
    ///
    /// ```text
    /// ESC[ ? 7h : Enable auto-wrap (default)  - Characters wrap to next line
    /// ESC[ ? 7l : Disable auto-wrap           - Characters overwrite at right margin
    /// ```
    ///
    /// When enabled (default), characters that would exceed the right margin
    /// automatically wrap to the beginning of the next line. When disabled, the cursor
    /// stays at the right margin and subsequent characters overwrite.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub auto_wrap_mode: AutoWrapState,

    /// Currently active [`SGR`] (Select Graphic Rendition) text formatting.
    ///
    /// Accumulates text attributes (bold, italic, etc.) and colors from `ESC [ ... m`
    /// sequences. These styles are stamped onto all subsequent characters printed to the
    /// buffer.
    ///
    /// [`SGR`]: crate::SgrCode
    pub current_style: TuiStyle,

    /// [`OSC`] events (hyperlinks, titles, etc.) accumulated during processing.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub pending_osc_events: Vec<OscEvent>,

    /// [`DSR`] response events accumulated during processing - need to be sent back to
    /// [`PTY`].
    ///
    /// [`DSR`]: crate::DsrSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub pending_pty_response_events: Vec<PtyResponseEvent>,

    /// Top margin for the **scrollable region** ([`DECSTBM`]) - 1-based row number.
    ///
    /// This variable defines the **upper boundary** of the area where scrolling occurs.
    /// Rows above this boundary are part of the **static top region** and do not scroll.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement [`DECSTBM`] (Set Top and Bottom
    /// Margins) functionality via the sequence: `ESC [ <top> ; <bottom> r`.
    ///
    /// When [`None`], the default top margin is row 1 (first row), making the entire
    /// terminal screen the scrollable region. When [`Some(n)`], scrolling operations
    /// only affect rows from n to [`scroll_region_bottom`].
    ///
    /// ## [`DECSTBM`] Usage:
    /// ```text
    /// ESC [ 5 ; 20 r   - Set scrolling region from row 5 to row 20
    /// ESC [ r          - Reset to full screen (clears both margins)
    /// ```
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`ESC`]: crate::EscSequence
    /// [`scroll_region_bottom`]: Self::scroll_region_bottom
    /// [`Some(n)`]: Some
    pub scroll_region_top: Option<TermRow>,

    /// Bottom margin for the **scrollable region** ([`DECSTBM`]) - 1-based row number.
    ///
    /// This variable defines the **lower boundary** of the area where scrolling occurs.
    /// Rows below this boundary are part of the **static bottom region** and do not
    /// scroll.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement [`DECSTBM`] (Set Top and Bottom
    /// Margins) functionality via the sequence: `ESC [ <top> ; <bottom> r`.
    ///
    /// When [`None`], the default bottom margin is the last row of the terminal, making
    /// the entire terminal screen the scrollable region. When [`Some(n)`], scrolling
    /// operations only affect rows from [`scroll_region_top`] to n.
    ///
    /// ## [`DECSTBM`] Behavior:
    /// - Scrolling commands ([`ESC`] D, [`ESC`] M, [`CSI`] S, [`CSI`] T) only affect the
    ///   region
    /// - Cursor movement is constrained to the region boundaries
    /// - Content outside the region remains unchanged during scrolling
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`CSI`]: crate::CsiSequence
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`ESC`]: crate::EscSequence
    /// [`scroll_region_top`]: Self::scroll_region_top
    /// [`Some(n)`]: Some
    pub scroll_region_bottom: Option<TermRow>,

    /// Controls whether the terminal cursor is visible.
    ///
    /// Corresponds to the [`DECTCEM`] (`?25`) private mode.
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    pub cursor_visibility: CursorVisibilityState,
}

/// Character set modes for terminal emulation.
///
/// Used by [`AnsiToOfsBufPerformer`] to handle `ESC ( <char>` sequences that switch
/// between [`ASCII`] and [`DEC`] line-drawing graphics.
///
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`ESC`]: crate::EscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterSet {
    /// Normal [`ASCII`] character set:
    /// - `ESC ( B`
    /// - or `[27, 40, 66]` in decimal
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`ESC`]: crate::EscSequence
    #[default]
    Ascii,
    /// [`DEC`] Special Graphics character set for line drawing:
    /// - `ESC ( 0`
    /// - or `[27, 40, 48]` in decimal
    ///
    /// Maps [`ASCII`] characters to box-drawing Unicode characters.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`ESC`]: crate::EscSequence
    DECGraphics,
}

/// Auto-wrap mode ([`DECAWM`]) state.
///
/// Controls line wrapping behavior when text reaches the right margin.
///
/// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoWrapState {
    /// Characters automatically wrap to the next line (DECAWM `?7h`)
    #[default]
    Enabled,
    /// Characters overwrite at the right margin (DECAWM `?7l`)
    Disabled,
}

/// Terminal cursor visibility state.
///
/// Controls whether the terminal cursor is displayed or hidden. Corresponds to the
/// [`DECTCEM`] (`?25`) private mode.
///
/// # Usage in [`PTY`] Mux
///
/// When used inside [`ParserGlobalState::cursor_visibility`], it stores the *requested*
/// visibility state of the child process. The [`PTY Mux`] compositor
/// ([`OutputRenderer::composite_virtual_cursor_into_buffer`]) reads this to determine if
/// it needs to paint a simulated, virtual block cursor into the [`OffscreenBuffer`].
///
/// > Note: The host terminal emulator's actual cursor is permanently suppressed via
/// > [`hide_cursor`] when the multiplexer is active. We rely exclusively
/// > on the virtual block cursor rendering (which allows us to have multiple cursors).
///
/// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
/// [`hide_cursor`]: crate::TerminalModeController::hide_cursor
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
/// [`OutputRenderer::composite_virtual_cursor_into_buffer`]:
///     crate::core::pty::OutputRenderer::composite_virtual_cursor_into_buffer
/// [`ParserGlobalState::cursor_visibility`]: crate::ParserGlobalState::cursor_visibility
/// [`PTY Mux`]: crate::PTYMux
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorVisibilityState {
    /// Cursor is visible ([`DECTCEM`] `ESC [ ? 25 h`)
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    #[default]
    Visible,
    /// Cursor is hidden ([`DECTCEM`] `ESC [ ? 25 l`)
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    Hidden,
}

/// State tracking for terminal operational modes.
///
/// Used by the [`VT-100`] [`ANSI`] parser performer ([`AnsiToOfsBufPerformer`])
/// to maintain state information about the operational modes requested by the
/// underlying [`PTY`] process.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalModeState {
    /// Alternate screen buffer status.
    ///
    /// When active, terminal output is redirected to an alternate screen buffer,
    /// preserving the original screen content.
    ///
    /// Toggled by the [`AnsiToOfsBufPerformer`] when processing the `ESC [ ? 1049 h`
    /// and `ESC [ ? 1049 l` sequences.
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    pub alternate_screen: AlternateScreenState,

    /// Mouse event tracking status.
    ///
    /// **TODO**: The parser currently ignores this [`VT-100`] sequence
    /// (`vt_100_shim_mode_ops.rs`) because the [`PTY`] multiplexer does not yet route
    /// complex input events. When supported, this field should be wired up to the
    /// [`ANSI`] parser and the `dead_code` allowance removed.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[allow(dead_code)]
    pub mouse_tracking: terminal_mode_state_todo::MouseTrackingState,

    /// Bracketed paste mode status.
    ///
    /// **TODO**: The parser currently ignores this [`VT-100`] sequence
    /// (`vt_100_shim_mode_ops.rs`) because the [`PTY`] multiplexer does not yet route
    /// complex input events. When supported, this field should be wired up to the
    /// [`ANSI`] parser and the `dead_code` allowance removed.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[allow(dead_code)]
    pub bracketed_paste: terminal_mode_state_todo::BracketedPasteState,

    /// Synchronized output mode (DEC private mode 2026).
    ///
    /// When enabled, the terminal should defer rendering until the mode is
    /// reset, allowing atomic screen updates.
    pub synchronized_output: bool,
}

mod terminal_mode_state_todo {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Mouse event tracking state.
    ///
    /// Controls whether the terminal captures mouse click, movement, and scroll events.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[allow(dead_code)]
    pub enum MouseTrackingState {
        /// Mouse tracking enabled - terminal sends mouse events
        Enabled,
        /// Mouse tracking disabled
        #[default]
        Disabled,
    }

    /// Bracketed paste mode state.
    ///
    /// Controls whether text pasted from clipboard is wrapped with special escape
    /// sequences (`OSC 52`), allowing applications to distinguish pasted text from
    /// keyboard input.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[allow(dead_code)]
    pub enum BracketedPasteState {
        /// Bracketed paste mode enabled
        Enabled,
        /// Bracketed paste mode disabled
        #[default]
        Disabled,
    }
}

/// Alternate screen buffer state.
///
/// Controls whether terminal output is redirected to an alternate screen buffer,
/// preserving the original screen content. This is used by full-screen applications
/// (`vim`, `less`, etc.) to avoid cluttering the shell history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlternateScreenState {
    /// Alternate screen buffer active
    Active,
    /// Alternate screen buffer inactive, using primary screen
    #[default]
    Inactive,
}

/// The requested screen buffer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedScreenMode {
    Primary,
    Alternate,
}

/// Encapsulated state representing the alternate screen support and its independent
/// cursor positions.
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
/// [`SGR`]: crate::SgrCode
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenScreenState {
    /// The secondary buffer grid representing the alternate screen. Always allocated at
    /// buffer creation time to avoid having to use [`Option`] which adds needless
    /// complexity for a very small cost in terms of memory.
    pub hidden_buffer: PixelCharLines,

    /// Saved cursor position for the hidden buffer.
    pub hidden_cursor_pos: Pos,

    /// Cached memory size of this struct to provide O(1) retrieval.
    pub cached_memory_size: MemorySize,
}

mod hidden_screen_state_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl HiddenScreenState {
        #[must_use]
        pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
            let window_size = arg_window_size.into();
            let hidden_buffer = PixelCharLines::new_empty(window_size);

            let cached_memory_size = {
                let primary_buffer_mem = hidden_buffer.get_mem_size();

                MemorySize::new(
                    primary_buffer_mem +
                // hidden_cursor_pos
                size_of::<Pos>(),
                )
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
    fn test_hidden_screen_state_struct_size() {
        // TRIPWIRE: If you add or remove a field from `HiddenScreenState`, this test will
        // fail. This is intentional! It reminds you to:
        // 1. Update the `GetMemSize` implementation for this struct to include your new
        //    field.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<HiddenScreenState>(), 416);
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

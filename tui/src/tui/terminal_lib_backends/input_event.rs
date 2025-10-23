// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use super::{KeyPress, MouseInput};
use crate::{Size, height, width};
use crossterm::event::{Event as CTEvent,
                       Event::{self},
                       KeyEvent, MouseEvent};
/// Unified input event abstraction for the TUI framework.
///
/// This enum represents all possible input events that can occur in a terminal
/// application. It provides a clean, unified interface for handling various types of user
/// input and terminal events, abstracting away the complexities of the underlying
/// terminal backend.
///
/// # Architecture: Event Abstraction Layers
///
/// The TUI framework uses a layered architecture to process terminal events:
///
/// 1. **Crossterm Event** (Raw terminal events)
///    - Platform-specific events with all their quirks
///    - May include Release/Repeat events on some platforms
///    - API tied to crossterm version
///
/// 2. **Specialized Types** (Clean abstractions)
///    - [`KeyPress`]: Normalized keyboard input
///    - [`MouseInput`]: Standardized mouse events
///    - [`Size`]: Terminal dimensions
///    - [`FocusEvent`]: Window focus changes
///
/// 3. **`InputEvent`** (This enum - unified interface)
///    - Single type for all input handling
///    - Each variant wraps a specialized type
///    - Enables polymorphic event processing
///
/// # Conversion Flow
///
/// ```text
/// crossterm::Event (raw events)
///     ├─→ Event::Key(KeyEvent)     → KeyPress      → InputEvent::Keyboard
///     ├─→ Event::Mouse(MouseEvent) → MouseInput    → InputEvent::Mouse
///     ├─→ Event::Resize(w, h)      → Size          → InputEvent::Resize
///     ├─→ Event::Focus*            → FocusEvent    → InputEvent::Focus
///     └─→ Event::Paste(String)     → String        → InputEvent::BracketedPaste
/// ```
///
/// # Paste Handling
///
/// The framework supports two distinct paste mechanisms:
/// - **Bracketed Paste**: Terminal-native paste (right-click, middle-click) arrives as
///   [`InputEvent::BracketedPaste`]
/// - **Clipboard Paste**: Ctrl+V arrives as [`InputEvent::Keyboard`] and requires
///   application to read from system clipboard
///
/// See [`InputEvent::BracketedPaste`] for detailed comparison.
///
/// This design ensures:
/// - **Backend independence**: Easy to swap terminal backends
/// - **Cross-platform consistency**: Same behavior on all platforms
/// - **Type safety**: Each event type is properly structured
/// - **Future-proofing**: Internal changes don't affect app code
///
/// Please see [`KeyPress`] for more information about handling keyboard input.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Keyboard(KeyPress),
    Resize(Size),
    Mouse(MouseInput),
    Focus(FocusEvent),
    /// Text pasted via terminal's paste mechanism (not Ctrl+V).
    ///
    /// # Bracketed Paste vs Clipboard Paste
    ///
    /// There are two distinct ways text can be pasted in a terminal application:
    ///
    /// ## 1. Bracketed Paste (this event)
    /// - **How to trigger**:
    ///   - Ctrl+Shift+V (some terminals)
    ///   - Right-click in terminal
    ///   - Middle mouse button click
    ///   - Shift+Insert (some terminals)
    ///   - Terminal menu → Edit → Paste
    ///   - Cmd+V on macOS Terminal
    /// - **How it works**: Terminal sends the pasted text wrapped in special escape
    ///   sequences (`ESC[200~` text `ESC[201~`)
    /// - **Characteristics**: Text arrives as a single chunk, not individual keystrokes
    ///
    /// ## 2. Clipboard Paste (Ctrl+V)
    /// - **How to trigger**: Ctrl+V key combination
    /// - **How it works**: Arrives as `InputEvent::Keyboard` with Ctrl+V, application
    ///   must read from system clipboard
    /// - **Characteristics**: Requires clipboard access permissions
    ///
    /// ## Visual Difference
    /// - **Bracketed paste**: Text appears instantly as one operation
    /// - **Ctrl+V paste**: May have slight delay while reading clipboard
    ///
    /// ## Why Two Mechanisms?
    /// - **Bracketed paste**: Terminal-native, works even without clipboard access
    /// - **Ctrl+V**: Application-controlled, consistent with desktop applications
    ///
    /// Note: Bracketed paste must be enabled via
    /// [`EnableBracketedPaste`](crate::PaintRenderOpImplCrossterm::raw_mode_enter) in raw
    /// mode.
    BracketedPaste(String),
}

/// Represents terminal window focus state changes.
///
/// Focus events are triggered when the terminal application gains or loses focus,
/// which can be useful for pausing/resuming operations or updating the UI state.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::FocusEvent;
///
/// let focus_event = FocusEvent::Gained;
/// match focus_event {
///     FocusEvent::Gained => println!("Application has focus"),
///     FocusEvent::Lost => println!("Application lost focus"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// The terminal application has gained focus (user switched to it).
    Gained,
    /// The terminal application has lost focus (user switched away).
    Lost,
}

mod helpers {
    use super::{InputEvent, KeyPress};

    impl InputEvent {
        #[must_use]
        pub fn matches_keypress(&self, other: KeyPress) -> bool {
            if let InputEvent::Keyboard(this) = self
                && this == &other
            {
                return true;
            }
            false
        }

        #[must_use]
        pub fn matches_any_of_these_keypresses(&self, others: &[KeyPress]) -> bool {
            for other in others {
                if self.matches_keypress(*other) {
                    return true;
                }
            }
            false
        }
    }

    impl InputEvent {
        /// Checks to see whether the `input_event` matches any of the `exit_keys`.
        /// Returns `true` if it does and `false` otherwise.
        #[must_use]
        pub fn matches(&self, exit_keys: &[InputEvent]) -> bool {
            for exit_key in exit_keys {
                if self == exit_key {
                    return true;
                }
            }
            false
        }
    }
}

pub(crate) mod converters {
    use super::{CTEvent, Event, FocusEvent, InputEvent, KeyEvent, MouseEvent, height,
                width};

    impl TryFrom<Event> for InputEvent {
        type Error = ();
        /// Typecast / convert [Event] to [`InputEvent`].
        ///
        /// This function routes crossterm events to their appropriate `InputEvent`
        /// variants. Each specific converter (`KeyPress`, `MouseInput`, etc.) is
        /// responsible for its own validation and filtering logic.
        fn try_from(event: Event) -> Result<Self, Self::Error> {
            match event {
                CTEvent::Key(key_event) => Ok(key_event.try_into()?),
                CTEvent::Mouse(mouse_event) => Ok(mouse_event.into()),
                CTEvent::Resize(columns, rows) => {
                    Ok(InputEvent::Resize(width(columns) + height(rows)))
                }
                CTEvent::FocusGained => Ok(InputEvent::Focus(FocusEvent::Gained)),
                CTEvent::FocusLost => Ok(InputEvent::Focus(FocusEvent::Lost)),
                CTEvent::Paste(text) => Ok(InputEvent::BracketedPaste(text)),
            }
        }
    }

    impl From<MouseEvent> for InputEvent {
        /// Typecast / convert [`MouseEvent`] to
        /// [`InputEvent::Mouse`].
        fn from(mouse_event: MouseEvent) -> Self { InputEvent::Mouse(mouse_event.into()) }
    }

    impl TryFrom<KeyEvent> for InputEvent {
        type Error = ();

        fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
            Ok(InputEvent::Keyboard(key_event.try_into()?))
        }
    }
}

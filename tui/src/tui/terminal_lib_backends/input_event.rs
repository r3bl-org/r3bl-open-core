/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */
use crossterm::event::{Event::{self},
                       KeyEvent,
                       MouseEvent};

use super::{KeyPress, MouseInput};
use crate::{height, width, Size};

/// Please see [`KeyPress`] for more information about handling keyboard input.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum InputEvent {
    Keyboard(KeyPress),
    Resize(Size),
    Mouse(MouseInput),
    Focus(FocusEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    Gained,
    Lost,
}

mod helpers {
    use super::{InputEvent, KeyPress};

    impl InputEvent {
        #[must_use]
        pub fn matches_keypress(&self, other: KeyPress) -> bool {
            if let InputEvent::Keyboard(this) = self {
                if this == &other {
                    return true;
                }
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
    use super::{Event, InputEvent, width, height, FocusEvent, MouseEvent, KeyEvent};

    impl TryFrom<Event> for InputEvent {
        type Error = ();
        /// Typecast / convert [Event] to [`InputEvent`].
        fn try_from(event: Event) -> Result<Self, Self::Error> {
            use crossterm::event::Event as CTEvent;
            match event {
                CTEvent::Key(key_event) => Ok(key_event.try_into()?),
                CTEvent::Mouse(mouse_event) => Ok(mouse_event.into()),
                CTEvent::Resize(columns, rows) => {
                    Ok(InputEvent::Resize(width(columns) + height(rows)))
                }
                CTEvent::FocusGained => Ok(InputEvent::Focus(FocusEvent::Gained)),
                CTEvent::FocusLost => Ok(InputEvent::Focus(FocusEvent::Lost)),
                CTEvent::Paste(_) => Err(()),
            }
        }
    }

    impl From<MouseEvent> for InputEvent {
        /// Typecast / convert [`MouseEvent`] to [`InputEvent::Mouse`].
        fn from(mouse_event: MouseEvent) -> Self { InputEvent::Mouse(mouse_event.into()) }
    }

    impl TryFrom<KeyEvent> for InputEvent {
        type Error = ();

        fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
            Ok(InputEvent::Keyboard(key_event.try_into()?))
        }
    }
}

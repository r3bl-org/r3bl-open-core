/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use clap::ValueEnum;
use miette::IntoDiagnostic as _;

use crate::{ch,
            enter_event_loop_async,
            fg_green,
            get_size,
            inline_string,
            usize,
            CalculateResizeHint,
            CaretVerticalViewportLocation,
            EventLoopResult,
            Header,
            Height,
            InputDevice,
            InputEvent,
            ItemsOwned,
            Key,
            KeyPress,
            KeyState,
            LineStateControlSignal,
            ModifierKeysMask,
            OutputDevice,
            SelectComponent,
            SharedWriter,
            SpecialKey,
            State,
            StyleSheet,
            Width,
            DEVELOPMENT_MODE};

pub const DEFAULT_HEIGHT: usize = 5;

/// This struct is provided for convenience to create a default set of IO devices which
/// can be used in the `choose_async()` function. The reason this has to be created
/// outside of the `choose_async()` function is because mutable references to these
/// devices are passed to it, and it can't take ownership of them.
#[allow(missing_debug_implementations)]
pub struct DefaultIoDevices {
    pub output_device: OutputDevice,
    pub input_device: InputDevice,
    pub maybe_shared_writer: Option<SharedWriter>,
}

impl Default for DefaultIoDevices {
    fn default() -> Self {
        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::new_event_stream();
        DefaultIoDevices {
            output_device,
            input_device,
            maybe_shared_writer: None,
        }
    }
}

impl DefaultIoDevices {
    pub fn as_mut_tuple(
        &mut self,
    ) -> (&mut OutputDevice, &mut InputDevice, Option<SharedWriter>) {
        (
            &mut self.output_device,
            &mut self.input_device,
            self.maybe_shared_writer.clone(),
        )
    }
}

/// Async function to choose an item from a list of items.
///
/// It takes a list of items, and returns the selected item or items (depending on the
/// selection mode). If the user does not select anything, it returns `None`. The function
/// also takes the maximum height and width of the display, and the selection mode (single
/// select or multiple select).
///
/// If the terminal is *fully* un-interactive, it returns `None`. This is useful so that
/// it won't block `cargo test` or when run in non-interactive CI/CD environments.
///
/// # Arguments
///
/// * `arg_header` - The header to display above the list.
/// * `arg_options_to_choose_from` - The list of items to choose from.
/// * `maybe_max_height` - Optional: the maximum height of the list.
/// * `maybe_max_width` - Optional: the maximum width of the list.
/// * `how` - The selection mode.
/// * `stylesheet` - The style to use for the list.
/// * `io` - The input and output devices to use. Call
///   [`DefaultIoDevices::as_mut_tuple()`] if you don't want to specify anything here.
///   * `output_device` - The output device to use.
///   * `input_device` - The input device to use.
///   * `maybe_shared_writer` - The shared writer to use, if `ReadlineAsync` is in use,
///     and the async stdout needs to be paused when this function is running.
pub async fn choose<'a>(
    arg_header: impl Into<Header>,
    arg_options_to_choose_from: impl Into<ItemsOwned>,
    maybe_max_height: Option<Height>,
    maybe_max_width: Option<Width>,
    how: HowToChoose,
    stylesheet: StyleSheet,
    io: (
        &'a mut OutputDevice,
        &'a mut InputDevice,
        Option<SharedWriter>,
    ),
) -> miette::Result<ItemsOwned> {
    let from = arg_options_to_choose_from.into();

    // Destructure the io tuple.
    let (od, id, msw) = io;

    // For compatibility with ReadlineAsync (if it is in use).
    if let Some(ref sw) = msw {
        // Pause the shared writer while the user is choosing an item.
        sw.line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .into_diagnostic()?;
    }

    // - If the max size is None, then set it to DEFAULT_HEIGHT.
    // - If the max size is Some, then this is the max height of the viewport.
    //   - However, if this is 0, then set to DEFAULT_HEIGHT.
    //   - Otherwise, check whether the number of items is less than this max height and
    //     set the max height to the number of items.
    //   - Otherwise, if there are more items than the max height, then clamp it to the
    //     max height.
    let max_display_height = ch({
        match maybe_max_height {
            None => DEFAULT_HEIGHT,
            Some(row_height) => {
                let row_height = row_height.as_usize();
                if row_height == 0 {
                    DEFAULT_HEIGHT
                } else {
                    std::cmp::min(row_height, from.len())
                }
            }
        }
    });

    let max_display_width = ch(match maybe_max_width {
        None => 0,
        Some(col_width) => col_width.as_usize(),
    });

    let mut state = State {
        max_display_height,
        max_display_width,
        items: from,
        header: arg_header.into(),
        selection_mode: how,
        ..Default::default()
    };

    let mut fc = SelectComponent {
        output_device: od.clone(),
        style: stylesheet,
    };

    if let Ok(size) = get_size() {
        state.set_size(size);
    }

    let res_user_input =
        enter_event_loop_async(&mut state, &mut fc, keypress_handler, id).await;

    // For compatibility with ReadlineAsync (if it is in use).
    if let Some(ref sw) = msw {
        // Resume the shared writer after the user has made their choice.
        sw.line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .into_diagnostic()?;
    }

    match res_user_input {
        Ok(EventLoopResult::ExitWithResult(it)) => Ok(it),
        _ => Ok(ItemsOwned::default()),
    }
}

fn keypress_handler(state: &mut State, ie: InputEvent) -> EventLoopResult {
    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "🔆🔆🔆 *before* keypress: locate_cursor_in_viewport()",
            cursor_location_in_viewport = ?state.locate_cursor_in_viewport()
        );
    });

    let selection_mode = state.selection_mode;

    let return_it = match ie {
        // Resize.
        InputEvent::Resize(size) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "🍎🍎🍎 keypress_handler() resize",
                    details = %inline_string!(
                        "New size width:{w} x height:{h}",
                        w = fg_green(&inline_string!("{:?}", size.col_width)),
                        h = fg_green(&inline_string!("{:?}", size.row_height)),
                    )
                };
            });
            state.set_resize_hint(size);
            EventLoopResult::ContinueAndRerenderAndClear
        }

        // Down.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        }) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "Down");
            });
            let caret_location = state.locate_cursor_in_viewport();
            match caret_location {
                CaretVerticalViewportLocation::AtAbsoluteTop
                | CaretVerticalViewportLocation::AboveTopOfViewport
                | CaretVerticalViewportLocation::AtTopOfViewport
                | CaretVerticalViewportLocation::InMiddleOfViewport => {
                    state.raw_caret_row_index += 1;
                }

                CaretVerticalViewportLocation::AtBottomOfViewport
                | CaretVerticalViewportLocation::BelowBottomOfViewport => {
                    state.scroll_offset_row_index += 1;
                }

                CaretVerticalViewportLocation::AtAbsoluteBottom
                | CaretVerticalViewportLocation::NotFound => {
                    // Do nothing.
                }
            }
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "enter_event_loop()::state",
                    state = ?state
                );
            });

            EventLoopResult::ContinueAndRerender
        }

        // Up.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        }) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "Up");
            });

            match state.locate_cursor_in_viewport() {
                CaretVerticalViewportLocation::NotFound
                | CaretVerticalViewportLocation::AtAbsoluteTop => {
                    // Do nothing.
                }
                CaretVerticalViewportLocation::AboveTopOfViewport
                | CaretVerticalViewportLocation::AtTopOfViewport => {
                    state.scroll_offset_row_index -= 1;
                }
                CaretVerticalViewportLocation::InMiddleOfViewport => {
                    state.raw_caret_row_index -= 1;
                }
                CaretVerticalViewportLocation::AtBottomOfViewport
                | CaretVerticalViewportLocation::BelowBottomOfViewport
                | CaretVerticalViewportLocation::AtAbsoluteBottom => {
                    state.raw_caret_row_index -= 1;
                }
            }

            EventLoopResult::ContinueAndRerender
        }

        // Enter on multi-select.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        }) if selection_mode == HowToChoose::Multiple => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Enter on multi-select",
                    selected_items = ?state.selected_items
                );
            });
            if state.selected_items.is_empty() {
                EventLoopResult::ExitWithoutResult
            } else {
                EventLoopResult::ExitWithResult(state.selected_items.clone())
            }
        }

        // Enter.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        }) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Enter",
                    focused_index = ?state.get_focused_index()
                );
            });
            let selection_index = usize(state.get_focused_index());
            let maybe_item = state.items.get(selection_index);
            match maybe_item {
                Some(it) => EventLoopResult::ExitWithResult(it.into()),
                None => EventLoopResult::ExitWithoutResult,
            }
        }

        // Escape or Ctrl + c.
        InputEvent::Keyboard(
            KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Esc),
            }
            | KeyPress::WithModifiers {
                key: Key::Character('c'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            },
        ) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "Esc");
            });
            EventLoopResult::ExitWithoutResult
        }

        // Space on multi-select.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character(' '),
        }) if selection_mode == HowToChoose::Multiple => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Space on multi-select",
                    focused_index = ?state.get_focused_index()
                );
            });
            let selection_index = usize(state.get_focused_index());
            let maybe_item = state.items.get(selection_index);
            let maybe_index = state
                .selected_items
                .iter()
                .position(|item| Some(item) == maybe_item);
            match (maybe_item, maybe_index) {
                // No selected_item.
                (None, _) => (),
                // Item already in selected_items so remove it.
                (Some(_), Some(it)) => {
                    state.selected_items.remove(it);
                }
                // Item not found in selected_items so add it.
                (Some(it), None) => state.selected_items.push(it.clone()),
            }

            EventLoopResult::ContinueAndRerender
        }

        // Default behavior on Space
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character(' '),
        }) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "Space");
            });
            EventLoopResult::Continue
        }

        // Ignore other keys.
        _ => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "Ignore key event");
            });
            EventLoopResult::Continue
        }
    };

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "👉 *after* keypress: locate_cursor_in_viewport()",
            cursor_location_in_viewport = ?state.locate_cursor_in_viewport()
        );
    });

    return_it
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Hash,
)]
pub enum HowToChoose {
    /// Select only one option from list.
    #[default]
    Single,
    /// Select multiple options from list.
    Multiple,
}

#[cfg(test)]
mod test_choose_async {
    use std::{io::Write as _, time::Duration};

    use smallvec::smallvec;

    use super::*;
    use crate::{CrosstermEventResult, InlineVec, InputDeviceExtMock, OutputDeviceExt};

    /// Simulated key inputs: Down, Down, Enter.
    fn generated_key_events() -> InlineVec<CrosstermEventResult> {
        // Simulated key inputs.
        let generator_vec: InlineVec<CrosstermEventResult> = smallvec![
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Down,
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Down,
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
            Ok(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Enter,
                    crossterm::event::KeyModifiers::empty(),
                ),
            )),
        ];
        generator_vec
    }

    /// This test verifies that the `SharedWriter` pauses and resumes correctly when
    /// `choose()` is called. It does not verify the actual output of the `SharedWriter`.
    /// When this test is run in an interactive vs non-interactive terminal, the
    /// assertions might be different, which is why the test is vague intentionally.
    #[tokio::test]
    async fn test_shared_writer_pause_works() {
        // Set up the io devices.
        let (mut line_receiver, shared_writer) = SharedWriter::new_mock();
        let (mut output_device, stdout_mock) = OutputDevice::new_mock();
        let mut input_device = InputDevice::new_mock_with_delay(
            generated_key_events(), /* Down, Down, Enter */
            Duration::from_millis(10),
        );

        // Spawn a task to write something to SharedWriter with delays.
        let mut sw_1 = shared_writer.clone();
        tokio::spawn(async move {
            // Wait 10ms then write something.
            tokio::time::sleep(Duration::from_millis(10)).await;
            sw_1.write_all(b"data after 10ms delay\n").unwrap();

            // Wait 100ms then write something. This should not show up since the test
            // will be over in 30ms.
            tokio::time::sleep(Duration::from_millis(100)).await;
            sw_1.write_all(b"data after 100ms delay\n").unwrap();
        });

        // Nothing should be written to the shared writer yet.
        assert_eq!(shared_writer.buffer, "");
        assert_eq!(stdout_mock.get_copy_of_buffer_as_string(), "");
        assert!(line_receiver.is_empty());

        // The following code waits for 30ms. In the meantime, the shared writer
        // 1. should be paused.
        // 2. after 10ms, "data after 10ms delay\n" will be written to shared writer.
        // 3. after 30ms, the shared writer will be resumed (when choose() completes).
        let _unused: ItemsOwned = choose(
            Header::SingleLine("Choose one:".into()),
            &["one", "two", "three"],
            None,
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            (
                &mut output_device,
                &mut input_device,
                Some(shared_writer.clone()),
            ),
        )
        .await
        .unwrap();

        let mut acc = vec![];
        line_receiver.close();
        while let Some(line) = line_receiver.recv().await {
            acc.push(line);
        }

        assert!(matches!(
            acc.first().unwrap(),
            LineStateControlSignal::Pause
        ));
        assert!(matches!(
            acc.last().unwrap(),
            LineStateControlSignal::Resume
        ));
    }
}

/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::io::stdout;

use clap::ValueEnum;
use crossterm::style::Stylize;
use r3bl_ansi_color::AnsiStyledText;
use r3bl_core::{call_if_true, ch, get_size, Size};

use crate::{enter_event_loop,
            CalculateResizeHint,
            CaretVerticalViewportLocation,
            CrosstermKeyPressReader,
            EventLoopResult,
            KeyPress,
            SelectComponent,
            State,
            StyleSheet,
            DEVELOPMENT_MODE};

pub const DEFAULT_HEIGHT: usize = 5;

/// This function does the work of rendering the TUI.
///
/// It takes a list of items, and returns the selected item or items (depending on the
/// selection mode). If the user does not select anything, it returns `None`. The function
/// also takes the maximum height and width of the display, and the selection mode (single
/// select or multiple select).
///
/// If the terminal is *fully* uninteractive, it returns `None`. This is useful so that it
/// won't block `cargo test` or when run in non-interactive CI/CD environments.
pub fn select_from_list(
    header: String,
    items: Vec<String>,
    max_height_row_count: usize,
    // If you pass 0, then the width of your terminal gets set as max_width_col_count.
    max_width_col_count: usize,
    selection_mode: SelectionMode,
    style: StyleSheet,
) -> Option<Vec<String>> {
    // There are fewer items than viewport height. So make viewport shorter.
    let max_height_row_count = if items.len() <= max_height_row_count {
        items.len()
    } else {
        max_height_row_count
    };

    let mut state = State {
        max_display_height: ch!(max_height_row_count),
        max_display_width: ch!(max_width_col_count),
        items,
        header,
        selection_mode,
        ..Default::default()
    };

    let mut function_component = SelectComponent {
        write: stdout(),
        style,
    };

    if let Ok(size) = get_size() {
        state.set_size(size);
    }

    let result_user_input = enter_event_loop(
        &mut state,
        &mut function_component,
        |state, key_press| keypress_handler(state, key_press),
        &mut CrosstermKeyPressReader {},
    );

    match result_user_input {
        Ok(EventLoopResult::ExitWithResult(it)) => Some(it),
        _ => None,
    }
}

pub fn select_from_list_with_multi_line_header(
    multi_line_header: Vec<Vec<AnsiStyledText<'_>>>,
    items: Vec<String>,
    maybe_max_height_row_count: Option<usize>,
    // If you pass None, then the width of your terminal gets used.
    maybe_max_width_col_count: Option<usize>,
    selection_mode: SelectionMode,
    style: StyleSheet,
) -> Option<Vec<String>> {
    // There are fewer items than viewport height. So make viewport shorter.
    let max_height_row_count = match maybe_max_height_row_count {
        Some(requested_height) => sanitize_height(&items, requested_height),
        None => sanitize_height(&items, DEFAULT_HEIGHT),
    };

    let max_width_col_count = maybe_max_width_col_count.unwrap_or(0);

    let mut state = State {
        max_display_height: ch!(max_height_row_count),
        max_display_width: ch!(max_width_col_count),
        items,
        multi_line_header,
        selection_mode,
        ..Default::default()
    };

    let mut function_component = SelectComponent {
        write: stdout(),
        style,
    };

    if let Ok(size) = get_size() {
        state.set_size(size);
    }

    let result_user_input = enter_event_loop(
        &mut state,
        &mut function_component,
        |state, key_press| keypress_handler(state, key_press),
        &mut CrosstermKeyPressReader {},
    );

    match result_user_input {
        Ok(EventLoopResult::ExitWithResult(it)) => Some(it),
        _ => None,
    }
}

fn sanitize_height(items: &[String], requested_height: usize) -> usize {
    let num_items = items.len();
    if num_items > requested_height {
        requested_height
    } else {
        num_items
    }
}

fn keypress_handler(state: &mut State<'_>, key_press: KeyPress) -> EventLoopResult {
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!(
            "🔆🔆🔆 *before* keypress: locate_cursor_in_viewport(): {}",
            format!("{:?}", state.locate_cursor_in_viewport()).magenta()
        );
    });

    let selection_mode = state.selection_mode;

    let return_it = match key_press {
        // Resize.
        KeyPress::Resize(Size {
            col_count,
            row_count,
        }) => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!(
                    "\n🍎🍎🍎\nNew size width:{} x height:{}",
                    format!("{col_count}").green(),
                    format!("{row_count}").green(),
                );
            });
            state.set_resize_hint(Size {
                col_count,
                row_count,
            });
            EventLoopResult::ContinueAndRerenderAndClear
        }

        // Down.
        KeyPress::Down => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("Down");
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
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!(
                    "enter_event_loop()::state: {}",
                    format!("{state:?}").blue()
                );
            });

            EventLoopResult::ContinueAndRerender
        }

        // Up.
        KeyPress::Up => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("Up");
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
        KeyPress::Enter if selection_mode == SelectionMode::Multiple => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!(
                    "Enter: {}",
                    format!("{:?}", state.selected_items).green()
                );
            });
            if state.selected_items.is_empty() {
                EventLoopResult::ExitWithoutResult
            } else {
                EventLoopResult::ExitWithResult(state.selected_items.clone())
            }
        }

        // Enter.
        KeyPress::Enter => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!(
                    "Enter: {}",
                    format!("{:?}", state.get_focused_index()).green()
                );
            });
            let selection_index: usize = ch!(@to_usize state.get_focused_index());
            let maybe_item: Option<&String> = state.items.get(selection_index);
            match maybe_item {
                Some(it) => EventLoopResult::ExitWithResult(vec![it.to_string()]),
                None => EventLoopResult::ExitWithoutResult,
            }
        }

        // Escape or Ctrl + c.
        KeyPress::Esc | KeyPress::CtrlC => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("Esc");
            });
            EventLoopResult::ExitWithoutResult
        }

        // Space on multi-select.
        KeyPress::Space if selection_mode == SelectionMode::Multiple => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!(
                    "Space: {}",
                    format!("{:?}", state.get_focused_index()).magenta()
                );
            });
            let selection_index: usize = ch!(@to_usize state.get_focused_index());
            let maybe_item: Option<&String> = state.items.get(selection_index);
            let maybe_index: Option<usize> = state
                .selected_items
                .iter()
                .position(|x| Some(x) == maybe_item);
            match (maybe_item, maybe_index) {
                // No selected_item.
                (None, _) => (),
                // Item already in selected_items so remove it.
                (Some(_), Some(it)) => {
                    state.selected_items.remove(it);
                }
                // Item not found in selected_items so add it.
                (Some(it), None) => state.selected_items.push(it.to_string()),
            };

            EventLoopResult::ContinueAndRerender
        }

        // Noop, default behavior on Space
        KeyPress::Noop | KeyPress::Space => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("Noop");
            });
            EventLoopResult::Continue
        }

        // Error.
        KeyPress::Error => {
            call_if_true!(DEVELOPMENT_MODE, {
                tracing::debug!("Exit with error");
            });
            EventLoopResult::ExitWithError
        }
    };

    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!(
            "👉 *after* keypress: locate_cursor_in_viewport(): {}",
            format!("{:?}", state.locate_cursor_in_viewport()).blue()
        );
    });

    return_it
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Hash,
)]
pub enum SelectionMode {
    /// Select only one option from list.
    #[default]
    Single,
    /// Select multiple options from list.
    Multiple,
}

#[cfg(test)]
mod test_select_from_list {
    use r3bl_ansi_color::{is_fully_uninteractive_terminal, TTYResult};
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::{TestStringWriter, TestVecKeyPressReader};

    fn create_state<'a>() -> State<'a> {
        State {
            max_display_height: ch!(10),
            items: ["a", "b", "c"].iter().map(|it| it.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn enter_pressed() {
        let mut state = create_state();
        let string_writer = TestStringWriter::new();
        let style_sheet = StyleSheet::default();

        let mut function_component = SelectComponent {
            write: string_writer,
            style: style_sheet,
        };

        let mut reader = TestVecKeyPressReader {
            key_press_vec: vec![KeyPress::Down, KeyPress::Down, KeyPress::Enter],
            index: None,
        };

        let result_event_loop_result = enter_event_loop(
            &mut state,
            &mut function_component,
            |state, key_press| keypress_handler(state, key_press),
            &mut reader,
        );

        assert_eq2!(
            result_event_loop_result.unwrap(),
            if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
                EventLoopResult::ExitWithError
            } else {
                EventLoopResult::ExitWithResult(vec!["c".to_string()])
            }
        );
    }

    #[test]
    fn ctrl_c_pressed() {
        let mut state = create_state();
        let string_writer = TestStringWriter::new();
        let style_sheet = StyleSheet::default();

        let mut function_component = SelectComponent {
            write: string_writer,
            style: style_sheet,
        };

        let mut reader = TestVecKeyPressReader {
            key_press_vec: vec![KeyPress::Down, KeyPress::Down, KeyPress::CtrlC],
            index: None,
        };

        let result_event_loop_result = enter_event_loop(
            &mut state,
            &mut function_component,
            |state, key_press| keypress_handler(state, key_press),
            &mut reader,
        );

        assert_eq2!(
            result_event_loop_result.unwrap(),
            if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
                EventLoopResult::ExitWithError
            } else {
                EventLoopResult::ExitWithoutResult
            }
        );
    }
}

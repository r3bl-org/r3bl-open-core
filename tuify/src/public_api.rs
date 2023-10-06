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
use r3bl_rs_utils_core::*;

use crate::*;

/// This function does the work of rendering the TUI. It takes a list of items, and returns
/// the selected item or items (depending on the selection mode). If the user does not
/// select anything, it returns `None`. The function also takes the maximum height and
/// width of the display, and the selection mode (single select or multiple select).
///
/// If the terminal is *fully* uninteractive, it returns `None`. This is useful so that it
/// won't block `cargo test` or when run in non-interactive CI/CD environments.
pub fn select_from_list(
    header: String,
    items: Vec<String>,
    max_height_row_count: usize,
    max_width_col_count: usize,
    selection_mode: SelectionMode,
    style: StyleSheet,
) -> Option<Vec<String>> {
    // Don't block tests.
    if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
        return None;
    }

    // There are fewer items than viewport height. So make viewport shorter.
    let max_height_row_count = if items.len() <= max_height_row_count {
        items.len()
    } else {
        max_height_row_count
    };

    let mut state = State {
        max_display_height: max_height_row_count.into(),
        max_display_width: max_width_col_count.into(),
        raw_caret_row_index: ch!(0),
        scroll_offset_row_index: ch!(0),
        items,
        selected_items: Vec::new(),
        header,
        selection_mode,
    };

    let mut function_component = SelectComponent {
        write: stdout(),
        style,
    };

    let user_input =
        enter_event_loop(&mut state, &mut function_component, |state, key_press| {
            keypress_handler(state, key_press)
        });

    match user_input {
        Ok(EventLoopResult::ExitWithResult(it)) => Some(it),
        _ => None,
    }
}

fn keypress_handler(state: &mut State, key_press: KeyPress) -> EventLoopResult {
    call_if_true!(TRACE, {
        log_debug(
            format!(
                "🔆🔆🔆 *before* keypress: locate_cursor_in_viewport(): {:?}",
                state.locate_cursor_in_viewport()
            )
            .magenta()
            .to_string(),
        );
    });

    let selection_mode = state.selection_mode;

    let return_it = match key_press {
        // Down.
        KeyPress::Down => {
            call_if_true!(TRACE, {
                log_debug("Down".black().bold().on_green().to_string());
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
            call_if_true!(TRACE, {
                log_debug(
                    format!("enter_event_loop()::state: {:?}", state)
                        .blue()
                        .to_string(),
                );
            });

            EventLoopResult::Continue
        }

        // Up.
        KeyPress::Up => {
            call_if_true!(TRACE, {
                log_debug("Up".black().bold().on_green().to_string());
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

            EventLoopResult::Continue
        }

        // Enter on multi-select.
        KeyPress::Enter if selection_mode == SelectionMode::Multiple => {
            call_if_true!(TRACE, {
                log_debug(
                    format!("Enter: {:?}", state.selected_items)
                        .green()
                        .to_string(),
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
            call_if_true!(TRACE, {
                log_debug(
                    format!("Enter: {}", state.get_focused_index())
                        .green()
                        .to_string(),
                );
            });
            let selection_index: usize = ch!(@to_usize state.get_focused_index());
            let maybe_item: Option<&String> = state.items.get(selection_index);
            match maybe_item {
                Some(it) => EventLoopResult::ExitWithResult(vec![it.to_string()]),
                None => EventLoopResult::ExitWithoutResult,
            }
        }

        // Escape.
        KeyPress::Esc => {
            call_if_true!(TRACE, {
                log_debug("Esc".red().to_string());
            });
            EventLoopResult::ExitWithoutResult
        }

        // Space on multi-select.
        KeyPress::Space if selection_mode == SelectionMode::Multiple => {
            call_if_true!(TRACE, {
                log_debug(
                    format!("Space: {}", state.get_focused_index())
                        .magenta()
                        .to_string(),
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
            EventLoopResult::Continue
        }

        // Noop, default behavior on Space
        KeyPress::Noop | KeyPress::Space => {
            call_if_true!(TRACE, {
                log_debug("Noop".yellow().to_string());
            });
            EventLoopResult::Continue
        }

        // Error.
        KeyPress::Error => {
            call_if_true!(TRACE, {
                log_debug("Exit with error".red().to_string());
            });
            EventLoopResult::ExitWithError
        }
    };

    call_if_true!(TRACE, {
        log_debug(
            format!(
                "👉 *after* keypress: locate_cursor_in_viewport(): {:?}",
                state.locate_cursor_in_viewport()
            )
            .blue()
            .to_string(),
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

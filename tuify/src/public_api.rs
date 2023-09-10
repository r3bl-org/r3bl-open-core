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

use crate::*;
use clap::ValueEnum;
use crossterm::style::Stylize;
use r3bl_rs_utils_core::*;
use std::io::stdout;

/// This function does the work of rendering the TUI. It takes a list of items, and returns
/// the selected item or items (depending on the selection mode). If the user does not
/// select anything, it returns `None`. The function also takes the maximum height and
/// width of the display, and the selection mode (single select or multiple select).
// TODO: pass styles for selected and unselected.
pub fn select_from_list(
    items: Vec<String>,
    max_height_row_count: usize,
    max_width_col_count: usize,
    // TODO: use this in the feature and unlock multiple selection.
    _selection_mode: SelectionMode,
) -> Option<Vec<String>> {
    let mut state = State {
        max_display_height: max_height_row_count.into(),
        max_display_width: max_width_col_count.into(),
        raw_caret_row_index: ch!(0),
        scroll_offset_row_index: ch!(0),
        items,
    };

    let mut function_component = SingleSelectComponent { write: stdout() };

    let user_input = enter_event_loop(
        &mut state,
        &mut function_component,
        |state, key_press| -> EventLoopResult {
            match key_press {
                // Down.
                KeyPress::Down => {
                    call_if_true!(TRACE, {
                        log_debug("Down".yellow().to_string());
                    });
                    match state.locate_cursor_in_viewport() {
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

                        CaretVerticalViewportLocation::AtAbsoluteBottom => {
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
                        log_debug("Up".yellow().to_string());
                    });

                    match state.locate_cursor_in_viewport() {
                        CaretVerticalViewportLocation::AtAbsoluteTop => {
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

                // Enter.
                KeyPress::Enter => {
                    call_if_true!(TRACE, {
                        log_debug(
                            format!("Enter: {}", state.get_selected_index())
                                .green()
                                .to_string(),
                        );
                    });
                    let selection_index: usize = ch!(@to_usize state.get_selected_index());
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

                // Noop.
                KeyPress::Noop => {
                    call_if_true!(TRACE, {
                        log_debug("Noop".yellow().to_string());
                    });
                    EventLoopResult::Continue
                }
            }
        },
    );

    match user_input {
        Ok(EventLoopResult::ExitWithResult(it)) => Some(it),
        _ => None,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SelectionMode {
    /// Select only one option from list.
    Single,
    /// Select multiple options from list.
    Multiple,
}

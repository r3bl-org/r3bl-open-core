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

use std::vec;

use r3bl_ansi_color::{AnsiStyledText, Color, Style as RStyle};
use r3bl_core::{call_if_true, get_size, get_terminal_width, throws, usize};
use r3bl_log::try_initialize_logging_global;
use r3bl_tuify::{components::style::StyleSheet,
                 select_from_list,
                 select_from_list_with_multi_line_header,
                 SelectionMode,
                 DEVELOPMENT_MODE};
mod single_select_quiz_game;
use single_select_quiz_game::main as single_select_quiz_game;

fn main() -> miette::Result<()> {
    throws!({
        call_if_true!(DEVELOPMENT_MODE, {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            tracing::debug!("Start logging... terminal window size: {:?}", get_size()?)
        });

        // Get display size.
        let max_width_col_count = usize(*get_terminal_width());
        let max_height_row_count: usize = 5;

        // Create styles.
        let default_style = StyleSheet::default();
        let sea_foam_style = StyleSheet::sea_foam_style();
        let hot_pink_style = StyleSheet::hot_pink_style();

        const MULTI_LINE_HEADER: &str = "Multi line header";
        const SINGLE_LINE_HEADER: &str = "Single line header";
        const MULTIPLE_SELECT_SINGLE_ITEM: &str = "Multiple select, single item";
        const MULTIPLE_SELECT_13_ITEMS_VPH_5: &str =
            "Multiple select, 13 items, viewport height = 5";
        const MULTIPLE_SELECT_2_ITEMS_VPH_5: &str =
            "Multiple select, 2 items, viewport height = 5";
        const SINGLE_SELECT_13_ITEMS_VPH_5: &str =
            "Single select, 13 items, viewport height = 5";
        const SINGLE_SELECT_2_ITEMS_VPH_5: &str =
            "Single select, 2 items, viewport height = 5";
        const SINGLE_SELECT_QUIZ_GAME: &str = "Single select, quiz game";

        // Add tuify to select which example to run.
        let maybe_user_input = select_from_list(
            "Select which example to run".to_string(),
            [
                MULTI_LINE_HEADER,
                SINGLE_LINE_HEADER,
                MULTIPLE_SELECT_SINGLE_ITEM,
                MULTIPLE_SELECT_13_ITEMS_VPH_5,
                MULTIPLE_SELECT_2_ITEMS_VPH_5,
                SINGLE_SELECT_13_ITEMS_VPH_5,
                SINGLE_SELECT_2_ITEMS_VPH_5,
                SINGLE_SELECT_QUIZ_GAME,
            ]
            .iter()
            .map(|it| it.to_string())
            .collect(),
            6, /* height of the tuify component */
            0, /* width of the tuify component. 0 means it will use the full terminal width */
            SelectionMode::Single,
            StyleSheet::default(),
        );

        match &maybe_user_input {
            Some(input) => {
                let first_line = input.first();

                match first_line {
                    Some(user_input) => {
                        if user_input == MULTI_LINE_HEADER {
                            multi_line_header();
                        } else if user_input == SINGLE_LINE_HEADER {
                            single_line_header();
                        } else if user_input == MULTIPLE_SELECT_SINGLE_ITEM {
                            // Multiple select, single item.
                            multiple_select_single_item()
                        } else if user_input == MULTIPLE_SELECT_13_ITEMS_VPH_5 {
                            // Multiple select.
                            multiple_select_13_items_vph_5(
                                max_height_row_count,
                                max_width_col_count,
                                sea_foam_style,
                            );
                        } else if user_input == MULTIPLE_SELECT_2_ITEMS_VPH_5 {
                            multiple_select_2_items_vph_5(
                                max_height_row_count,
                                max_width_col_count,
                                sea_foam_style,
                            );
                        } else if user_input == SINGLE_SELECT_13_ITEMS_VPH_5 {
                            // Single select.
                            single_select_13_items_vph_5(
                                max_height_row_count,
                                max_width_col_count,
                                hot_pink_style,
                            );
                        } else if user_input == SINGLE_SELECT_2_ITEMS_VPH_5 {
                            single_select_2_items_vph_5(
                                max_height_row_count,
                                max_width_col_count,
                                default_style,
                            );
                        } else if user_input == SINGLE_SELECT_QUIZ_GAME {
                            let _ = single_select_quiz_game();
                        } else {
                            println!("User did not select anything")
                        }
                    }
                    None => println!("User did not select anything"),
                }
            }
            None => println!("User did not select anything"),
        }

        call_if_true!(DEVELOPMENT_MODE, {
            tracing::debug!("Stop logging...");
        });
    });
}

// Multi line header.
fn multi_line_header() {
    let header = AnsiStyledText {
        text: " Please select one or more items. This is a really long heading that just keeps going and if your terminal viewport is small enough, this heading will be clipped",
        style: &[
            RStyle::Foreground(Color::Rgb(171, 204, 242)),
            RStyle::Background(Color::Rgb(31, 36, 46)),
        ],
    };
    let line_5 = vec![header];

    let mut instructions: Vec<Vec<AnsiStyledText>> = multi_select_instructions();
    instructions.push(line_5);

    let user_input = select_from_list_with_multi_line_header(
        instructions,
        [
            "item 1 of 13",
            "item 2 of 13",
            "item 3 of 13",
            "item 4 of 13",
            "item 5 of 13",
            "item 6 of 13",
            "item 7 of 13",
            "item 8 of 13",
            "item 9 of 13",
            "item 10 of 13",
            "item 11 of 13",
            "item 12 of 13",
            "item 13 of 13",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        Some(6),
        None,
        SelectionMode::Multiple,
        StyleSheet::default(),
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
}

fn single_line_header() {
    let max_width_col_count = usize(*get_terminal_width());
    let user_input = select_from_list(
        "🦜 Please select one or more items. This is an example of a very long header text 🐧. You can pass emoji here 🐥 and text gets clipped off correctly 🐒, based on terminal size".to_string(),
        [
            "item 1 of 13",
            "item 2 of 13",
            "item 3 of 13",
            "item 4 of 13",
            "item 5 of 13",
            "item 6 of 13",
            "item 7 of 13",
            "item 8 of 13",
            "item 9 of 13",
            "item 10 of 13",
            "item 11 of 13",
            "item 12 of 13",
            "item 13 of 13",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        5,
        max_width_col_count,
        SelectionMode::Multiple,
        StyleSheet::default(),
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!("user_input: {user_input:?}");
    });
}

/// Multiple select, single item.
fn multiple_select_single_item() {
    let mut instructions = multi_select_instructions();
    let header = AnsiStyledText {
        text: " Please select one or more items",
        style: &[
            RStyle::Foreground(Color::Rgb(171, 204, 242)),
            RStyle::Background(Color::Rgb(31, 36, 46)),
        ],
    };
    instructions.push(vec![header]);
    let list = vec![format!("one element")];
    let user_input = select_from_list_with_multi_line_header(
        instructions,
        list,
        Some(6),
        None,
        SelectionMode::Multiple,
        StyleSheet::default(),
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
}

/// 13 items & viewport height = 5.
fn multiple_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    let mut instructions = multi_select_instructions();
    let header = AnsiStyledText {
        text: " Please select one or more items",
        style: &[
            RStyle::Foreground(Color::Rgb(229, 239, 123)),
            RStyle::Background(Color::Rgb(31, 36, 46)),
        ],
    };
    instructions.push(vec![header]);

    let user_input = select_from_list_with_multi_line_header(
        instructions,
        [
            "item 1 of 13",
            "item 2 of 13",
            "item 3 of 13",
            "item 4 of 13",
            "item 5 of 13",
            "item 6 of 13",
            "item 7 of 13",
            "item 8 of 13",
            "item 9 of 13",
            "item 10 of 13",
            "item 11 of 13",
            "item 12 of 13",
            "item 13 of 13",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        Some(max_height_row_count),
        Some(max_width_col_count),
        SelectionMode::Multiple,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!("user_input: {user_input:?}");
    });
}

/// 2 items & viewport height = 5.
fn multiple_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    let mut instructions = multi_select_instructions();
    let header = AnsiStyledText {
        text: " Please select one or more items",
        style: &[
            RStyle::Foreground(Color::Rgb(229, 239, 123)),
            RStyle::Background(Color::Rgb(31, 36, 46)),
        ],
    };

    instructions.push(vec![header]);

    let user_input = select_from_list_with_multi_line_header(
        instructions,
        ["item 1 of 2", "item 2 of 2"]
            .iter()
            .map(|it| it.to_string())
            .collect(),
        Some(max_height_row_count),
        Some(max_width_col_count),
        SelectionMode::Multiple,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!("user_input: {user_input:?}");
    });
}

/// 13 items & viewport height = 5.
fn single_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    let user_input = select_from_list(
        "Single select".to_string(),
        [
            "item 1 of 13",
            "item 2 of 13",
            "item 3 of 13",
            "item 4 of 13",
            "item 5 of 13",
            "item 6 of 13",
            "item 7 of 13",
            "item 8 of 13",
            "item 9 of 13",
            "item 10 of 10",
            "item 11 of 13",
            "item 12 of 13",
            "item 13 of 13",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        max_height_row_count,
        max_width_col_count,
        SelectionMode::Single,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!("user_input: {user_input:?}");
    });
}

/// 2 items & viewport height = 5.
fn single_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    let mut instructions = single_select_instruction();
    let header = AnsiStyledText {
        text: " Please select one item",
        style: &[
            RStyle::Foreground(Color::Rgb(171, 204, 242)),
            RStyle::Background(Color::Rgb(31, 36, 46)),
        ],
    };
    instructions.push(vec![header]);

    let user_input = select_from_list_with_multi_line_header(
        instructions,
        ["item 1 of 2", "item 2 of 2"]
            .iter()
            .map(|it| it.to_string())
            .collect(),
        Some(max_height_row_count),
        Some(max_width_col_count),
        SelectionMode::Single,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(DEVELOPMENT_MODE, {
        tracing::debug!("user_input: {user_input:?}");
    });
}

fn multi_select_instructions() -> Vec<Vec<AnsiStyledText<'static>>> {
    let up_and_down = AnsiStyledText {
        text: " Up or down:",
        style: &[
            RStyle::Foreground(Color::Rgb(9, 238, 211)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let navigate = AnsiStyledText {
        text: "     navigate",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let line_1 = vec![up_and_down, navigate];

    let space = AnsiStyledText {
        text: " Space:",
        style: &[
            RStyle::Foreground(Color::Rgb(255, 216, 9)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let select = AnsiStyledText {
        text: "          select or deselect item",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let line_2 = vec![space, select];

    let esc = AnsiStyledText {
        text: " Esc or Ctrl+C:",
        style: &[
            RStyle::Foreground(Color::Rgb(255, 132, 18)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let exit = AnsiStyledText {
        text: "  exit program",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let line_3 = vec![esc, exit];
    let return_key = AnsiStyledText {
        text: " Return:",
        style: &[
            RStyle::Foreground(Color::Rgb(234, 0, 196)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let confirm = AnsiStyledText {
        text: "         confirm selection",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let line_4 = vec![return_key, confirm];
    vec![line_1, line_2, line_3, line_4]
}

fn single_select_instruction() -> Vec<Vec<AnsiStyledText<'static>>> {
    let up_and_down = AnsiStyledText {
        text: " Up or down:",
        style: &[
            RStyle::Foreground(Color::Rgb(9, 238, 211)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let navigate = AnsiStyledText {
        text: "     navigate",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let line_1 = vec![up_and_down, navigate];

    let esc = AnsiStyledText {
        text: " Esc or Ctrl+C:",
        style: &[
            RStyle::Foreground(Color::Rgb(255, 132, 18)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let exit = AnsiStyledText {
        text: "  exit program",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let line_2 = vec![esc, exit];
    let return_key = AnsiStyledText {
        text: " Return:",
        style: &[
            RStyle::Foreground(Color::Rgb(234, 0, 196)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let confirm = AnsiStyledText {
        text: "         confirm selection",
        style: &[
            RStyle::Foreground(Color::Rgb(94, 103, 111)),
            RStyle::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let line_3 = vec![return_key, confirm];
    vec![line_1, line_2, line_3]
}

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

use std::io::Result;

use r3bl_ansi_color::{AnsiStyledText, Color as RColor, Style as RStyle};
use r3bl_rs_utils_core::*;
use r3bl_tuify::*;

fn print_header(msg: &str) {
    AnsiStyledText {
        text: msg,
        style: &[
            RStyle::Bold,
            RStyle::Italic,
            RStyle::Underline,
            RStyle::Foreground(RColor::Rgb(236, 230, 230)),
            RStyle::Background(RColor::Rgb(10, 109, 33)),
        ],
    }
    .println();
}

fn main() -> Result<()> {
    throws!({
        call_if_true!(TRACE, {
            try_to_set_log_level(log::LevelFilter::Trace).ok();
            log_debug("Start logging...".to_string());
            log_debug(format!("og_size: {:?}", get_size()?).to_string());
        });

        // Get display size.
        let max_width_col_count: usize =
            get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
        let max_height_row_count: usize = 5;

        let style = StyleSheet::default();

        // Multiple select, single item.
        multiple_select_single_item();

        // Multiple select.
        multiple_select_13_items_vph_5(max_height_row_count, max_width_col_count, style);
        multiple_select_2_items_vph_5(max_height_row_count, max_width_col_count, style);

        // Single select.
        single_select_13_items_vph_5(max_height_row_count, max_width_col_count, style);
        single_select_2_items_vph_5(max_height_row_count, max_width_col_count, style);

        call_if_true!(TRACE, {
            log_debug("Stop logging...".to_string());
        });
    });
}

/// Multiple select, single item.
fn multiple_select_single_item() {
    let max_width_col_count: usize = r3bl_tuify::get_size()
        .map(|it| it.col_count)
        .unwrap_or(ch!(80))
        .into();
    let list = vec![format!("one element")];
    r3bl_tuify::select_from_list(
        "There is only one item to choose from".to_owned(),
        list,
        6, /* whatever*/
        max_width_col_count,
        r3bl_tuify::SelectionMode::Multiple,
        r3bl_tuify::StyleSheet::default(),
    );
}

/// 13 items & viewport height = 5.
fn multiple_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    print_header(
        "Multiple select (move up and down, press space, then enter or esc) - 10 items",
    );

    let user_input = select_from_list(
        "Multiple select".to_string(),
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
        max_height_row_count,
        max_width_col_count,
        SelectionMode::Multiple,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(TRACE, {
        log_debug(format!("user_input: {:?}", user_input).to_string());
    });
}

/// 2 items & viewport height = 5.
fn multiple_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    print_header(
        "Multiple select (move up and down, press space, then enter or esc) - 2 items",
    );

    let user_input = select_from_list(
        "Multiple select".to_string(),
        ["item 1 of 2", "item 2 of 2"]
            .iter()
            .map(|it| it.to_string())
            .collect(),
        max_height_row_count,
        max_width_col_count,
        SelectionMode::Multiple,
        style,
    );
    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }
    call_if_true!(TRACE, {
        log_debug(format!("user_input: {:?}", user_input).to_string());
    });
}

/// 13 items & viewport height = 5.
fn single_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    print_header("Single select (move up and down, press enter or esc) - 10 items");

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
    call_if_true!(TRACE, {
        log_debug(format!("user_input: {:?}", user_input).to_string());
    });
}

/// 2 items & viewport height = 5.
fn single_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) {
    print_header("Single select (move up and down, press enter or esc) - 2 items");

    let user_input = select_from_list(
        "Single select".to_string(),
        ["item 1 of 2", "item 2 of 2"]
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
    call_if_true!(TRACE, {
        log_debug(format!("user_input: {:?}", user_input).to_string());
    });
}

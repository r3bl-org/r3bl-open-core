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

use r3bl_tui::{ASTColor, ASTStyle, AnsiStyledText, DefaultIoDevices, InlineVec, ast,
               ast_line, ast_lines, choose, get_size, get_terminal_width, height,
               inline_vec,
               log::try_initialize_logging_global,
               new_style, ok,
               readline_async::{DEVELOPMENT_MODE, HowToChoose, style::StyleSheet},
               set_mimalloc_in_main, throws, tui_color, usize, width};
use smallvec::smallvec;

#[tokio::main]
async fn main() -> miette::Result<()> {
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

    set_mimalloc_in_main!();

    throws!({
        DEVELOPMENT_MODE.then(|| {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Start logging...",
                window_size = ?get_size()
            );
        });

        // Get display size.
        let max_width_col_count = usize(*get_terminal_width());
        let max_height_row_count: usize = 5;

        // Create styles.
        let default_style = StyleSheet::default();
        let sea_foam_style = StyleSheet::sea_foam_style();
        let hot_pink_style = StyleSheet::hot_pink_style();

        // Choose which example to run.
        let mut default_io_devices = DefaultIoDevices::default();
        let user_input = choose(
            "Select which example to run",
            &[
                MULTI_LINE_HEADER,
                SINGLE_LINE_HEADER,
                MULTIPLE_SELECT_SINGLE_ITEM,
                MULTIPLE_SELECT_13_ITEMS_VPH_5,
                MULTIPLE_SELECT_2_ITEMS_VPH_5,
                SINGLE_SELECT_13_ITEMS_VPH_5,
                SINGLE_SELECT_2_ITEMS_VPH_5,
            ],
            Some(height(6)), /* height of the tuify component */
            Some(width(0)),  /* width of the tuify component. 0 means it will use the
                              * full terminal width */
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await?;

        if user_input.is_empty() {
            println!("User did not select anything");
            // Exit the program.
            return Ok(());
        }

        match user_input.first() {
            Some(input_item) => {
                if input_item == MULTI_LINE_HEADER {
                    multi_line_header().await?;
                } else if input_item == SINGLE_LINE_HEADER {
                    single_line_header().await?;
                } else if input_item == MULTIPLE_SELECT_SINGLE_ITEM {
                    // Multiple select, single item.
                    multiple_select_single_item().await?;
                } else if input_item == MULTIPLE_SELECT_13_ITEMS_VPH_5 {
                    // Multiple select.
                    multiple_select_13_items_vph_5(
                        max_height_row_count,
                        max_width_col_count,
                        sea_foam_style,
                    )
                    .await?;
                } else if input_item == MULTIPLE_SELECT_2_ITEMS_VPH_5 {
                    multiple_select_2_items_vph_5(
                        max_height_row_count,
                        max_width_col_count,
                        sea_foam_style,
                    )
                    .await?;
                } else if input_item == SINGLE_SELECT_13_ITEMS_VPH_5 {
                    // Single select.
                    single_select_13_items_vph_5(
                        max_height_row_count,
                        max_width_col_count,
                        hot_pink_style,
                    )
                    .await?;
                } else if input_item == SINGLE_SELECT_2_ITEMS_VPH_5 {
                    single_select_2_items_vph_5(
                        max_height_row_count,
                        max_width_col_count,
                        default_style,
                    )
                    .await?;
                } else {
                    println!("User did not select anything");
                }
            }
            None => println!("User did not select anything"),
        }

        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Stop logging...");
        });
    });
}

// Multi line header.
async fn multi_line_header() -> miette::Result<()> {
    let header = ast(
        " Please select one or more items. This is a really long heading that just keeps going and if your terminal viewport is small enough, this heading will be clipped",
        new_style!(
            color_fg: {tui_color!(171, 204, 242)}
            color_bg: {tui_color!(31, 36, 46)}
        ),
    );
    let line_5 = inline_vec![header];

    let mut instructions = multi_select_instructions();
    instructions.push(line_5);

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        instructions,
        &[
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
        ],
        Some(height(6)),
        None,
        HowToChoose::Multiple,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    ok!()
}

async fn single_line_header() -> miette::Result<()> {
    let max_width_col_count = usize(*get_terminal_width());

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        "🦜 Please select one or more items. This is an example of a very long header text 🐧. You can pass emoji here 🐥 and text gets clipped off correctly 🐒, based on terminal size".to_string(),
        &[
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
        ],
        Some(height(5)),
        Some(width(max_width_col_count)),
        HowToChoose::Multiple,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    ).await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "User selected something",
            user_input = ?user_input
        );
    });

    ok!()
}

/// Multiple select, single item.
async fn multiple_select_single_item() -> miette::Result<()> {
    let mut instructions = multi_select_instructions();
    let header = ast(
        " Please select one or more items",
        new_style!(
            color_fg: {tui_color!(171, 204, 242)}
            color_bg: {tui_color!(31, 36, 46)}
        ),
    );
    instructions.push(inline_vec![header]);

    let list = inline_vec!["one element"];

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        instructions,
        list,
        Some(height(6)),
        None,
        HowToChoose::Multiple,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    ok!()
}

/// 13 items & viewport height = 5.
async fn multiple_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) -> miette::Result<()> {
    let mut instructions = multi_select_instructions();
    let header = ast(
        " Please select one or more items",
        new_style!(
            color_fg: {tui_color!(229, 239, 123)}
            color_bg: {tui_color!(31, 36, 46)}
        ),
    );
    instructions.push(inline_vec![header]);

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        instructions,
        &[
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
        ],
        Some(height(max_height_row_count)),
        Some(width(max_width_col_count)),
        HowToChoose::Multiple,
        style,
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "User selected something",
            user_input = ?user_input
        );
    });

    ok!()
}

/// 2 items & viewport height = 5.
async fn multiple_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) -> miette::Result<()> {
    let mut instructions = multi_select_instructions();
    let header = ast(
        " Please select one or more items",
        new_style!(
            color_fg: {tui_color!(229, 239, 123)}
            color_bg: {tui_color!(31, 36, 46)}
        ),
    );

    instructions.push(inline_vec![header]);

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        instructions,
        &["item 1 of 2", "item 2 of 2"],
        Some(height(max_height_row_count)),
        Some(width(max_width_col_count)),
        HowToChoose::Multiple,
        style,
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "User selected something",
            user_input = ?user_input
        );
    });

    ok!()
}

/// 13 items & viewport height = 5.
async fn single_select_13_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) -> miette::Result<()> {
    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        "Single select",
        &[
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
        ],
        Some(height(max_height_row_count)),
        Some(width(max_width_col_count)),
        HowToChoose::Single,
        style,
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "User selected something",
            user_input = ?user_input
        );
    });

    ok!()
}

/// 2 items & viewport height = 5.
async fn single_select_2_items_vph_5(
    max_height_row_count: usize,
    max_width_col_count: usize,
    style: StyleSheet,
) -> miette::Result<()> {
    let mut instructions = single_select_instruction();
    let header = ast(
        " Please select one item",
        new_style!(
            color_fg: {tui_color!(171, 204, 242)}
            color_bg: {tui_color!(31, 36, 46)}
        ),
    );
    instructions.push(inline_vec![header]);

    let mut default_io_devices = DefaultIoDevices::default();
    let user_input = choose(
        instructions,
        &["item 1 of 2", "item 2 of 2"],
        Some(height(max_height_row_count)),
        Some(width(max_width_col_count)),
        HowToChoose::Single,
        style,
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if user_input.is_empty() {
        println!("User did not select anything");
        // Exit the program.
        return Ok(());
    }

    println!("User selected: {user_input:?}");

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "User selected something",
            user_input = ?user_input
        );
    });

    ok!()
}

fn multi_select_instructions() -> InlineVec<InlineVec<AnsiStyledText>> {
    let line_1 = {
        let up_and_down = ast(
            " Up or down:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let navigate = ast(
            "     navigate",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![up_and_down, navigate]
    };

    let line_2 = {
        let space = ast(
            " Space:",
            new_style!(
                color_fg: {tui_color!(255, 216, 9)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let select = ast(
            "          select or deselect item",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![space, select]
    };

    let line_3 = {
        let esc = ast(
            " Esc or Ctrl+C:",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let exit = ast(
            "  request_shutdown program",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![esc, exit]
    };

    let line_4 = {
        let return_key = ast(
            " Return:",
            new_style!(
                color_fg: {tui_color!(234, 0, 196)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let confirm = ast(
            "         confirm selection",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![return_key, confirm]
    };

    ast_lines![line_1, line_2, line_3, line_4]
}

fn single_select_instruction() -> InlineVec<InlineVec<AnsiStyledText>> {
    let line_1 = {
        let up_and_down = ast(
            " Up or down:",
            new_style!(
                color_fg: {tui_color!(9, 238, 211)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let navigate = ast(
            "     navigate",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![up_and_down, navigate]
    };

    let line_2 = {
        let esc = ast(
            " Esc or Ctrl+C:",
            new_style!(
                color_fg: {tui_color!(255, 132, 18)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        let exit = ast(
            "  request_shutdown program",
            new_style!(
                color_fg: {tui_color!(94, 103, 111)}
                color_bg: {tui_color!(14, 17, 23)}
            ),
        );

        ast_line![esc, exit]
    };

    let line_3 = {
        let return_key = ast(
            " Return:",
            smallvec![
                ASTStyle::Foreground(ASTColor::Rgb((234, 0, 196).into())),
                ASTStyle::Background(ASTColor::Rgb((14, 17, 23).into())),
            ],
        );

        let confirm = ast(
            "         confirm selection",
            smallvec![
                ASTStyle::Foreground(ASTColor::Rgb((94, 103, 111).into())),
                ASTStyle::Background(ASTColor::Rgb((14, 17, 23).into())),
            ],
        );

        ast_line![return_key, confirm]
    };

    ast_lines![line_1, line_2, line_3]
}

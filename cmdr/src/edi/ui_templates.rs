/*
 *   Copyright (c) 2025 R3BL LLC
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

use r3bl_tui::{DefaultIoDevices,
               HowToChoose,
               InlineString,
               InlineVec,
               StyleSheet,
               ast,
               ast_line,
               choose,
               height,
               inline_vec};

use super::CLIArg;
use crate::{common, edi::ui_str, prefix_single_select_instruction_header};

/// Ask the user to select a file to edit, and return the selected file path (if there is
/// one).
pub async fn handle_multiple_files_not_supported_yet(
    cli_arg: CLIArg,
) -> Option<InlineString> {
    let mut default_io_devices = DefaultIoDevices::default();
    let file_path_options = cli_arg
        .file_paths
        .iter()
        .map(String::as_str)
        .collect::<InlineVec<_>>();
    let header_with_instructions = {
        let last_line = ast_line![ast(
            ui_str::multiple_files_not_supported_yet(),
            common::ui_templates::header_style_default()
        )];
        prefix_single_select_instruction_header(inline_vec![last_line])
    };
    choose(
        header_with_instructions,
        file_path_options.as_slice(),
        Some(height(5)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await
    .ok()
    .and_then(|items| items.into_iter().next())
}

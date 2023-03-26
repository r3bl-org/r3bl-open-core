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

use r3bl_rs_utils_core::*;

use crate::*;

/// This is the main function that the [editor] uses this in order to display the markdown to the
/// user.It is responsible for converting:
/// - from a &[Vec] of [US] which comes from the [editor],
/// - into a [StyleUSSpanLines], which the [editor] will clip & render.
/// ## Arguments
/// - `editor_text` - The text that the user has typed into the editor.
/// - `current_box_computed_style` - The computed style of the box that the editor is in.
pub fn try_parse_and_highlight(
    editor_text_lines: &Vec<US>,
    maybe_current_box_computed_style: &Option<Style>,
) -> CommonResult<StyleUSSpanLines> {
    // Convert the editor text into a string.
    let editor_text_to_string = {
        let mut line_to_str_acc = Vec::<&str>::new();
        for line in editor_text_lines {
            line_to_str_acc.push(line.string.as_str());
            line_to_str_acc.push("\n");
        }
        line_to_str_acc.join("")
    };

    // Try and parse `editor_text_to_string` into a `Document`.
    match parse_markdown(&editor_text_to_string) {
        Ok((_, document)) => Ok(StyleUSSpanLines::from_document(
            &document,
            maybe_current_box_computed_style,
        )),
        Err(_) => CommonError::new_err_with_only_type(CommonErrorType::ParsingError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_parse_and_highlight() -> CommonResult<()> {
        let editor_text_lines = vec![US::new("Hello"), US::new("World")];
        let maybe_current_box_computed_style = None;
        let result =
            try_parse_and_highlight(&editor_text_lines, &maybe_current_box_computed_style)?;

        println!("result: {}", result.pretty_print());

        assert_eq2!(editor_text_lines.len(), result.len());
        assert_eq2!(editor_text_lines[0], result[0][0].text);
        assert_eq2!(editor_text_lines[1], result[1][0].text);

        Ok(())
    }
}

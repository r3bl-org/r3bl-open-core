/*
 *   Copyright (c) 2022 R3BL LLC
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

#[cfg(test)]
mod syntect {
    use r3bl_rs_utils_core::throws;

    use crate::*;

    /// Use a [std::io::Cursor] as a fake [std::fs::File]:
    /// <https://stackoverflow.com/a/41069910/2085356>
    #[test]
    fn load_theme() -> std::io::Result<()> {
        throws!({
            let theme = try_load_r3bl_theme()?;
            dbg!(&theme);
        });
    }

    #[test]
    fn simple_md_highlight() {
        use r3bl_rs_utils_core::{assert_eq2, color};
        use syntect::{easy::*, highlighting::*, parsing::*, util::*};

        // Generate MD content.
        let md_content = {
            #[cfg(target_os = "windows")]
            {
                let mut it = include_str!("test_assets/valid-content.md").to_string();
                it = it.replace("\r\n", "\n");
                it
            }
            #[cfg(not(target_os = "windows"))]
            {
                include_str!("test_assets/valid-content.md").to_string()
            }
        };

        // Load these once at the start of your program.
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme = try_load_r3bl_theme().unwrap();

        // Prepare Markdown syntax highlighting.q
        let md_syntax = syntax_set.find_syntax_by_extension("md").unwrap();
        let mut highlight_lines = HighlightLines::new(md_syntax, &theme);

        let mut line_idx = 0;
        let mut vec_styled_texts = vec![];

        for line in /* LinesWithEndings enables use of newlines mode. */
            LinesWithEndings::from(md_content.as_str())
        {
            let vec_styled_str: Vec<(Style, &str)> =
                highlight_lines.highlight_line(line, &syntax_set).unwrap();

            // // To pretty print the output, use the following:
            // let escaped = as_24_bit_terminal_escaped(&vec_styled_str[..], false);
            // print!("{}", escaped);

            let styled_texts = TuiStyledTexts::from(vec_styled_str);
            line_idx += 1;
            for (col_idx, styled_text) in styled_texts.inner.iter().enumerate() {
                println!("[L#:{line_idx} => C#:{col_idx}] {styled_text:#?}");
            }
            vec_styled_texts.push(styled_texts);
        }

        // 42 lines.
        assert_eq2!(vec_styled_texts.len(), 42);

        // Interrogate first line.
        {
            let line = &vec_styled_texts[0];
            assert_eq2!(line.len(), 4);
            assert_eq2!(line.to_plain_text_us(), "# My Heading\n".into());
            let col1 = &line[0];
            assert_eq2!(col1.get_style().bold, true);
            let col3 = &line[2];
            assert_eq2!(col3.get_style().color_fg.unwrap(), color!(46, 206, 43));
        }

        // Interrogate last line.
        {
            let line = &vec_styled_texts[41];
            assert_eq2!(line.len(), 1);
            assert_eq2!(line.to_plain_text_us(), "--- END ---\n".into());
            let col1 = &line[0];
            assert_eq2!(col1.get_style().color_fg.unwrap(), color!(193, 179, 208));
        }
    }
}

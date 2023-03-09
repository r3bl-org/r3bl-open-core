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

use crate::{constants::CODE_BLOCK_START_PARTIAL, *};

pub fn translate_to_string(doc: Document) -> String {
    let mut acc = vec![];
    for block in doc {
        match block {
            Block::Heading((level, line)) => acc.push(translate_header(&level, line.to_vec())),
            Block::UnorderedList(lines) => acc.push(translate_unordered_list(lines.to_vec())),
            Block::OrderedList(lines) => acc.push(translate_ordered_list(lines.to_vec())),
            Block::CodeBlock(code_block) => acc.push(translate_codeblock_lines(&code_block)),
            Block::Text(line) => acc.push(translate_line(line.to_vec())),
            _ => {}
        }
    }
    acc.join("")
}

fn translate_bold(input: &str) -> String { format!("<b>{input}</b>") }

fn translate_italic(input: &str) -> String { format!("<i>{input}</i>") }

fn translate_inline_code(input: &str) -> String { format!("<code>{input}</code>") }

fn translate_link(link_text: &str, url: &str) -> String {
    format!("<a href=\"{url}\">{link_text}</a>")
}

fn translate_image(link_text: &str, url: &str) -> String {
    format!("<img src=\"{url}\" alt=\"{link_text}\" />")
}

fn translate_list_elements(lines: Vec<Fragments>) -> String {
    lines
        .iter()
        .map(|line| format!("<li>{}</li>", translate_text(line.to_vec())))
        .collect::<Vec<String>>()
        .join("")
}

fn translate_header(heading_level: &Level, text: Fragments) -> String {
    let heading_level_number = (*heading_level) as u8;
    format!(
        "<h{}>{}</h{}>",
        heading_level_number,
        translate_text(text),
        heading_level_number
    )
}

fn translate_unordered_list(lines: Vec<Fragments>) -> String {
    format!("<ul>{}</ul>", translate_list_elements(lines.to_vec()))
}

fn translate_ordered_list(lines: Vec<Fragments>) -> String {
    format!("<ol>{}</ol>", translate_list_elements(lines.to_vec()))
}

fn translate_codeblock_lines(code_block_lines: &Vec<CodeBlockLine>) -> String {
    let lang = {
        if let Some(lang) = code_block_lines.get(0) {
            lang.language
        } else {
            None
        }
    };

    let mut acc = vec![];
    code_block_lines.iter().for_each(|line| match line.content {
        CodeBlockLineContent::Text(text) => acc.push(text),
        CodeBlockLineContent::EmptyLine => acc.push(""),
        _ => {}
    });
    let text = acc.join("\n");

    match lang {
        Some(language) => format!("<pre><code class=\"lang-{language}\">\n{text}\n</code></pre>"),
        None => format!("<pre><code>\n{text}\n</code></pre>"),
    }
}

fn translate_line(text: Fragments) -> String {
    let line = translate_text(text);
    if !line.is_empty() {
        format!("<p>{line}</p>")
    } else {
        line
    }
}

fn translate_text(text: Fragments) -> String {
    text.iter()
        .map(|part| match part {
            Fragment::Bold(text) => translate_bold(text),
            Fragment::Italic(text) => translate_italic(text),
            Fragment::BoldItalic(text) => translate_italic(&translate_bold(text)),
            Fragment::InlineCode(code) => translate_inline_code(code),
            Fragment::Link((text, url)) => translate_link(text, url),
            Fragment::Image((text, url)) => translate_image(text, url),
            Fragment::Checkbox(flag) => match flag {
                true => "[x]".to_string(),
                false => "[ ]".to_string(),
            },
            Fragment::Plain(text) => text.to_string(),
        })
        .collect::<Vec<String>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_translate_bold() {
        assert_eq2!(translate_bold("bold af"), String::from("<b>bold af</b>"));
    }

    #[test]
    fn test_translate_italic() {
        assert_eq2!(
            translate_italic("italic af"),
            String::from("<i>italic af</i>")
        );
    }

    #[test]
    fn test_translate_inline_code() {
        assert_eq2!(
            translate_inline_code("code af"),
            String::from("<code>code af</code>")
        );
    }

    #[test]
    fn test_translate_link() {
        assert_eq2!(
            translate_link("click me!", "https://github.com"),
            String::from("<a href=\"https://github.com\">click me!</a>")
        );
    }

    #[test]
    fn test_translate_image() {
        assert_eq2!(
            translate_image("alt text", "https://github.com"),
            String::from("<img src=\"https://github.com\" alt=\"alt text\" />")
        );
    }

    #[test]
    fn test_translate_text() {
        let x = translate_text(vec![
            Fragment::Plain("Foobar is a Python library for dealing with word pluralization."),
            Fragment::Bold("bold"),
            Fragment::Italic("italic"),
            Fragment::InlineCode("code"),
            Fragment::Link(("tag", "https://link.com")),
            Fragment::Image(("tag", "https://link.com")),
            Fragment::Plain(". the end!"),
        ]);
        assert_eq2!(x, String::from("Foobar is a Python library for dealing with word pluralization.<b>bold</b><i>italic</i><code>code</code><a href=\"https://link.com\">tag</a><img src=\"https://link.com\" alt=\"tag\" />. the end!"));
        let x = translate_text(vec![]);
        assert_eq2!(x, String::from(""));
    }

    #[test]
    fn test_translate_header() {
        assert_eq2!(
            translate_header(&Level::Heading1, vec![Fragment::Plain("Foobar")]),
            String::from("<h1>Foobar</h1>")
        );
    }

    #[test]
    fn test_translate_list_elements() {
        assert_eq2!(
            translate_list_elements(vec![
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
            ]),
            String::from("<li>Foobar</li><li>Foobar</li><li>Foobar</li><li>Foobar</li>")
        );
    }

    #[test]
    fn test_translate_unordered_list() {
        assert_eq2!(
            translate_unordered_list(vec![
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
            ]),
            String::from("<ul><li>Foobar</li><li>Foobar</li><li>Foobar</li><li>Foobar</li></ul>")
        );
    }

    #[test]
    fn test_translate_ordered_list() {
        assert_eq2!(
            translate_ordered_list(vec![
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
                vec![Fragment::Plain("Foobar")],
            ]),
            String::from("<ol><li>Foobar</li><li>Foobar</li><li>Foobar</li><li>Foobar</li></ol>")
        );
    }

    #[test]
    fn test_translate_codeblock() {
        let it = convert_into_code_block_lines(
            Some("python"),
            vec![
                "import foobar",
                "",
                "foobar.pluralize('word') # returns 'words'",
                "foobar.pluralize('goose') # returns 'geese'",
                "foobar.singularize('phenomena') # returns 'phenomenon'",
            ],
        );
        let lhs = translate_codeblock_lines(&it);
        let rhs = String::from(raw_strings::CODE_BLOCK_HTML);
        assert_eq2!(lhs, rhs);
    }

    #[test]
    fn test_translate_line() {
        assert_eq2!(
            translate_line(vec![
                Fragment::Plain("Foobar"),
                Fragment::Bold("Foobar"),
                Fragment::Italic("Foobar"),
                Fragment::InlineCode("Foobar"),
            ]),
            String::from("<p>Foobar<b>Foobar</b><i>Foobar</i><code>Foobar</code></p>")
        );
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod raw_strings {
    pub const CODE_BLOCK_HTML: &str =
r#"<pre><code class="lang-python">
import foobar

foobar.pluralize('word') # returns 'words'
foobar.pluralize('goose') # returns 'geese'
foobar.singularize('phenomena') # returns 'phenomenon'
</code></pre>"#;
}

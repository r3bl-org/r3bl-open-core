// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module contains code for converting between syntect styled texts and tui styled
//! texts.
//!
//! A [Vec] or [`crate::List`] of styled text represents a single line of text in an
//! editor component, which is the output of a syntax highlighter (that takes plain text
//! and returns the styled text).
//!
//! There is a major difference in doing this conversion which is:
//! - tui styled texts are styled unicode strings,
//! - while syntect styled texts are styled plain text strings.
//!
//! This requires the conversion code to perform the following steps:
//! 1. Convert the syntect [`SyntectStyleStrSpanLine`] into a [`StyleUSSpanLine`].
//! 2. Then convert [`StyleUSSpanLine`] into a [`TuiStyledTexts`].

use super::{StyleUSSpan, StyleUSSpanLine};
use crate::{RenderList, TuiColor, TuiStyle, TuiStyleAttribs, TuiStyledTexts, tui_color,
            tui_style_attrib, tui_styled_text};
use syntect::parsing::SyntaxSet;

// Type aliases for syntect types.

type SyntectStyle = syntect::highlighting::Style;
type SyntectFontStyle = syntect::highlighting::FontStyle;
type SyntectColor = syntect::highlighting::Color;

/// Span are chunks of a text that have an associated style. There are usually multiple
/// spans in a line of text.
pub type SyntectStyleStrSpan<'a> = (SyntectStyle, &'a str);

/// A line of text is made up of multiple [`SyntectStyleStrSpan`]s.
pub type SyntectStyleStrSpanLine<'a> = Vec<SyntectStyleStrSpan<'a>>;

/// Maps common language names to their file extensions for syntax highlighting.
/// This allows markdown code blocks to use either language names (e.g., "rust")
/// or file extensions (e.g., "rs").
///
/// TypeScript, TOML, SCSS, Kotlin, Swift, and Dockerfile are not supported by `syntect`.
/// In order to add these languages we need to add custom `.sublime-syntax` files later
/// for better support of TypeScript, TOML, etc., `syntect` makes that easy:
/// ```no_run
/// # use syntect::parsing::SyntaxSet;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Example of loading additional syntaxes
/// let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
/// builder.add_from_folder("path/to/extra/syntaxes", true)?;
/// let syntax_set = builder.build();
/// # Ok(())
/// # }
/// ```
fn map_language_to_extension(lang: &str) -> &str {
    match lang {
        // Direct mappings
        "python" => "py",
        "golang" | "go" => "go",
        "csharp" | "c#" => "cs",
        "cpp" | "c++" => "cpp",
        "objective-c" | "objc" => "m",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "html" => "html",
        "xml" => "xml",
        "markdown" | "md" => "md",
        "ruby" | "rb" => "rb",
        "r" => "r",
        "sql" => "sql",
        "makefile" => "makefile",

        // Languages that fall back to JavaScript.
        "javascript" | "typescript" | "ts" => "js",

        // Languages that fall back to CSS.
        "css" | "scss" | "sass" => "css",

        // Languages that fall back to Java.
        "java" | "kotlin" | "kt" => "java",

        // Languages that fall back to Rust.
        "rust" | "toml" | "swift" => "rs",

        // Languages that fall back to shell.
        "shell" | "bash" | "sh" | "dockerfile" => "sh",

        // Default: assume it's already a file extension
        _ => lang,
    }
}

pub fn try_get_syntax_ref<'a>(
    syntax_set: &'a SyntaxSet,
    file_extension: &'a str,
) -> Option<&'a syntect::parsing::SyntaxReference> {
    let mapped_extension = map_language_to_extension(file_extension);
    syntax_set.find_syntax_by_extension(mapped_extension)
}

#[must_use]
pub fn convert_style_from_syntect_to_tui(st_style: SyntectStyle) -> TuiStyle {
    TuiStyle {
        color_fg: Some(convert_color_from_syntect_to_tui(st_style.foreground)),
        color_bg: Some(convert_color_from_syntect_to_tui(st_style.background)),
        attribs: TuiStyleAttribs {
            bold: st_style
                .font_style
                .contains(SyntectFontStyle::BOLD)
                .then_some(tui_style_attrib::Bold),
            italic: st_style
                .font_style
                .contains(SyntectFontStyle::ITALIC)
                .then_some(tui_style_attrib::Italic),
            underline: st_style
                .font_style
                .contains(SyntectFontStyle::UNDERLINE)
                .then_some(tui_style_attrib::Underline),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[must_use]
pub fn convert_color_from_syntect_to_tui(st_color: SyntectColor) -> TuiColor {
    tui_color!(st_color.r, st_color.g, st_color.b)
}

#[must_use]
pub fn convert_highlighted_line_from_syntect_to_tui(
    syntect_highlighted_line: &SyntectStyleStrSpanLine<'_>,
) -> StyleUSSpanLine {
    fn convert(vec_styled_str: &SyntectStyleStrSpanLine<'_>) -> StyleUSSpanLine {
        let mut it: StyleUSSpanLine = RenderList::default();

        for (style, text) in vec_styled_str {
            let my_style = convert_style_from_syntect_to_tui(*style);
            it.push(StyleUSSpan::new(my_style, text));
        }

        it
    }

    let mut it = convert(syntect_highlighted_line);

    // Remove the background color from each style in the theme.
    for span in it.iter_mut() {
        span.style.remove_bg_color();
    }

    it
}

#[must_use]
pub fn convert_span_line_from_syntect_to_tui_styled_texts(
    syntect_styles: &SyntectStyleStrSpanLine<'_>,
) -> TuiStyledTexts {
    let mut acc = TuiStyledTexts::default();
    for (syntect_style, text) in syntect_styles {
        let my_style = convert_style_from_syntect_to_tui(*syntect_style);
        acc += tui_styled_text!(@style: my_style, @text: (*text).to_string());
    }
    acc
}

#[cfg(test)]
mod tests_simple_md_highlight {
    use crate::{ConvertToPlainText, TuiStyledTexts, assert_eq2,
                convert_span_line_from_syntect_to_tui_styled_texts,
                get_cached_syntax_set, get_cached_theme, tui_color};
    use syntect::{easy::HighlightLines, highlighting::Style, util::LinesWithEndings};

    #[test]
    fn simple_md_highlight() {
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
        let syntax_set = get_cached_syntax_set();
        let theme = get_cached_theme();

        // Prepare Markdown syntax highlighting.
        let md_syntax = syntax_set.find_syntax_by_extension("md").unwrap();
        let mut highlight_lines = HighlightLines::new(md_syntax, theme);

        let mut line_idx = 0;
        let mut vec_styled_texts = vec![];

        for line in /* LinesWithEndings enables use of newlines mode. */
            LinesWithEndings::from(md_content.as_str())
        {
            let vec_styled_str: Vec<(Style, &str)> =
                highlight_lines.highlight_line(line, syntax_set).unwrap();

            // // To pretty print the output, use the following:
            // let escaped = as_24_bit_terminal_escaped(&vec_styled_str[..], false);
            // print!("{}", escaped);

            let styled_texts: TuiStyledTexts =
                convert_span_line_from_syntect_to_tui_styled_texts(&vec_styled_str);

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
            assert_eq2!(line.to_plain_text(), "# My Heading\n");
            let col1 = &line[0];
            assert!(col1.get_style().attribs.bold.is_some());
            let col3 = &line[2];
            assert_eq2!(col3.get_style().color_fg.unwrap(), tui_color!(46, 206, 43));
        }

        // Interrogate last line.
        {
            let line = &vec_styled_texts[41];
            assert_eq2!(line.len(), 1);
            assert_eq2!(line.to_plain_text(), "--- END ---\n");
            let col1 = &line[0];
            assert_eq2!(
                col1.get_style().color_fg.unwrap(),
                tui_color!(193, 179, 208)
            );
        }
    }
}

#[cfg(test)]
mod tests_convert_span_line_and_highlighted_line {
    use crate::{TuiStyledTexts, assert_eq2,
                convert_span_line_from_syntect_to_tui_styled_texts, tui_color};

    #[test]
    fn syntect_conversion() {
        let st_color_1 = syntect::highlighting::Color {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };

        let st_color_2 = syntect::highlighting::Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };

        let vec_styled_str: Vec<(syntect::highlighting::Style, &str)> = vec![
            // item 1.
            (
                syntect::highlighting::Style {
                    foreground: st_color_1,
                    background: st_color_1,
                    font_style: syntect::highlighting::FontStyle::empty(),
                },
                "st_color_1",
            ),
            // item 2.
            (
                syntect::highlighting::Style {
                    foreground: st_color_2,
                    background: st_color_2,
                    font_style: syntect::highlighting::FontStyle::BOLD,
                },
                "st_color_2",
            ),
            // item 3.
            (
                syntect::highlighting::Style {
                    foreground: st_color_1,
                    background: st_color_2,
                    font_style: syntect::highlighting::FontStyle::UNDERLINE
                        | syntect::highlighting::FontStyle::BOLD
                        | syntect::highlighting::FontStyle::ITALIC,
                },
                "st_color_1 and 2",
            ),
        ];

        let styled_texts: TuiStyledTexts =
            convert_span_line_from_syntect_to_tui_styled_texts(&vec_styled_str);

        // Should have 3 items.
        assert_eq2!(styled_texts.len(), 3);

        // item 1.
        {
            assert_eq2!(styled_texts[0].get_text(), "st_color_1");
            assert_eq2!(
                styled_texts[0].get_style().color_fg.unwrap(),
                tui_color!(255, 255, 255)
            );
            assert_eq2!(
                styled_texts[0].get_style().color_bg.unwrap(),
                tui_color!(255, 255, 255)
            );
        }

        // item 2.
        {
            assert_eq2!(styled_texts[1].get_text(), "st_color_2");
            assert_eq2!(
                styled_texts[1].get_style().color_fg.unwrap(),
                tui_color!(0, 0, 0)
            );
            assert_eq2!(
                styled_texts[1].get_style().color_bg.unwrap(),
                tui_color!(0, 0, 0)
            );
            assert!(styled_texts[1].get_style().attribs.bold.is_some());
        }

        // item 3.
        {
            assert_eq2!(styled_texts[2].get_text(), "st_color_1 and 2");
            assert_eq2!(
                styled_texts[2].get_style().color_fg.unwrap(),
                tui_color!(255, 255, 255)
            );
            assert_eq2!(
                styled_texts[2].get_style().color_bg.unwrap(),
                tui_color!(0, 0, 0)
            );
            assert!(styled_texts[2].get_style().attribs.bold.is_some());
            assert!(styled_texts[2].get_style().attribs.underline.is_some());
        }
    }
}

#[cfg(test)]
mod tests_convert_style_and_color {
    use crate::{CommonResult, InlineVec, TuiStyle, TuiStyleAttribs, TuiStylesheet,
                assert_eq2, ch, console_log, convert_style_from_syntect_to_tui,
                get_tui_style, get_tui_styles, new_style, throws, tui_color,
                tui_style_attrib, tui_style_id, tui_stylesheet};
    use smallvec::smallvec;

    #[test]
    fn syntect_style_conversion() {
        let st_style: syntect::highlighting::Style = syntect::highlighting::Style {
            foreground: syntect::highlighting::Color::WHITE,
            background: syntect::highlighting::Color::BLACK,
            font_style: syntect::highlighting::FontStyle::BOLD
                | syntect::highlighting::FontStyle::ITALIC
                | syntect::highlighting::FontStyle::UNDERLINE,
        };
        let style = convert_style_from_syntect_to_tui(st_style);
        assert_eq2!(style.color_fg.unwrap(), tui_color!(255, 255, 255));
        assert_eq2!(style.color_bg.unwrap(), tui_color!(0, 0, 0));
        assert!(style.attribs.bold.is_some());
        assert!(style.attribs.underline.is_some());
    }

    #[test]
    fn test_cascade_style() {
        let style_bold_green_fg = new_style!(
            id: {1} // "bold_green_fg"
            bold
            color_fg: {tui_color!(green)}
        );

        let style_dim = new_style!(
            id: {2} // "dim"
            dim
        );

        let style_yellow_bg = new_style!(
            id: {3} // "yellow_bg"
            color_bg: {tui_color!(yellow)}
        );

        let style_padding = new_style!(
            id: {4} // "padding"
            padding: {2}
        );

        let style_red_fg = new_style!(
            id: {5} // "red_fg"
            color_fg: {tui_color!(red)}
        );

        let style_padding_another = new_style!(
            id: {6} // "padding"
            padding: {1}
        );

        let my_style = style_bold_green_fg
            + style_dim
            + style_yellow_bg
            + style_padding
            + style_red_fg
            + style_padding_another;

        console_log!(my_style);

        assert_eq2!(my_style.padding.unwrap(), ch(3));
        assert_eq2!(my_style.color_bg.unwrap(), tui_color!(yellow));
        assert_eq2!(my_style.color_fg.unwrap(), tui_color!(red));
        assert!(my_style.attribs.bold.is_some());
        assert!(my_style.attribs.dim.is_some());
        assert!(my_style.computed.is_some());
        assert!(my_style.attribs.underline.is_none());
    }

    #[test]
    fn test_stylesheet_find_style_by_id() {
        let mut stylesheet = TuiStylesheet::new();

        let style1 = make_a_style(1);
        let result = stylesheet.add_style(style1);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 1);

        let style2 = make_a_style(2);
        let result = stylesheet.add_style(style2);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 2);

        // No macro.
        assert_eq2!(stylesheet.find_style_by_id(1).unwrap().id, tui_style_id(1));
        assert_eq2!(stylesheet.find_style_by_id(2).unwrap().id, tui_style_id(2));
        assert!(stylesheet.find_style_by_id(3).is_none());
        // Macro.
        assert_eq2!(
            get_tui_style!(@from: stylesheet, 1).unwrap().id,
            tui_style_id(1)
        );
        assert_eq2!(
            get_tui_style!(@from: stylesheet, 2).unwrap().id,
            tui_style_id(2)
        );
        assert!(get_tui_style!(@from: stylesheet, 3).is_none());
    }

    #[test]
    fn test_stylesheet_find_styles_by_ids() {
        fn assertions_for_find_styles_by_ids(result: Option<&InlineVec<TuiStyle>>) {
            assert_eq2!(result.unwrap().len(), 2);
            assert_eq2!(result.unwrap()[0].id, tui_style_id(1));
            assert_eq2!(result.unwrap()[1].id, tui_style_id(2));
        }

        let mut stylesheet = TuiStylesheet::new();

        let style1 = make_a_style(1);
        let result = stylesheet.add_style(style1);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 1);

        let style2 = make_a_style(2);
        let result = stylesheet.add_style(style2);
        result.unwrap();
        assert_eq2!(stylesheet.styles.len(), 2);

        // Contains.
        assertions_for_find_styles_by_ids(
            stylesheet.find_styles_by_ids(&[1, 2]).as_ref(),
        );
        assertions_for_find_styles_by_ids(
            get_tui_styles!(
                @from: &stylesheet,
                [1, 2]
            )
            .as_ref(),
        );
        // Does not contain.
        assert_eq2!(stylesheet.find_styles_by_ids(&[3, 4]), None);
        assert_eq2!(get_tui_styles!(@from: stylesheet, [3, 4]), None);
    }

    #[test]
    fn test_stylesheet_builder() -> CommonResult<()> {
        throws!({
            let id_2 = 2;
            let style1 = make_a_style(1);
            let mut stylesheet = tui_stylesheet! {
                style1,
                new_style!(
                    id: {id_2} /* using a variable instead of string literal */
                    padding: {1}
                    color_bg: {tui_color!(55, 55, 248)}
                ),
                make_a_style(3),
                smallvec![
                    new_style!(
                        id: {4}
                        padding: {1}
                        color_bg: {tui_color!(55, 55, 248)}
                    ),
                    new_style!(
                        id: {5}
                        padding: {1}
                        color_bg: {tui_color!(85, 85, 255)}
                    ),
                ],
                make_a_style(6)
            };

            assert_eq2!(stylesheet.styles.len(), 6);
            assert_eq2!(stylesheet.find_style_by_id(1).unwrap().id, tui_style_id(1));
            assert_eq2!(stylesheet.find_style_by_id(2).unwrap().id, tui_style_id(2));
            assert_eq2!(stylesheet.find_style_by_id(3).unwrap().id, tui_style_id(3));
            assert_eq2!(stylesheet.find_style_by_id(4).unwrap().id, tui_style_id(4));
            assert_eq2!(stylesheet.find_style_by_id(5).unwrap().id, tui_style_id(5));
            assert_eq2!(stylesheet.find_style_by_id(6).unwrap().id, tui_style_id(6));
            assert!(stylesheet.find_style_by_id(7).is_none());

            let result = stylesheet.find_styles_by_ids(&[1, 2]);
            assert_eq2!(result.as_ref().unwrap().len(), 2);
            assert_eq2!(result.as_ref().unwrap()[0].id, tui_style_id(1));
            assert_eq2!(result.as_ref().unwrap()[1].id, tui_style_id(2));
            assert_eq2!(stylesheet.find_styles_by_ids(&[13, 41]), None);
            let style7 = make_a_style(7);
            let result = stylesheet.add_style(style7);
            result.unwrap();
            assert_eq2!(stylesheet.styles.len(), 7);
            assert_eq2!(stylesheet.find_style_by_id(7).unwrap().id, tui_style_id(7));
        });
    }

    /// Helper function.
    fn make_a_style(arg: u8) -> TuiStyle {
        TuiStyle {
            id: tui_style_id(arg),
            attribs: TuiStyleAttribs {
                dim: Some(tui_style_attrib::Dim),
                bold: Some(tui_style_attrib::Bold),
                ..Default::default()
            },
            color_fg: tui_color!(0, 0, 0).into(),
            color_bg: tui_color!(0, 0, 0).into(),
            ..TuiStyle::default()
        }
    }
}

#[cfg(test)]
mod tests_language_mapping {
    use super::*;
    use crate::get_cached_syntax_set;

    #[test]
    fn test_map_language_to_extension() {
        // Test Rust mapping.
        assert_eq!(map_language_to_extension("rust"), "rs");
        assert_eq!(map_language_to_extension("rs"), "rs");

        // Test JavaScript mapping.
        assert_eq!(map_language_to_extension("javascript"), "js");
        assert_eq!(map_language_to_extension("js"), "js");

        // Test TypeScript mapping (falls back to JS)
        assert_eq!(map_language_to_extension("typescript"), "js");
        assert_eq!(map_language_to_extension("ts"), "js");

        // Test Python mapping.
        assert_eq!(map_language_to_extension("python"), "py");
        assert_eq!(map_language_to_extension("py"), "py");

        // Test Go mapping
        assert_eq!(map_language_to_extension("golang"), "go");
        assert_eq!(map_language_to_extension("go"), "go");

        // Test shell/bash mapping
        assert_eq!(map_language_to_extension("shell"), "sh");
        assert_eq!(map_language_to_extension("bash"), "sh");
        assert_eq!(map_language_to_extension("sh"), "sh");

        // Test C# mapping
        assert_eq!(map_language_to_extension("csharp"), "cs");
        assert_eq!(map_language_to_extension("c#"), "cs");
        assert_eq!(map_language_to_extension("cs"), "cs");

        // Test fallback mappings.
        assert_eq!(map_language_to_extension("toml"), "rs"); // Falls back to Rust
        assert_eq!(map_language_to_extension("scss"), "css"); // Falls back to CSS
        assert_eq!(map_language_to_extension("sass"), "css"); // Falls back to CSS
        assert_eq!(map_language_to_extension("kotlin"), "java"); // Falls back to Java
        assert_eq!(map_language_to_extension("swift"), "rs"); // Falls back to Rust
        assert_eq!(map_language_to_extension("dockerfile"), "sh"); // Falls back to shell

        // Test unknown language (should return as-is)
        assert_eq!(map_language_to_extension("unknown"), "unknown");
        assert_eq!(map_language_to_extension("xyz"), "xyz");
    }

    #[test]
    fn test_try_get_syntax_ref_with_language_names() {
        let syntax_set = get_cached_syntax_set();

        // Test that both "rust" and "rs" resolve to the same syntax.
        let rust_syntax = try_get_syntax_ref(syntax_set, "rust");
        let rs_syntax = try_get_syntax_ref(syntax_set, "rs");

        assert!(rust_syntax.is_some());
        assert!(rs_syntax.is_some());
        assert_eq!(rust_syntax.unwrap().name, rs_syntax.unwrap().name);

        // Test other common language mappings.
        assert!(try_get_syntax_ref(syntax_set, "javascript").is_some());
        assert!(try_get_syntax_ref(syntax_set, "python").is_some());
        assert!(try_get_syntax_ref(syntax_set, "golang").is_some());

        // Test that unknown languages return None.
        assert!(try_get_syntax_ref(syntax_set, "unknown_language").is_none());
    }

    #[test]
    fn test_available_syntaxes_and_mappings() {
        let syntax_set = syntect::parsing::SyntaxSet::load_defaults_newlines();

        println!("Available syntaxes and their extensions:");
        for syntax in syntax_set.syntaxes() {
            println!(
                "Syntax: {} -> Extensions: {:?}",
                syntax.name, syntax.file_extensions
            );
        }

        // Test our mappings.
        let test_mappings = [
            ("rust", "rs"),
            ("javascript", "js"),
            ("ts", "js"),         // Falls back to JS
            ("typescript", "js"), // Falls back to JS
            ("python", "py"),
            ("golang", "go"),
            ("go", "go"),
            ("csharp", "cs"),
            ("c#", "cs"),
            ("cpp", "cpp"),
            ("c++", "cpp"),
            ("objective-c", "m"),
            ("objc", "m"),
            ("shell", "sh"),
            ("bash", "sh"),
            ("sh", "sh"),
            ("yaml", "yaml"),
            ("yml", "yaml"),
            ("toml", "rs"), // Falls back to Rust
            ("json", "json"),
            ("html", "html"),
            ("css", "css"),
            ("scss", "css"), // Falls back to CSS
            ("sass", "css"), // Falls back to CSS
            ("xml", "xml"),
            ("markdown", "md"),
            ("md", "md"),
            ("ruby", "rb"),
            ("rb", "rb"),
            ("java", "java"),
            ("kotlin", "java"), // Falls back to Java
            ("kt", "java"),     // Falls back to Java
            ("swift", "rs"),    // Falls back to Rust
            ("r", "r"),
            ("sql", "sql"),
            ("dockerfile", "sh"), // Falls back to shell
            ("makefile", "makefile"),
        ];

        println!("\nTesting our mappings:");
        for (lang, expected_ext) in &test_mappings {
            let mapped_ext = map_language_to_extension(lang);
            assert_eq!(mapped_ext, *expected_ext, "Mapping failed for {lang}");

            let syntax_ref = syntax_set.find_syntax_by_extension(expected_ext);
            println!(
                "{} -> {} : {}",
                lang,
                expected_ext,
                if syntax_ref.is_some() { "✓" } else { "✗" }
            );
        }
    }
}

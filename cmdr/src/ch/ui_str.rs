// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::Report;
use r3bl_tui::{HyperlinkSupport, InlineString, fg_color, get_terminal_width,
               global_hyperlink_support, glyphs, hyperlink::format_file_hyperlink,
               inline_string, tui_color};

use super::types::SavedImageInfo;
use crate::common::fmt;

/// Build a horizontal separator line similar to the log formatter
fn build_horizontal_separator() -> InlineString {
    let terminal_width = get_terminal_width();
    let separator = glyphs::TOP_UNDERLINE_GLYPH.repeat(terminal_width.as_usize());
    fg_color(tui_color!(dark_lizard_green), &separator).to_small_str()
}

/// Generate error message for unrecoverable errors
#[must_use]
pub fn unrecoverable_error_msg(report: Report) -> InlineString {
    inline_string!(
        "{a}{b}\n{c}",
        a = fmt::error("❌ Could not run ch due to the following problem"),
        b = fmt::colon(),
        c = fmt::error(report)
    )
}

/// Generate error message when ~/.claude.json file is not found
#[must_use]
pub fn claude_config_file_not_found_msg() -> InlineString {
    inline_string!(
        "{a}{b}\n\n{c}",
        a = fmt::error("❌ Claude configuration file not found"),
        b = fmt::colon(),
        c = fmt::normal(
            "Could not find ~/.claude.json file. Make sure you have run Claude Code at least once."
        )
    )
}

/// Generate error message when terminal is not interactive
#[must_use]
pub fn terminal_not_interactive_msg() -> InlineString {
    inline_string!(
        "{a}{b}\n\n{c}",
        a = fmt::error("❌ Terminal not interactive"),
        b = fmt::colon(),
        c = fmt::normal(
            "ch requires an interactive terminal to display the prompt selection interface."
        )
    )
}

/// Generate error message when no prompts are found for current project
#[must_use]
pub fn no_prompts_found_msg(project_path: &str) -> InlineString {
    inline_string!(
        "{a}{b}\n\n{c} {d}",
        a = fmt::error("❌ No prompts found"),
        b = fmt::colon(),
        c = fmt::normal("No Claude Code prompt history found for project:"),
        d = fmt::emphasis(project_path)
    )
}

/// Generate header message for prompt selection (raw text for use with `choose()`)
#[must_use]
pub fn select_prompt_header_msg_raw(project_path: &str, total_prompts: usize) -> String {
    format!("📋 Select a prompt from {project_path} ({total_prompts} available)")
}

/// Generate success message when prompt is copied
#[must_use]
pub fn prompt_copied_msg(prompt: &str) -> InlineString {
    let separator = build_horizontal_separator();
    inline_string!(
        "{a}{b}\n\n{separator}\n{c}\n{separator}",
        a = fmt::emphasis("✅ Prompt copied to clipboard"),
        b = fmt::colon(),
        c = fmt::normal(prompt),
        separator = separator
    )
}

/// Generate message when user cancels selection
#[must_use]
pub fn selection_cancelled_msg() -> InlineString { fmt::normal("No prompt selected") }

/// Generate success message when prompt with images is copied
#[must_use]
pub fn prompt_with_images_copied_msg(
    prompt: &str,
    image_count: usize,
    saved_images: &[SavedImageInfo],
) -> InlineString {
    use std::fmt::Write;

    let downloads_dir = saved_images.first().map_or_else(
        || "Downloads".to_string(),
        |img| {
            img.filepath
                .parent()
                .unwrap_or(&img.filepath)
                .display()
                .to_string()
        },
    );

    let image_text = if image_count == 1 { "image" } else { "images" };
    let separator = build_horizontal_separator();

    // Build the image files list with optional hyperlinks
    let mut image_list = String::new();
    let hyperlink_support = global_hyperlink_support::detect();

    for (index, image_info) in saved_images.iter().enumerate() {
        let file_path_display = match hyperlink_support {
            HyperlinkSupport::Supported => {
                // Create clickable hyperlink
                format_file_hyperlink(&image_info.filepath)
            }
            HyperlinkSupport::NotSupported => {
                // Plain text fallback
                image_info.filepath.display().to_string()
            }
        };
        writeln!(&mut image_list, "{}. {}", index + 1, file_path_display)
            .expect("Writing to String should never fail");
    }
    // Remove trailing newline
    if image_list.ends_with('\n') {
        image_list.pop();
    }

    inline_string!(
        "{a}{b}\n\n{separator}\n{c}\n\n{d} {e} {f} saved to {g}:\n{h}\n{separator}",
        a = fmt::emphasis("✅ Prompt copied to clipboard"),
        b = fmt::colon(),
        c = fmt::normal(prompt),
        d = fmt::normal("📷"),
        e = fmt::emphasis(image_count),
        f = fmt::normal(image_text),
        g = fmt::emphasis(&downloads_dir),
        h = fmt::normal(&image_list),
        separator = separator
    )
}

/// Generate error message when prompt is copied but images failed to save
#[must_use]
pub fn prompt_copied_with_image_error_msg(prompt: &str) -> InlineString {
    let separator = build_horizontal_separator();
    inline_string!(
        "{a}{b}\n\n{separator}\n{c}\n\n{d}\n{separator}",
        a = fmt::emphasis("⚠️ Prompt copied to clipboard"),
        b = fmt::colon(),
        c = fmt::normal(prompt),
        d = fmt::error("Failed to save images from this prompt"),
        separator = separator
    )
}

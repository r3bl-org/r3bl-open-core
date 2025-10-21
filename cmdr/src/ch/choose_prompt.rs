// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CLIArg, image_handler, prompt_history,
            types::{ChResult, HistoryItem, ImageContent},
            ui_str};
use crate::prefix_single_select_instruction_header;
use r3bl_tui::{CommonResult, DefaultIoDevices, InlineString, TTYResult, cli_text_line,
               choose, cli_text, height, inline_vec, is_fully_uninteractive_terminal,
               readline_async::{HowToChoose, StyleSheet},
               tui::editor::editor_buffer::{clipboard_service::SystemClipboard,
                                            clipboard_support::ClipboardService}};

/// Handle the main ch command logic
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get prompt history for the current project
/// - Failed to display the prompt selection UI
/// - Failed to copy selected prompt to clipboard
/// - Failed to parse or handle pasted image contents
pub async fn handle_ch_command(_cli_arg: CLIArg) -> CommonResult<ChResult> {
    // Check if terminal is interactive.
    if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
        return Ok(ChResult::TerminalNotInteractive);
    }

    // Get prompt history for current project.
    let (project_path, history) = prompt_history::get_prompts_for_current_project()?;

    // Check if we have any prompts.
    if history.is_empty() {
        return Ok(ChResult::NoPromptsFound { project_path });
    }

    // Prepare display/original pairs for ALL history items.
    let prompt_pairs: Vec<(String, String)> = history
        .iter()
        .map(|item| {
            let original = item.display.clone(); // Keep original with newlines intact
            let mut display = original.replace(['\n', '\r'], " "); // Strip newlines only for display

            // Add image indicator prefix to display only.
            let image_count = image_handler::count_images_in_history_item(item);
            if image_count > 0 {
                let prefix = if image_count == 1 {
                    "[📷] ".to_string()
                } else {
                    format!("[📷x{image_count}] ")
                };
                display = format!("{prefix}{display}");
            }

            (display, original) // display for choose(), original for output/clipboard
        })
        .collect();

    // Extract display texts for the choose() function.
    let display_prompts: Vec<String> = prompt_pairs
        .iter()
        .map(|(display, _)| display.clone())
        .collect();

    // Create header with project information.
    let header_with_instructions = {
        let last_line = cli_text_line![cli_text(
            ui_str::select_prompt_header_msg_raw(&project_path, display_prompts.len()),
            crate::common::ui_templates::header_style_default()
        )];
        prefix_single_select_instruction_header(inline_vec![last_line])
    };

    // Show selection interface.
    let mut default_io_devices = DefaultIoDevices::default();
    let maybe_user_choice = choose(
        header_with_instructions,
        display_prompts.clone(),
        Some(height(7)), // Limit to 7 as specified
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await? // Propagate UI errors (e.g., I/O errors).
    .into_iter() // Convert to iterator.
    .next(); // Get the first (and only) selected item, if any.

    // Handle user selection.
    match maybe_user_choice {
        Some(selected_display_prompt) => Ok(handle_selected_prompt(
            selected_display_prompt,
            &prompt_pairs,
            &history,
            &project_path,
        )),
        None => {
            // User cancelled selection.
            Ok(ChResult::SelectionCancelled {
                project_path,
                total_prompts: history.len(),
            })
        }
    }
}

fn handle_selected_prompt(
    selected_display_prompt: InlineString,
    prompt_pairs: &[(String, String)],
    history: &[HistoryItem],
    project_path: &str,
) -> ChResult {
    // Find the original prompt from the display prompt.
    let selected_as_string = selected_display_prompt.to_string();
    let selected_index = prompt_pairs
        .iter()
        .position(|(display, _)| display == &selected_as_string)
        .unwrap_or(0);

    let original_prompt = prompt_pairs
        .iter()
        .find(|(display, _)| display == &selected_as_string)
        .map(|(_, original)| original.clone())
        .unwrap_or(selected_as_string.clone());

    // Process images and text content.
    let (success_message, text_content_to_clipboard) =
        process_history_item(history.get(selected_index), &original_prompt, project_path);

    // Copy text content to clipboard.
    let mut clipboard = SystemClipboard;
    if let Err(e) =
        clipboard.try_to_put_content_into_clipboard(text_content_to_clipboard.clone())
    {
        tracing::warn!("Failed to copy to clipboard: {}", e);
    }

    // Return success result.
    ChResult::PromptSelected {
        prompt: original_prompt,
        project_path: project_path.to_string(),
        total_prompts: history.len(),
        success_message: success_message.to_string(),
    }
}

fn process_history_item(
    history_item: Option<&HistoryItem>,
    original_prompt: &str,
    project_path: &str,
) -> (InlineString, String) {
    let mut text_content_to_clipboard = original_prompt.to_string();

    let success_message = if let Some(item) = history_item {
        match image_handler::parse_pasted_contents(item.pasted_contents.clone()) {
            Ok(parsed) => {
                // Use text content if available.
                if !parsed.text_content.is_empty() {
                    text_content_to_clipboard =
                        format!("{}\n{}", original_prompt, parsed.text_content);
                }

                // Handle images if present.
                if parsed.images.is_empty() {
                    ui_str::prompt_copied_msg(original_prompt)
                } else {
                    handle_images(&parsed.images, original_prompt, project_path)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse pasted contents: {}", e);
                ui_str::prompt_copied_msg(original_prompt)
            }
        }
    } else {
        ui_str::prompt_copied_msg(original_prompt)
    };

    (success_message, text_content_to_clipboard)
}

fn handle_images(
    images: &[ImageContent],
    original_prompt: &str,
    project_path: &str,
) -> InlineString {
    match image_handler::save_images_to_downloads(project_path, images) {
        Ok(saved_images) => ui_str::prompt_with_images_copied_msg(
            original_prompt,
            saved_images.len(),
            &saved_images,
        ),
        Err(e) => {
            tracing::warn!("Failed to save images: {}", e);
            ui_str::prompt_copied_with_image_error_msg(original_prompt)
        }
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{CommandRunResult, CommonResult, DefaultIoDevices, TTYResult, ast,
               ast_line, choose, height, inline_vec, is_fully_uninteractive_terminal,
               readline_async::{HowToChoose, StyleSheet},
               tui::editor::editor_buffer::{clipboard_service::SystemClipboard,
                                            clipboard_support::ClipboardService}};

use super::{CLIArg, image_handler, prompt_history,
            types::{ChDetails, CommandRunDetails},
            ui_str};
use crate::prefix_single_select_instruction_header;

/// Handle the main ch command logic
pub async fn handle_ch_command(
    _cli_arg: CLIArg,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    // Check if terminal is interactive
    if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
        return Ok(CommandRunResult::Noop(
            ui_str::terminal_not_interactive_msg(),
            CommandRunDetails::Ch(ChDetails {
                selected_prompt: None,
                project_path: "unknown".to_string(),
                total_prompts: 0,
            }),
        ));
    }

    // Get prompt history for current project
    let (project_path, history) = prompt_history::get_prompts_for_current_project()?;

    // Check if we have any prompts
    if history.is_empty() {
        return Ok(CommandRunResult::Noop(
            ui_str::no_prompts_found_msg(&project_path),
            CommandRunDetails::Ch(ChDetails {
                selected_prompt: None,
                project_path,
                total_prompts: 0,
            }),
        ));
    }

    // Prepare display/original pairs for ALL history items
    let prompt_pairs: Vec<(String, String)> = history
        .iter()
        .map(|item| {
            let original = item.display.clone(); // Keep original with newlines intact
            let mut display = original.replace(['\n', '\r'], " "); // Strip newlines only for display

            // Add image indicator prefix to display only
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

    // Extract display texts for the choose() function
    let display_prompts: Vec<String> = prompt_pairs
        .iter()
        .map(|(display, _)| display.clone())
        .collect();

    // Create header with project information
    let header_with_instructions = {
        let last_line = ast_line![ast(
            ui_str::select_prompt_header_msg_raw(&project_path, display_prompts.len()),
            crate::common::ui_templates::header_style_default()
        )];
        prefix_single_select_instruction_header(inline_vec![last_line])
    };

    // Show selection interface
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

    // Handle user selection
    match maybe_user_choice {
        Some(selected_display_prompt) => {
            // Find the original prompt from the display prompt (convert SmallString to
            // String)
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

            // Get the corresponding history item for image processing
            let selected_history_item = history.get(selected_index);

            // Process images and text content
            let success_message;
            let mut text_content_to_clipboard = original_prompt.clone();

            if let Some(history_item) = selected_history_item {
                match image_handler::parse_pasted_contents(
                    history_item.pasted_contents.clone(),
                ) {
                    Ok(parsed) => {
                        // Handle images if present
                        if parsed.images.is_empty() {
                            success_message = ui_str::prompt_copied_msg(&original_prompt);
                        } else {
                            match image_handler::save_images_to_downloads(
                                &project_path,
                                &parsed.images,
                            ) {
                                Ok(saved_images) => {
                                    success_message =
                                        ui_str::prompt_with_images_copied_msg(
                                            &original_prompt,
                                            saved_images.len(),
                                            &saved_images,
                                        );
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to save images: {}", e);
                                    success_message =
                                        ui_str::prompt_copied_with_image_error_msg(
                                            &original_prompt,
                                        );
                                }
                            }
                        }

                        // Use text content if available, otherwise use display text
                        if !parsed.text_content.is_empty() {
                            text_content_to_clipboard =
                                format!("{}\n{}", original_prompt, parsed.text_content);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse pasted contents: {}", e);
                        success_message = ui_str::prompt_copied_msg(&original_prompt);
                    }
                }
            } else {
                success_message = ui_str::prompt_copied_msg(&original_prompt);
            }

            // Copy text content to clipboard
            let mut clipboard = SystemClipboard;
            if let Err(e) = clipboard
                .try_to_put_content_into_clipboard(text_content_to_clipboard.clone())
            {
                tracing::warn!("Failed to copy to clipboard: {}", e);
            }

            // Return success result
            Ok(CommandRunResult::Noop(
                success_message,
                CommandRunDetails::Ch(ChDetails {
                    selected_prompt: Some(original_prompt),
                    project_path,
                    total_prompts: history.len(),
                }),
            ))
        }
        None => {
            // User cancelled selection
            Ok(CommandRunResult::Noop(
                ui_str::selection_cancelled_msg(),
                CommandRunDetails::Ch(ChDetails {
                    selected_prompt: None,
                    project_path,
                    total_prompts: history.len(),
                }),
            ))
        }
    }
}

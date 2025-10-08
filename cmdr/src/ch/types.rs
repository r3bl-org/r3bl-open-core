// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use serde::Deserialize;
use std::{collections::HashMap,
          fmt::{Display, Formatter, Result}};

/// Root structure for deserializing Claude Code's ~/.claude.json file
#[derive(Debug, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default)]
    pub projects: HashMap<String, Project>,
}

/// Project-specific configuration containing history
#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub history: Vec<HistoryItem>,
}

/// Individual history item representing a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryItem {
    pub display: String,
    #[serde(rename = "pastedContents")]
    pub pasted_contents: serde_json::Value,
}

/// Represents different types of pasted content in a prompt
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum PastedContent {
    #[serde(rename = "image")]
    Image {
        id: u32,
        content: String, // base64 encoded image data
        #[serde(rename = "mediaType")]
        media_type: String, // e.g., "image/png", "image/jpeg"
    },
    #[serde(rename = "text")]
    Text { id: u32, content: String },
}

/// Parsed pasted contents with separated images and text
#[derive(Debug, Clone)]
pub struct ParsedPastedContents {
    pub images: Vec<ImageContent>,
    pub text_content: String,
}

/// Image content extracted from pasted contents
#[derive(Debug, Clone)]
pub struct ImageContent {
    pub content: String, // base64 encoded
    pub media_type: String,
}

/// Result of saving images to filesystem
#[derive(Debug, Clone)]
pub struct SavedImageInfo {
    pub filename: String,
    pub filepath: std::path::PathBuf,
    pub media_type: String,
}

/// Result type specific to ch command operations
#[derive(Debug)]
pub enum ChResult {
    /// User successfully selected a prompt
    PromptSelected {
        prompt: String,
        project_path: String,
        total_prompts: usize,
        success_message: String,
    },
    /// User cancelled the selection (ESC or Ctrl+C)
    SelectionCancelled {
        project_path: String,
        total_prompts: usize,
    },
    /// No prompts found for the current project
    NoPromptsFound { project_path: String },
    /// Terminal is not interactive
    TerminalNotInteractive,
}

impl Display for ChResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use crate::ch::ui_str;

        match self {
            ChResult::PromptSelected {
                success_message, ..
            } => {
                write!(f, "{success_message}")
            }
            ChResult::SelectionCancelled { .. } => {
                write!(f, "{}", ui_str::selection_cancelled_msg())
            }
            ChResult::NoPromptsFound { project_path } => {
                write!(f, "{}", ui_str::no_prompts_found_msg(project_path))
            }
            ChResult::TerminalNotInteractive => {
                write!(f, "{}", ui_str::terminal_not_interactive_msg())
            }
        }
    }
}

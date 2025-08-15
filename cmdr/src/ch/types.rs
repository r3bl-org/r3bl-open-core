// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::collections::HashMap;

use serde::Deserialize;

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

/// Command run details for analytics and reporting
#[derive(Debug)]
pub struct ChDetails {
    pub selected_prompt: Option<String>,
    pub project_path: String,
    pub total_prompts: usize,
}

/// Command run details enum for the ch binary
#[derive(Debug)]
pub enum CommandRunDetails {
    Ch(ChDetails),
}

impl std::fmt::Display for CommandRunDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandRunDetails::Ch(details) => {
                if let Some(ref prompt) = details.selected_prompt {
                    write!(f, "Selected prompt: {prompt}")
                } else {
                    write!(f, "No prompt selected")
                }
            }
        }
    }
}

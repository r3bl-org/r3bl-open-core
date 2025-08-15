// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::path::PathBuf;

use base64::{Engine, engine::general_purpose};
use miette::IntoDiagnostic;
use r3bl_tui::{CommonResult, friendly_random_id};
use serde_json::Value;

use super::types::{HistoryItem, ImageContent, ParsedPastedContents, PastedContent,
                   SavedImageInfo};

/// Count the number of images in a history item's pasted contents
#[must_use]
pub fn count_images_in_history_item(history_item: &HistoryItem) -> usize {
    if let Value::Object(contents) = &history_item.pasted_contents {
        contents
            .values()
            .filter_map(|value| {
                serde_json::from_value::<PastedContent>(value.clone()).ok()
            })
            .filter(|content| matches!(content, PastedContent::Image { .. }))
            .count()
    } else {
        0
    }
}

/// Parse pasted contents to extract images and text
///
/// # Errors
///
/// Returns an error if the JSON deserialization fails for any content entry
pub fn parse_pasted_contents(
    pasted_contents: Value,
) -> CommonResult<ParsedPastedContents> {
    let mut images = Vec::new();
    let mut text_parts = Vec::new();

    if let Value::Object(contents) = pasted_contents {
        for (_key, value) in contents {
            match serde_json::from_value::<PastedContent>(value) {
                Ok(PastedContent::Image {
                    content,
                    media_type,
                    ..
                }) => {
                    images.push(ImageContent {
                        content,
                        media_type,
                    });
                }
                Ok(PastedContent::Text { content, .. }) => {
                    text_parts.push(content);
                }
                Err(_) => {
                    // Skip malformed entries
                    tracing::warn!("Failed to parse pasted content entry, skipping");
                }
            }
        }
    }

    let text_content = text_parts.join("\n");

    Ok(ParsedPastedContents {
        images,
        text_content,
    })
}

/// Generate a unique filename for an image
#[must_use]
pub fn generate_image_filename(
    project_path: &str,
    image_num: usize,
    friendly_id: &str,
    media_type: &str,
) -> String {
    let project_name = extract_project_name(project_path);
    let extension = media_type_to_extension(media_type);

    format!("{project_name}_image_{image_num}_{friendly_id}.{extension}")
}

/// Extract project name from full project path
#[must_use]
pub fn extract_project_name(project_path: &str) -> String {
    std::path::Path::new(project_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-project")
        .to_string()
}

/// Convert media type to file extension
#[must_use]
pub fn media_type_to_extension(media_type: &str) -> &str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "bin", // fallback for unknown types
    }
}

/// Decode base64 image data
///
/// # Errors
///
/// Returns an error if the base64 decoding fails
pub fn decode_base64_image(base64_data: &str) -> CommonResult<Vec<u8>> {
    general_purpose::STANDARD
        .decode(base64_data)
        .into_diagnostic()
        .map_err(|e| miette::miette!("Failed to decode base64 image data: {}", e))
}

/// Get the Downloads directory for the current platform
///
/// # Errors
///
/// Returns an error if neither the Downloads directory nor the home directory can be
/// determined
pub fn get_downloads_directory() -> CommonResult<PathBuf> {
    if let Some(downloads_dir) = dirs::download_dir() {
        Ok(downloads_dir)
    } else {
        // Fallback to home directory if Downloads can't be found
        let home_dir = dirs::home_dir()
            .ok_or_else(|| miette::miette!("Could not determine home directory"))?;
        Ok(home_dir.join("Downloads"))
    }
}

/// Save images to the Downloads directory
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get the Downloads directory
/// - Failed to create the Downloads directory
/// - Failed to decode base64 image data
/// - Failed to write image file to disk
///
/// # Panics
///
/// Panics if the `saved_images` vector is empty when trying to access the last element
/// (this should never happen as we always push before accessing)
pub fn save_images_to_downloads(
    project_path: &str,
    images: &[ImageContent],
) -> CommonResult<Vec<SavedImageInfo>> {
    if images.is_empty() {
        return Ok(Vec::new());
    }

    let downloads_dir = get_downloads_directory()?;

    // Create Downloads directory if it doesn't exist
    if !downloads_dir.exists() {
        std::fs::create_dir_all(&downloads_dir)
            .into_diagnostic()
            .map_err(|e| {
                miette::miette!("Failed to create Downloads directory: {}", e)
            })?;
    }

    // Generate a single friendly ID for this batch of images
    let friendly_id = friendly_random_id::generate_friendly_random_id();
    let mut saved_images = Vec::new();

    for (index, image) in images.iter().enumerate() {
        let filename = generate_image_filename(
            project_path,
            index + 1, // Start from 1 for user-friendly numbering
            &friendly_id,
            &image.media_type,
        );

        let filepath = downloads_dir.join(&filename);

        // Decode and save the image
        let image_data = decode_base64_image(&image.content)?;

        std::fs::write(&filepath, image_data)
            .into_diagnostic()
            .map_err(|e| {
                miette::miette!(
                    "Failed to write image file {}: {}",
                    filepath.display(),
                    e
                )
            })?;

        saved_images.push(SavedImageInfo {
            filename,
            filepath,
            media_type: image.media_type.clone(),
        });

        tracing::info!(
            "Saved image to: {}",
            saved_images.last().unwrap().filepath.display()
        );
    }

    Ok(saved_images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_project_name() {
        assert_eq!(extract_project_name("/home/user/my-project"), "my-project");
        assert_eq!(
            extract_project_name("/home/user/projects/r3bl-open-core"),
            "r3bl-open-core"
        );
        assert_eq!(extract_project_name("/"), "unknown-project"); // fallback
        assert_eq!(extract_project_name(""), "unknown-project"); // fallback

        // Test Windows paths on Windows
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                extract_project_name("C:\\Users\\user\\my-project"),
                "my-project"
            );
            assert_eq!(extract_project_name("C:\\"), "unknown-project");
        }

        // Test Unix paths on Unix
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(extract_project_name("/home/user/my-project"), "my-project");
        }
    }

    #[test]
    fn test_media_type_to_extension() {
        assert_eq!(media_type_to_extension("image/png"), "png");
        assert_eq!(media_type_to_extension("image/jpeg"), "jpg");
        assert_eq!(media_type_to_extension("image/jpg"), "jpg");
        assert_eq!(media_type_to_extension("image/gif"), "gif");
        assert_eq!(media_type_to_extension("image/webp"), "webp");
        assert_eq!(media_type_to_extension("image/bmp"), "bmp");
        assert_eq!(media_type_to_extension("image/svg+xml"), "svg");
        assert_eq!(media_type_to_extension("unknown"), "bin"); // fallback
        assert_eq!(media_type_to_extension(""), "bin"); // fallback
    }

    #[test]
    fn test_generate_image_filename() {
        let filename = generate_image_filename(
            "/home/user/my-project",
            1,
            "buddy-apple-023",
            "image/png",
        );
        assert_eq!(filename, "my-project_image_1_buddy-apple-023.png");

        let filename = generate_image_filename(
            "/Users/nazmul/github/r3bl-open-core",
            2,
            "tiger-banana-456",
            "image/jpeg",
        );
        assert_eq!(filename, "r3bl-open-core_image_2_tiger-banana-456.jpg");
    }

    #[test]
    fn test_decode_valid_base64_image() {
        // Valid 1x1 PNG base64 data
        let valid_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let result = decode_base64_image(valid_png_base64);
        assert!(result.is_ok());

        let decoded_data = result.unwrap();
        assert!(!decoded_data.is_empty());
    }

    #[test]
    fn test_decode_invalid_base64() {
        let invalid_base64 = "not-valid-base64!";
        let result = decode_base64_image(invalid_base64);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pasted_contents_with_images() {
        use serde_json::json;

        let json_data = json!({
            "1": {
                "id": 1,
                "type": "image",
                "content": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
                "mediaType": "image/png"
            },
            "2": {
                "id": 2,
                "type": "text",
                "content": "Some text content"
            }
        });

        let parsed = parse_pasted_contents(json_data).unwrap();
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].media_type, "image/png");
        assert_eq!(parsed.text_content, "Some text content");
    }

    #[test]
    fn test_parse_pasted_contents_text_only() {
        use serde_json::json;

        let json_data = json!({});
        let parsed = parse_pasted_contents(json_data).unwrap();
        assert_eq!(parsed.images.len(), 0);
        assert_eq!(parsed.text_content, "");
    }

    #[test]
    fn test_parse_pasted_contents_multiple_images() {
        use serde_json::json;

        let json_data = json!({
            "1": {
                "id": 1,
                "type": "image",
                "content": "iVBORw0KGgo...",
                "mediaType": "image/png"
            },
            "2": {
                "id": 2,
                "type": "image",
                "content": "R0lGODlhAQAB...",
                "mediaType": "image/gif"
            },
            "3": {
                "id": 3,
                "type": "text",
                "content": "First text"
            },
            "4": {
                "id": 4,
                "type": "text",
                "content": "Second text"
            }
        });

        let parsed = parse_pasted_contents(json_data).unwrap();
        assert_eq!(parsed.images.len(), 2);
        assert_eq!(parsed.images[0].media_type, "image/png");
        assert_eq!(parsed.images[1].media_type, "image/gif");
        assert_eq!(parsed.text_content, "First text\nSecond text");
    }

    #[test]
    fn test_count_images_in_history_item() {
        use serde_json::json;

        use super::super::types::HistoryItem;

        // Test with no images
        let history_item = HistoryItem {
            display: "No images here".to_string(),
            pasted_contents: json!({}),
        };
        assert_eq!(count_images_in_history_item(&history_item), 0);

        // Test with single image
        let history_item = HistoryItem {
            display: "One image [Image #1]".to_string(),
            pasted_contents: json!({
                "1": {
                    "id": 1,
                    "type": "image",
                    "content": "base64data",
                    "mediaType": "image/png"
                }
            }),
        };
        assert_eq!(count_images_in_history_item(&history_item), 1);

        // Test with multiple images
        let history_item = HistoryItem {
            display: "Multiple images [Image #1] [Image #2]".to_string(),
            pasted_contents: json!({
                "1": {
                    "id": 1,
                    "type": "image",
                    "content": "base64data1",
                    "mediaType": "image/png"
                },
                "2": {
                    "id": 2,
                    "type": "image",
                    "content": "base64data2",
                    "mediaType": "image/jpeg"
                },
                "3": {
                    "id": 3,
                    "type": "text",
                    "content": "Some text"
                }
            }),
        };
        assert_eq!(count_images_in_history_item(&history_item), 2);
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{env, fs, path::PathBuf};

use miette::IntoDiagnostic;
use r3bl_tui::CommonResult;

use super::types::{ClaudeConfig, HistoryItem};

/// Get the path to the Claude configuration file based on the current platform
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined
pub fn get_claude_config_path() -> CommonResult<PathBuf> {
    if cfg!(target_os = "windows") {
        // Try APPDATA first, then home directory
        if let Ok(appdata) = env::var("APPDATA") {
            let appdata_path = PathBuf::from(appdata).join(".claude.json");
            if appdata_path.exists() {
                return Ok(appdata_path);
            }
        }

        // Fallback to home directory
        if let Some(home_dir) = dirs::home_dir() {
            let home_path = home_dir.join(".claude.json");
            if home_path.exists() {
                return Ok(home_path);
            }
        }

        // If neither exists, return the APPDATA path for error reporting
        if let Ok(appdata) = env::var("APPDATA") {
            Ok(PathBuf::from(appdata).join(".claude.json"))
        } else {
            Err(miette::miette!("Could not determine home directory"))
        }
    } else {
        // Linux/macOS: ~/.claude.json
        let home_path = dirs::home_dir()
            .ok_or_else(|| miette::miette!("Could not determine home directory"))?
            .join(".claude.json");
        Ok(home_path)
    }
}

/// Read and parse the Claude configuration file
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get the configuration file path
/// - Configuration file doesn't exist
/// - Failed to read the configuration file
/// - Failed to parse the JSON content
pub fn read_claude_config() -> CommonResult<ClaudeConfig> {
    // Check if we should use test data
    if let Ok(test_file) = std::env::var("CH_USE_TEST_DATA") {
        let test_contents = match test_file.as_str() {
            "empty" => include_str!("test_data/.claude_empty.json"),
            "no_projects" => include_str!("test_data/.claude_no_projects.json"),
            "empty_projects" => {
                include_str!("test_data/.claude_empty_projects.json")
            }
            "empty_history" => include_str!("test_data/.claude_empty_history.json"),
            "single_image" => include_str!("test_data/.claude_with_single_image.json"),
            "multiple_images" => {
                include_str!("test_data/.claude_with_multiple_images.json")
            }
            "mixed_content" => include_str!("test_data/.claude_mixed_content.json"),
            "malformed_image" => include_str!("test_data/.claude_malformed_image.json"),
            _ => include_str!("test_data/.claude.json"),
        };
        return serde_json::from_str::<ClaudeConfig>(test_contents)
            .into_diagnostic()
            .map_err(|e| {
                miette::miette!("Failed to parse test data ({}): {}", test_file, e)
            });
    }

    let config_path = get_claude_config_path()?;

    if !config_path.exists() {
        return Err(miette::miette!(
            "Claude configuration file not found at: {}",
            config_path.display()
        ));
    }

    let contents = fs::read_to_string(&config_path)
        .into_diagnostic()
        .map_err(|e| {
            miette::miette!("Failed to read {}: {}", config_path.display(), e)
        })?;

    serde_json::from_str::<ClaudeConfig>(&contents)
        .into_diagnostic()
        .map_err(|e| miette::miette!("Failed to parse {}: {}", config_path.display(), e))
}

/// Get the current working directory as a string
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined
pub fn get_current_project_path() -> CommonResult<String> {
    let current_dir = env::current_dir().into_diagnostic()?;
    Ok(current_dir.to_string_lossy().to_string())
}

/// Find the best matching project path by looking for parent directories.
///
/// This function searches for a project configuration that matches the current path by:
/// 1. First checking for an exact match with the current path
/// 2. Then walking up the directory tree, checking each parent directory
/// 3. Returning the first project path found in the configuration
///
/// # Examples
/// If projects contains `"/home/user/project"` and `current_path` is:
/// - `"/home/user/project/src/lib"` → returns `Some("/home/user/project")`
/// - `"/home/user/project/tests/"` → returns `Some("/home/user/project")`
/// - `"/home/user/other/src"` → returns `None`
///
/// # Arguments
/// * `config` - The Claude configuration containing project paths
/// * `current_path` - The current working directory path to find a project for
///
/// # Returns
/// * `Some(project_path)` - The matching project path from the configuration
/// * `None` - If no matching project is found in any parent directory
#[must_use]
pub fn find_matching_project_path(
    config: &ClaudeConfig,
    current_path: &str,
) -> Option<String> {
    // First try exact match
    if config.projects.contains_key(current_path) {
        return Some(current_path.to_string());
    }

    // Try parent directories
    let mut path = PathBuf::from(current_path);
    while let Some(parent) = path.parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if config.projects.contains_key(&parent_str) {
            return Some(parent_str);
        }
        path = parent.to_path_buf();
    }

    None
}

/// Get prompt history for the current project
///
/// # Errors
///
/// Returns an error if:
/// - Failed to read the Claude configuration
/// - Failed to get the current project path
///
/// # Panics
///
/// Panics if the project path returned by `find_matching_project_path` doesn't exist in
/// the configuration (this should never happen as the function ensures the path exists)
pub fn get_prompts_for_current_project() -> CommonResult<(String, Vec<HistoryItem>)> {
    let config = read_claude_config()?;
    let current_path = get_current_project_path()?;

    // Find the best matching project path
    let result = match find_matching_project_path(&config, &current_path) {
        Some(project_path) => {
            let project = config.projects.get(&project_path).unwrap(); // Safe because find_matching_project_path ensures it exists
            (project_path, project.history.clone())
        }
        None => (current_path, Vec::new()),
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use r3bl_tui::{MkdirOptions, try_create_temp_dir, try_mkdir};

    use super::*;
    use crate::ch::types::Project;

    /// Create a mock Claude configuration with project paths for testing
    fn create_test_config() -> ClaudeConfig {
        let mut projects = HashMap::new();

        // Add a few test projects
        projects.insert(
            "/home/user/projects/app1".to_string(),
            Project {
                history: vec![HistoryItem {
                    display: "test prompt 1".to_string(),
                    pasted_contents: serde_json::Value::Null,
                }],
            },
        );

        projects.insert(
            "/home/user/projects/app2".to_string(),
            Project {
                history: vec![HistoryItem {
                    display: "test prompt 2".to_string(),
                    pasted_contents: serde_json::Value::Null,
                }],
            },
        );

        projects.insert(
            "/tmp/workspace/demo".to_string(),
            Project {
                history: vec![HistoryItem {
                    display: "demo prompt".to_string(),
                    pasted_contents: serde_json::Value::Null,
                }],
            },
        );

        ClaudeConfig { projects }
    }

    #[test]
    fn test_find_matching_project_path_exact_match() {
        let config = create_test_config();

        // Test exact match
        let result = find_matching_project_path(&config, "/home/user/projects/app1");
        assert_eq!(result, Some("/home/user/projects/app1".to_string()));

        let result = find_matching_project_path(&config, "/tmp/workspace/demo");
        assert_eq!(result, Some("/tmp/workspace/demo".to_string()));
    }

    #[test]
    fn test_find_matching_project_path_parent_match() {
        let config = create_test_config();

        // Test subdirectory matching (should find parent)
        let result = find_matching_project_path(&config, "/home/user/projects/app1/src");
        assert_eq!(result, Some("/home/user/projects/app1".to_string()));

        let result = find_matching_project_path(
            &config,
            "/home/user/projects/app1/src/lib/components",
        );
        assert_eq!(result, Some("/home/user/projects/app1".to_string()));

        let result = find_matching_project_path(&config, "/tmp/workspace/demo/tests");
        assert_eq!(result, Some("/tmp/workspace/demo".to_string()));
    }

    #[test]
    fn test_find_matching_project_path_no_match() {
        let config = create_test_config();

        // Test no match scenarios
        let result = find_matching_project_path(&config, "/home/user/other_project");
        assert_eq!(result, None);

        let result = find_matching_project_path(&config, "/completely/different/path");
        assert_eq!(result, None);

        let result = find_matching_project_path(&config, "/home/user/projects"); // Parent of app1 but not in config
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_matching_project_path_empty_config() {
        let config = ClaudeConfig {
            projects: HashMap::new(),
        };

        let result = find_matching_project_path(&config, "/any/path");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_matching_project_path_root_directory() {
        let mut projects = HashMap::new();
        projects.insert("/".to_string(), Project { history: vec![] });
        let config = ClaudeConfig { projects };

        // Should find root for any path
        let result = find_matching_project_path(&config, "/home/user/any/deep/path");
        assert_eq!(result, Some("/".to_string()));
    }

    #[test]
    fn test_find_matching_project_path_with_real_filesystem() -> miette::Result<()> {
        // Create a temporary directory structure for realistic testing
        let temp_dir = try_create_temp_dir()?;

        // Create a realistic project structure
        let project_root = temp_dir.join("my_project");
        let src_dir = project_root.join("src");
        let tests_dir = project_root.join("tests");
        let lib_dir = src_dir.join("lib");

        try_mkdir(&project_root, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&src_dir, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&tests_dir, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&lib_dir, MkdirOptions::CreateIntermediateDirectories)?;

        // Create config with the project root
        let mut projects = HashMap::new();
        projects.insert(
            project_root.to_string_lossy().to_string(),
            Project { history: vec![] },
        );
        let config = ClaudeConfig { projects };

        // Test exact match
        let result = find_matching_project_path(&config, &project_root.to_string_lossy());
        assert_eq!(result, Some(project_root.to_string_lossy().to_string()));

        // Test subdirectory matches
        let result = find_matching_project_path(&config, &src_dir.to_string_lossy());
        assert_eq!(result, Some(project_root.to_string_lossy().to_string()));

        let result = find_matching_project_path(&config, &lib_dir.to_string_lossy());
        assert_eq!(result, Some(project_root.to_string_lossy().to_string()));

        let result = find_matching_project_path(&config, &tests_dir.to_string_lossy());
        assert_eq!(result, Some(project_root.to_string_lossy().to_string()));

        // Test sibling directory (should not match)
        let sibling_dir = temp_dir.join("other_project");
        try_mkdir(&sibling_dir, MkdirOptions::CreateIntermediateDirectories)?;

        let result = find_matching_project_path(&config, &sibling_dir.to_string_lossy());
        assert_eq!(result, None);

        Ok(())
    }

    #[test]
    fn test_find_matching_project_path_multiple_nested_projects() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;

        // Create nested project structure
        let workspace = temp_dir.join("workspace");
        let outer_project = workspace.join("outer_project");
        let inner_project = outer_project.join("nested_project");
        let src_dir = inner_project.join("src");

        try_mkdir(&workspace, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&outer_project, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&inner_project, MkdirOptions::CreateIntermediateDirectories)?;
        try_mkdir(&src_dir, MkdirOptions::CreateIntermediateDirectories)?;

        // Create config with both projects
        let mut projects = HashMap::new();
        projects.insert(
            outer_project.to_string_lossy().to_string(),
            Project { history: vec![] },
        );
        projects.insert(
            inner_project.to_string_lossy().to_string(),
            Project { history: vec![] },
        );
        let config = ClaudeConfig { projects };

        // Should find the closest (most specific) match
        let result = find_matching_project_path(&config, &src_dir.to_string_lossy());
        assert_eq!(result, Some(inner_project.to_string_lossy().to_string()));

        // Should find exact match for inner project
        let result =
            find_matching_project_path(&config, &inner_project.to_string_lossy());
        assert_eq!(result, Some(inner_project.to_string_lossy().to_string()));

        // Should find exact match for outer project
        let result =
            find_matching_project_path(&config, &outer_project.to_string_lossy());
        assert_eq!(result, Some(outer_project.to_string_lossy().to_string()));

        Ok(())
    }

    #[test]
    fn test_find_matching_project_path_edge_cases() {
        let config = create_test_config();

        // Test empty path
        let result = find_matching_project_path(&config, "");
        assert_eq!(result, None);

        // Test single character path
        let result = find_matching_project_path(&config, "/");
        assert_eq!(result, None);

        // Test relative path (shouldn't match absolute project paths)
        let result = find_matching_project_path(&config, "src/lib");
        assert_eq!(result, None);
    }
}

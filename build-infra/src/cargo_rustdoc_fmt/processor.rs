// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Orchestrate rustdoc formatting for files.
//!
//! Files containing `// rustdoc-fmt: skip` are skipped entirely. Place this comment
//! anywhere in the file (typically after the copyright line). This is useful for files
//! whose doc comments discuss the formatter's own concepts (e.g., technical terms like
//! CSI that would be incorrectly linkified by the term linker).

use crate::cargo_rustdoc_fmt::{constants::RUSTDOC_FMT_SKIP_DIRECTIVE,
                               cst::{FileChunk, SourceFileCst},
                               formatters::{TechnicalTermDictionary,
                                            convert_links_in_doc_block,
                                            format_table_node, link_terms_in_doc_block,
                                            normalize_code_fence_node},
                               types::{ExecutionMode, FormatOptions, FormatStatus,
                                       PassPolicy}};
use r3bl_tui::{RangeBoundsExt, RangeOverlapResult};
use std::path::{Path, PathBuf};

/// Processes Rust files to format their rustdoc comments.
#[derive(Debug)]
pub struct FileProcessor<'a> {
    options: FormatOptions,
    maybe_term_dict: Option<&'a TechnicalTermDictionary>,
}

impl<'a> FileProcessor<'a> {
    /// Creates a new file processor with the given formatting options and optional
    /// technical term dictionary.
    #[must_use]
    pub fn new(
        options: FormatOptions,
        maybe_term_dict: Option<&'a TechnicalTermDictionary>,
    ) -> Self {
        Self {
            options,
            maybe_term_dict,
        }
    }

    /// Formats the given Rust `source` string in memory using the lossless
    /// [`SourceFileCst`] pipeline.
    ///
    /// Returns a tuple of:
    /// - [`FormatStatus::Modified`] and the reconstructed string if changes were made.
    /// - [`FormatStatus::Unchanged`] and `source.to_owned()` if no formatting was needed
    ///   or the file was skipped via `// rustdoc-fmt: skip`.
    #[must_use]
    pub fn process_source(&self, source: &str) -> (FormatStatus, String) {
        // Skip entire file if it contains `// rustdoc-fmt: skip`, UNLESS
        // line_range is specified (e.g. `--lines-force`). Explicit line range
        // overrides file-level skip directive for our binary.
        let range = self.options.line_range.as_ref();
        if range.is_none() && has_rustdoc_fmt_skip(source) {
            return (FormatStatus::Unchanged, source.to_owned());
        }

        let mut cst = SourceFileCst::parse(source);
        let mut modified = false;

        for chunk in &mut cst.chunks {
            if let FileChunk::DocBlock(block) = chunk {
                if self.options.format_tables == PassPolicy::Enabled {
                    for node in &mut block.nodes {
                        if range.is_none_or(|target_range| {
                            node.span().overlaps(target_range)
                                == RangeOverlapResult::Overlaps
                        }) && format_table_node(node) == FormatStatus::Modified
                        {
                            modified = true;
                        }
                    }
                }
                if self.options.convert_links == PassPolicy::Enabled
                    && convert_links_in_doc_block(block, range) == FormatStatus::Modified
                {
                    modified = true;
                }
                if self.options.link_terms == PassPolicy::Enabled
                    && let Some(dict) = self.maybe_term_dict
                    && link_terms_in_doc_block(block, dict, range)
                        == FormatStatus::Modified
                {
                    modified = true;
                }
                if self.options.normalize_fences == PassPolicy::Enabled {
                    for node in &mut block.nodes {
                        if range.is_none_or(|target_range| {
                            node.span().overlaps(target_range)
                                == RangeOverlapResult::Overlaps
                        }) && normalize_code_fence_node(node) == FormatStatus::Modified
                        {
                            modified = true;
                        }
                    }
                }
            }
        }

        if modified {
            let new_source = cst.reconstruct();
            if new_source != source {
                return (FormatStatus::Modified, new_source);
            }
        }

        (FormatStatus::Unchanged, source.to_owned())
    }

    /// Processes multiple files using the lossless [`SourceFileCst`] pipeline.
    #[must_use]
    pub fn process_files(&self, paths: &[PathBuf]) -> Vec<ProcessingResult> {
        paths.iter().map(|path| self.process_file(path)).collect()
    }

    /// Processes a single file using the lossless [`SourceFileCst`] pipeline.
    fn process_file(&self, path: &Path) -> ProcessingResult {
        let mut result = ProcessingResult::new(path.to_path_buf());

        // Read file.
        let source = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                result.add_error(format!("Failed to read file: {err}"));
                return result;
            }
        };

        let (status, new_source) = self.process_source(&source);

        if status == FormatStatus::Modified {
            if self.options.mode == ExecutionMode::CheckOnly {
                result.mark_modified();
            } else if let Err(err) = std::fs::write(path, new_source) {
                result.add_error(format!("Failed to write file: {err}"));
            } else {
                result.mark_modified();
            }
        }

        result
    }
}

/// Checks if a file contains the `// rustdoc-fmt: skip` marker.
///
/// When present, the entire file is skipped by `cargo rustdoc-fmt`. Place this
/// comment anywhere in the file (typically near the top, after the copyright line).
fn has_rustdoc_fmt_skip(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim() == RUSTDOC_FMT_SKIP_DIRECTIVE)
}

/// Result of processing a single file.
#[derive(Debug)]
pub struct ProcessingResult {
    /// Path to the processed file.
    pub file_path: PathBuf,

    /// Whether the file was modified by the formatting pipeline.
    pub status: FormatStatus,

    /// Any errors encountered.
    pub errors: Vec<String>,
}

impl ProcessingResult {
    fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            status: FormatStatus::Unchanged,
            errors: Vec::new(),
        }
    }

    fn mark_modified(&mut self) { self.status = FormatStatus::Modified; }

    fn add_error(&mut self, error: String) { self.errors.push(error); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::types::{EditorLineNum, Verbosity, editor_line_num};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_process_nonexistent_file() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options, None);
        let result = processor.process_file(Path::new("/nonexistent/file.rs"));
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_process_source_in_memory() {
        let options = FormatOptions {
            format_tables: PassPolicy::Skipped,
            convert_links: PassPolicy::Enabled,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: None,
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };
        let processor = FileProcessor::new(options, None);
        let input = "//! [example](https://example.com)\nfn main() {}\n";
        let (status, formatted) = processor.process_source(input);
        assert_eq!(status, FormatStatus::Modified);
        assert!(formatted.contains("//! [example]: https://example.com"));
    }

    #[test]
    fn test_process_source_skip_directive() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options, None);
        let input =
            "// rustdoc-fmt: skip\n\n//! [example](https://example.com)\nfn main() {}\n";
        let (status, formatted) = processor.process_source(input);
        assert_eq!(status, FormatStatus::Unchanged);
        assert_eq!(formatted, input);
    }

    #[test]
    fn test_has_rustdoc_fmt_skip() {
        assert!(has_rustdoc_fmt_skip(
            "// Copyright\n// rustdoc-fmt: skip\n\n//! Module docs.\n"
        ));
        assert!(has_rustdoc_fmt_skip("    // rustdoc-fmt: skip\n"));
        assert!(!has_rustdoc_fmt_skip("// Copyright\n\n//! Module docs.\n"));
        // Should only match plain comments, not doc comments.
        assert!(!has_rustdoc_fmt_skip("/// rustdoc-fmt: skip\n"));
    }

    #[test]
    fn test_file_processor_check_mode_does_not_write() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        let content = "//! Test with link [docs](https://example.com)\n\nfn main() {}\n";
        fs::write(&test_file, content).unwrap();

        let options = FormatOptions {
            format_tables: PassPolicy::Skipped,
            convert_links: PassPolicy::Enabled,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: None,
            mode: ExecutionMode::CheckOnly,
            verbosity: Verbosity::Quiet,
        };

        let processor = FileProcessor::new(options, None);
        let result = processor.process_file(&test_file);

        assert!(result.errors.is_empty());
        assert_eq!(result.status, FormatStatus::Modified);
        let after_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, after_content);
    }

    #[test]
    fn test_lines_range_formatting_single_table() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_range.rs");

        let input = "fn header() {}\n/// | A | B |\n/// |---|---|\n/// | 1 | 2 |\nfn footer() {}\n";
        fs::write(&test_file, input).unwrap();

        let options = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Skipped,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: Some(EditorLineNum::to_c_row_range(
                editor_line_num(2),
                editor_line_num(4),
            )),
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };

        let proc = FileProcessor::new(options, None);
        let result = proc.process_file(&test_file);
        assert_eq!(result.status, FormatStatus::Modified);

        let output = fs::read_to_string(&test_file).unwrap();
        assert!(output.contains("fn header() {}\n"));
        assert!(output.contains("/// | A   | B   |\n"));
        assert!(output.contains("fn footer() {}\n"));
    }

    #[test]
    fn test_lines_range_formatting_selective_multi_table() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_multi.rs");

        let input = "/// | T1 | Col |\n/// |---|---|\n/// | 1 | 2 |\n///\n/// | T2 | Col |\n/// |---|---|\n/// | 3 | 4 |\nfn main() {}\n";
        fs::write(&test_file, input).unwrap();

        // Target Table 2 (1-indexed lines 5..=7).
        let options = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Skipped,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: Some(EditorLineNum::to_c_row_range(
                editor_line_num(5),
                editor_line_num(7),
            )),
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };

        let proc = FileProcessor::new(options, None);
        let result = proc.process_file(&test_file);
        assert_eq!(result.status, FormatStatus::Modified);

        let output = fs::read_to_string(&test_file).unwrap();
        // Table 1 must remain 100% byte-for-byte untouched.
        assert!(output.contains("/// | T1 | Col |\n/// |---|---|\n/// | 1 | 2 |\n"));
        // Table 2 must be formatted.
        assert!(
            output.contains("/// | T2  | Col |\n/// | --- | --- |\n/// | 3   | 4   |\n")
        );
    }

    #[test]
    fn test_lines_range_formatting_non_overlapping_unmodified() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_non_overlap.rs");

        let input = "fn line_1() {}\n/// | A | B |\n/// |---|---|\n/// | 1 | 2 |\nfn line_5() {}\n";
        fs::write(&test_file, input).unwrap();

        // Target line 1 (pure code).
        let options = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Skipped,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: Some(EditorLineNum::to_c_row_range(
                editor_line_num(1),
                editor_line_num(1),
            )),
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };

        let proc = FileProcessor::new(options, None);
        let result = proc.process_file(&test_file);
        assert_eq!(result.status, FormatStatus::Unchanged);

        let output = fs::read_to_string(&test_file).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_lines_force_bypasses_rustdoc_fmt_skip() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_skip_bypass.rs");

        let input = "// rustdoc-fmt: skip\n\n/// | A | B |\n/// |---|---|\n/// | 1 | 2 |\nfn main() {}\n";
        fs::write(&test_file, input).unwrap();

        // 1. Without line_range, file is skipped entirely.
        let options_normal = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Skipped,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: None,
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };
        let proc = FileProcessor::new(options_normal, None);
        let res1 = proc.process_file(&test_file);
        assert_eq!(res1.status, FormatStatus::Unchanged);
        assert_eq!(fs::read_to_string(&test_file).unwrap(), input);

        // 2. With line_range, // rustdoc-fmt: skip is bypassed.
        let options_force = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Skipped,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: Some(EditorLineNum::to_c_row_range(
                editor_line_num(3),
                editor_line_num(5),
            )),
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };
        let proc_force = FileProcessor::new(options_force, None);
        let res2 = proc_force.process_file(&test_file);
        assert_eq!(res2.status, FormatStatus::Modified);

        let output = fs::read_to_string(&test_file).unwrap();
        assert!(output.contains("// rustdoc-fmt: skip\n"));
        assert!(
            output.contains("/// | A   | B   |\n/// | --- | --- |\n/// | 1   | 2   |\n")
        );
    }

    #[test]
    fn test_lines_force_converts_inline_links_and_aggregates() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_links_range.rs");

        let input = "/// Line 1 [first](https://example.com/first)\n///\n/// Line 3 [second](https://example.com/second)\nfn main() {}\n";
        fs::write(&test_file, input).unwrap();

        // Target only Line 1 (line 1..=1).
        let options = FormatOptions {
            format_tables: PassPolicy::Enabled,
            convert_links: PassPolicy::Enabled,
            link_terms: PassPolicy::Skipped,
            normalize_fences: PassPolicy::Skipped,
            line_range: Some(EditorLineNum::to_c_row_range(
                editor_line_num(1),
                editor_line_num(1),
            )),
            mode: ExecutionMode::ApplyChanges,
            verbosity: Verbosity::Quiet,
        };

        let proc = FileProcessor::new(options, None);
        let res = proc.process_file(&test_file);
        assert_eq!(res.status, FormatStatus::Modified);

        let output = fs::read_to_string(&test_file).unwrap();
        // Line 1 link converted to reference style.
        assert!(output.contains("/// Line 1 [first]\n"));
        // Line 3 link preserved as inline.
        assert!(output.contains("/// Line 3 [second](https://example.com/second)\n"));
        // Reference aggregated at bottom of doc block.
        assert!(output.contains("/// [first]: https://example.com/first\n"));
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use clap::Parser;
use r3bl_build_infra::{cargo_rustdoc_fmt::{CLIArg, FileProcessor},
                       common::{cargo_fmt_runner, git_utils, workspace_utils}};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:?}");
        process::exit(1);
    }
}

fn run() -> miette::Result<()> {
    let cli_arg = CLIArg::parse();
    let options = cli_arg.to_format_options();

    // Get files to process
    let files = if !cli_arg.paths.is_empty() {
        // Specific paths provided - highest priority
        if cli_arg.verbose {
            println!("Formatting specific paths...");
        }
        workspace_utils::find_rust_files_in_paths(&cli_arg.paths)?
    } else if cli_arg.workspace {
        // --workspace flag: format entire workspace
        let workspace_root = workspace_utils::get_workspace_root()?;
        if cli_arg.verbose {
            println!("Formatting entire workspace...");
        }
        workspace_utils::find_rust_files(&workspace_root)?
    } else {
        // Default: use git to find changed files
        if git_utils::is_git_repo() {
            let git_files = git_utils::get_changed_rust_files()?;
            if git_files.is_empty() {
                // No git changes, format entire workspace as fallback
                let workspace_root = workspace_utils::get_workspace_root()?;
                if cli_arg.verbose {
                    println!("No git changes found. Formatting entire workspace...");
                }
                workspace_utils::find_rust_files(&workspace_root)?
            } else {
                if cli_arg.verbose {
                    println!("Formatting {} files from git changes...", git_files.len());
                }
                git_files
            }
        } else {
            // Not a git repo, format entire workspace as fallback
            let workspace_root = workspace_utils::get_workspace_root()?;
            if cli_arg.verbose {
                println!("Not a git repository. Formatting entire workspace...");
            }
            workspace_utils::find_rust_files(&workspace_root)?
        }
    };

    if files.is_empty() {
        println!("No Rust files found to format.");
        return Ok(());
    }

    if cli_arg.verbose {
        println!("Processing {} files...", files.len());
    }

    // Process files
    let processor = FileProcessor::new(options);
    let results = processor.process_files(&files);

    // Report results
    let mut total_modified = 0;
    let mut total_errors = 0;

    for result in &results {
        if result.modified {
            total_modified += 1;
            if cli_arg.verbose || cli_arg.check {
                println!("Modified: {}", result.file_path.display());
            }
        }
        if !result.errors.is_empty() {
            total_errors += result.errors.len();
            eprintln!("Errors in {}:", result.file_path.display());
            for error in &result.errors {
                eprintln!("  - {error}");
            }
        }
    }

    println!(
        "\nProcessed {} files, {} modified, {} errors",
        results.len(),
        total_modified,
        total_errors
    );

    // Run cargo fmt on successfully modified files (unless skipped or in check mode)
    if !cli_arg.skip_cargo_fmt && total_modified > 0 && !cli_arg.check {
        let modified_files: Vec<_> = results
            .iter()
            .filter(|r| r.modified && r.errors.is_empty())
            .map(|r| r.file_path.clone())
            .collect();

        if !modified_files.is_empty() {
            if cli_arg.verbose {
                println!(
                    "\nRunning cargo fmt on {} modified files...",
                    modified_files.len()
                );
            }
            cargo_fmt_runner::run_cargo_fmt_on_files(&modified_files, cli_arg.verbose)?;
        }
    }

    if cli_arg.check && total_modified > 0 {
        eprintln!("\nSome files need formatting. Run without --check to format them.");
        process::exit(1);
    }

    if total_errors > 0 {
        process::exit(1);
    }

    Ok(())
}

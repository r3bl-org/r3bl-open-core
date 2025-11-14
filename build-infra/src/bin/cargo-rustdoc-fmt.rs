// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use clap::Parser;
use r3bl_build_infra::{cargo_rustdoc_fmt::{CLIArg, FileProcessor},
                       common::{cargo_fmt_runner, workspace_utils}};
use r3bl_tui::core::script::{try_get_changed_files_by_ext, try_is_git_repo};
use std::process;

#[tokio::main]
async fn main() {
    match run().await {
        Err(e) => {
            eprintln!("Error: {e:?}");
            process::exit(1);
        }
        Ok(_) => (),
    }
}

/// Prepares command-line arguments, accounting for Cargo's plugin invocation convention.
///
/// Here are the two ways in which this program can be invoked from the command line:
///
/// | Method    | Command                                   | `args[0]`              | `args[1]`       | `args[2..]`                     |
/// | --------- | ----------------------------------------- | ---------------------- | --------------- | ------------------------------- |
/// | Direct    | `cargo-rustdoc-fmt --verbose --workspace` | `"cargo-rustdoc-fmt"`  | `"--verbose"`   | `["--workspace"]`               |
/// | Via Cargo | `cargo rustdoc-fmt --verbose --workspace` | `"cargo-rustdoc-fmt"`  | `"rustdoc-fmt"` | `["--verbose", "--workspace"]`* |
///
/// When run via Cargo, notice the extra `rustdoc-fmt` string at `args[1]`!
/// Cargo automatically injects the "subcommand name" there.
///
/// # Cargo Plugin Convention
///
/// When a Cargo plugin is invoked via the `cargo` binary, Cargo injects the the
/// "subcommand name" as the first argument after the program name (which is `cargo`).
/// This is a Cargo convention to help plugins identify how they were invoked.
///
/// ## Why We Skip `args[1]` When Invoked Via Cargo
///
/// `clap` doesn't know about Cargo's plugin convention, so it would interpret
/// "rustdoc-fmt" as a command line argument passed in by the user. We manually remove it
/// when detected so `clap` only sees the actual user-provided arguments.
fn strip_cargo_subcommand_injection() -> Vec<String> {
    // Dynamically extract subcommand name from binary name, so we don't have to hardcode
    // the binary name here.
    // - "cargo-rustdoc-fmt" -> "rustdoc-fmt"
    // - `CARGO_BIN_NAME` is set by Cargo at compile time.
    let injected_subcommand_name = env!("CARGO_BIN_NAME")
        .strip_prefix("cargo-")
        .expect("This binary name must start with 'cargo-'");

    // Skip the injected subcommand name if invoked via Cargo
    let mut args: Vec<_> = std::env::args().collect();
    let is_cargo_invocation = args.len() >= 2 && args[1] == injected_subcommand_name;
    if is_cargo_invocation {
        args.remove(1);
    }
    args
}

async fn run() -> miette::Result<()> {
    let args = strip_cargo_subcommand_injection();

    // Parse args
    let cli_arg = CLIArg::parse_from(&args);
    let options = cli_arg.to_format_options();

    // Get files to process
    let files = if !cli_arg.paths.is_empty() {
        // Specific paths provided - highest priority
        if cli_arg.verbose {
            println!(
                "File discovery: Using {} specific path(s) provided as arguments",
                cli_arg.paths.len()
            );
        }
        workspace_utils::find_rust_files_in_paths(&cli_arg.paths)?
    } else if cli_arg.workspace {
        // --workspace flag: format entire workspace
        let workspace_root = workspace_utils::get_workspace_root()?;
        if cli_arg.verbose {
            println!("File discovery: Using --workspace flag (entire workspace)");
        }
        workspace_utils::find_rust_files(&workspace_root)?
    } else {
        // Default: use git to find changed files
        let (is_git_repo_result, _) = try_is_git_repo().await;
        let is_git_repo = is_git_repo_result.map_err(|e| miette::miette!("{:?}", e))?;
        if is_git_repo {
            let (git_files_result, _) = try_get_changed_files_by_ext(&["rs"]).await;
            let git_files = git_files_result.map_err(|e| miette::miette!("{:?}", e))?;
            if git_files.is_empty() {
                // No git changes, format entire workspace as fallback
                let workspace_root = workspace_utils::get_workspace_root()?;
                if cli_arg.verbose {
                    println!(
                        "File discovery: No git changes found, using entire workspace"
                    );
                }
                workspace_utils::find_rust_files(&workspace_root)?
            } else {
                if cli_arg.verbose {
                    println!(
                        "File discovery: Found {} changed file(s) from git",
                        git_files.len()
                    );
                }
                git_files
            }
        } else {
            // Not a git repo, format entire workspace as fallback
            let workspace_root = workspace_utils::get_workspace_root()?;
            if cli_arg.verbose {
                println!("File discovery: Not a git repository, using entire workspace");
            }
            workspace_utils::find_rust_files(&workspace_root)?
        }
    };

    if files.is_empty() {
        println!("No Rust files found to format.");
        return Ok(());
    }

    // Dry-run mode: show files and exit
    if cli_arg.dry_run {
        println!("Dry-run mode: {} files would be processed:", files.len());
        for file in &files {
            println!("  - {}", file.display());
        }
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

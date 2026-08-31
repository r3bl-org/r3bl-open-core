// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use clap::Parser;
use r3bl_tui::OutputFormat;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "env-source",
    about = "📜 Fast environment loader in native Rust 🚀",
    version
)]
pub struct CLIArg {
    /// Target shell script file to source (e.g. ~/.profile or setenv.bat).
    #[arg(short = 'i', long = "input-file", value_name = "SCRIPT_FILE")]
    pub input_file: PathBuf,

    /// Output format syntax.
    #[arg(short = 'o', long = "output-format", value_enum)]
    pub output_format: OutputFormat,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli_schema() { CLIArg::command().debug_assert(); }

    #[test]
    fn test_cli_arg_parsing() {
        let args = CLIArg::parse_from(["env-source", "-i", "test.sh", "-o", "fish"]);
        assert_eq!(args.input_file, PathBuf::from("test.sh"));
        assert_eq!(args.output_format, OutputFormat::Fish);
    }
}

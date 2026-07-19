// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Command-line argument parsing and configuration for the [`rust-analyzer`] MCP server.
//!
//! Provides the [`CLIArg`] structure and [`LogLevel`] enumeration for configuring the
//! server workspace, tracing output destinations, and verbosity filters.
//!
//! [`rust-analyzer`]: https://rust-analyzer.github.io/

use clap::{Parser, ValueEnum};
use r3bl_tui::log::{DisplayPreference, TracingConfig, WriterConfig};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;

/// Model Context Protocol (MCP) server for [`rust-analyzer`].
///
/// Exposes semantic code navigation, AST symbol search, hover information, diagnostics,
/// formatting, and code actions over JSON-RPC stdio transport.
///
/// [`rust-analyzer`]: https://rust-analyzer.github.io/
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
#[command(
    name = "rust-analyzer-mcp-server",
    version,
    about = "Model Context Protocol (MCP) server for rust-analyzer",
    long_about = None
)]
pub struct CLIArg {
    /// Workspace root directory path. Defaults to current directory if omitted.
    #[arg(value_name = "WORKSPACE_PATH", index = 1)]
    pub positional_workspace: Option<PathBuf>,

    /// Explicit workspace root directory path.
    #[arg(short = 'w', long = "workspace", value_name = "PATH")]
    pub workspace: Option<PathBuf>,

    /// Log level filter for stderr or log file.
    #[arg(
        short = 'l',
        long = "log-level",
        value_enum,
        default_value_t = LogLevel::Info,
        value_name = "LEVEL"
    )]
    pub log_level: LogLevel,

    /// Optional path prefix to write rolling log files to.
    #[arg(long = "log-file", value_name = "PATH")]
    pub log_file: Option<String>,
}

impl CLIArg {
    /// Resolves the effective workspace root path.
    ///
    /// Priority order:
    /// 1. Explicit `--workspace` / `-w` flag.
    /// 2. Positional `[WORKSPACE_PATH]` argument.
    /// 3. Current working directory fallback (`.`).
    #[must_use]
    pub fn resolve_workspace_path(&self) -> PathBuf {
        if let Some(ref path) = self.workspace {
            return path.clone();
        }
        if let Some(ref path) = self.positional_workspace {
            return path.clone();
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Converts CLI arguments into a [`TracingConfig`].
    #[must_use]
    pub fn to_tracing_config(&self) -> TracingConfig {
        let level_filter: LevelFilter = self.log_level.into();

        let writer_config = match (&self.log_file, self.log_level) {
            (_, LogLevel::Off) => WriterConfig::None,
            (Some(file_path), _) => {
                WriterConfig::DisplayAndFile(DisplayPreference::Stderr, file_path.clone())
            }
            (None, _) => WriterConfig::Display(DisplayPreference::Stderr),
        };

        TracingConfig {
            writer_config,
            level_filter,
        }
    }
}

/// Supported log levels for structured tracing output.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LogLevel {
    /// Disables all logging output.
    Off,
    /// Critical errors only.
    Error,
    /// Warnings and errors.
    Warn,
    /// Informational notices, warnings, and errors.
    #[default]
    Info,
    /// Verbose debug details, LSP frames, and tool call traces.
    Debug,
    /// Deep tracing of every event.
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Off => LevelFilter::OFF,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cli_args() {
        let cli_arg = CLIArg::parse_from(["rust-analyzer-mcp-server"]);
        assert_eq!(cli_arg.positional_workspace, None);
        assert_eq!(cli_arg.workspace, None);
        assert_eq!(cli_arg.log_level, LogLevel::Info);
        assert_eq!(cli_arg.log_file, None);

        let config = cli_arg.to_tracing_config();
        assert_eq!(config.level_filter, LevelFilter::INFO);
        assert_eq!(
            config.writer_config,
            WriterConfig::Display(DisplayPreference::Stderr)
        );
    }

    #[test]
    fn test_positional_workspace() {
        let cli_arg =
            CLIArg::parse_from(["rust-analyzer-mcp-server", "/path/to/my/project"]);
        assert_eq!(
            cli_arg.positional_workspace,
            Some(PathBuf::from("/path/to/my/project"))
        );
        assert_eq!(
            cli_arg.resolve_workspace_path(),
            PathBuf::from("/path/to/my/project")
        );
    }

    #[test]
    fn test_named_workspace_flag() {
        let cli_arg = CLIArg::parse_from([
            "rust-analyzer-mcp-server",
            "--workspace",
            "/explicit/workspace",
        ]);
        assert_eq!(
            cli_arg.workspace,
            Some(PathBuf::from("/explicit/workspace"))
        );
        assert_eq!(
            cli_arg.resolve_workspace_path(),
            PathBuf::from("/explicit/workspace")
        );

        let cli_arg_short =
            CLIArg::parse_from(["rust-analyzer-mcp-server", "-w", "/short/workspace"]);
        assert_eq!(
            cli_arg_short.workspace,
            Some(PathBuf::from("/short/workspace"))
        );
        assert_eq!(
            cli_arg_short.resolve_workspace_path(),
            PathBuf::from("/short/workspace")
        );
    }

    #[test]
    fn test_log_level_flags() {
        let test_cases = [
            ("off", LogLevel::Off, LevelFilter::OFF),
            ("error", LogLevel::Error, LevelFilter::ERROR),
            ("warn", LogLevel::Warn, LevelFilter::WARN),
            ("info", LogLevel::Info, LevelFilter::INFO),
            ("debug", LogLevel::Debug, LevelFilter::DEBUG),
            ("trace", LogLevel::Trace, LevelFilter::TRACE),
        ];

        for (arg_str, expected_level, expected_filter) in test_cases {
            let cli_arg =
                CLIArg::parse_from(["rust-analyzer-mcp-server", "--log-level", arg_str]);
            assert_eq!(cli_arg.log_level, expected_level);
            let config = cli_arg.to_tracing_config();
            assert_eq!(config.level_filter, expected_filter);
        }
    }

    #[test]
    fn test_log_level_off_writer_config() {
        let cli_arg =
            CLIArg::parse_from(["rust-analyzer-mcp-server", "--log-level", "off"]);
        let config = cli_arg.to_tracing_config();
        assert_eq!(config.level_filter, LevelFilter::OFF);
        assert_eq!(config.writer_config, WriterConfig::None);
    }

    #[test]
    fn test_log_file_flag() {
        let cli_arg = CLIArg::parse_from([
            "rust-analyzer-mcp-server",
            "--log-file",
            "/tmp/custom_mcp_log",
            "--log-level",
            "debug",
        ]);
        assert_eq!(cli_arg.log_file, Some("/tmp/custom_mcp_log".to_string()));
        assert_eq!(cli_arg.log_level, LogLevel::Debug);

        let config = cli_arg.to_tracing_config();
        assert_eq!(config.level_filter, LevelFilter::DEBUG);
        assert_eq!(
            config.writer_config,
            WriterConfig::DisplayAndFile(
                DisplayPreference::Stderr,
                "/tmp/custom_mcp_log".to_string()
            )
        );
    }
}

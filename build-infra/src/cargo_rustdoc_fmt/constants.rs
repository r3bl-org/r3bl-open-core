// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Shared syntax constants, markers, and delimiters for `cargo-rustdoc-fmt`.

// -----------------------------------------------------------------------------
// Rustdoc comment markers & directives
// -----------------------------------------------------------------------------

/// Inner doc comment marker (`//!`).
pub const INNER_DOC_MARKER: &str = "//!";

/// Outer doc comment marker (`///`).
pub const OUTER_DOC_MARKER: &str = "///";

/// Four-slash regular comment prefix (`////`), which is NOT a rustdoc comment.
pub const FOUR_SLASH_COMMENT_PREFIX: &str = "////";

/// File-level directive to skip `cargo-rustdoc-fmt` processing.
pub const RUSTDOC_FMT_SKIP_DIRECTIVE: &str = "// rustdoc-fmt: skip";

/// Single space character separating a doc marker (`///` or `//!`) from content.
pub const CONTENT_SPACE_CHAR: char = ' ';

/// Single space string slice.
pub const SINGLE_SPACE_STR: &str = " ";

// -----------------------------------------------------------------------------
// Line endings
// -----------------------------------------------------------------------------

/// Windows CRLF line ending (`\r\n`).
pub const CRLF_LINE_ENDING: &str = "\r\n";

/// Unix LF line ending (`\n`).
pub const LF_LINE_ENDING: &str = "\n";

/// Unix LF line ending character (`\n`).
pub const LF_LINE_ENDING_CHAR: char = '\n';

// -----------------------------------------------------------------------------
// Code fences & inline code spans
// -----------------------------------------------------------------------------

/// Markdown code fence delimiter (```` ``` ````).
pub const CODE_FENCE_DELIMITER: &str = "```";

/// Opening code fence prefix with redundant `rust` language tag (```` ```rust ````).
pub const RUST_FENCE_PREFIX: &str = "```rust";

/// Comma character (`,`) separating language tag from attributes in code fences (e.g.,
/// `rust,ignore`).
pub const FENCE_ATTR_COMMA: char = ',';

/// Backtick byte (`` b'`' ``).
pub const BACKTICK_BYTE: u8 = b'`';

/// Backtick character (`` ` ``).
pub const BACKTICK_CHAR: char = '`';

/// Opening prefix of a backticked Markdown link (`` "[`" ``).
pub const BACKTICKED_LINK_PREFIX: &str = "[`";

// -----------------------------------------------------------------------------
// Markdown tables
// -----------------------------------------------------------------------------

/// Pipe character (`|`) delimiting Markdown table cells.
pub const TABLE_CELL_DELIMITER: char = '|';

/// Formatted Markdown table row opening (`"| "`).
pub const TABLE_ROW_PREFIX: &str = "| ";

/// Formatted Markdown table row closing (`" |"`).
pub const TABLE_ROW_SUFFIX: &str = " |";

/// Separator between formatted Markdown table cells (`" | "`).
pub const TABLE_CELL_JOINER: &str = " | ";

/// Colon character (`:`) used for column alignment in `GFM` table separator rows.
pub const TABLE_ALIGN_COLON: char = ':';

/// Dash character (`-`) used in `GFM` table separator rows.
pub const TABLE_SEPARATOR_DASH: char = '-';

/// Dash string (`"-"`) repeated to build `GFM` table separator cells.
pub const TABLE_SEPARATOR_DASH_STR: &str = "-";

/// Minimum width of a `GFM` table column separator cell (`---`).
pub const MIN_TABLE_COL_WIDTH: usize = 3;

// -----------------------------------------------------------------------------
// Markdown links, reference definitions & blockquotes
// -----------------------------------------------------------------------------

/// Opening bracket (`[`) of a Markdown link label or reference definition.
pub const LINK_LABEL_OPEN: char = '[';

/// Opening bracket byte (`b'['`) of a Markdown link label or reference definition.
pub const LINK_LABEL_OPEN_BYTE: u8 = b'[';

/// Closing bracket (`]`) of a Markdown link label or reference definition.
pub const LINK_LABEL_CLOSE: char = ']';

/// Closing bracket byte (`b']'`) of a Markdown link label or reference definition.
pub const LINK_LABEL_CLOSE_BYTE: u8 = b']';

/// Opening parenthesis (`(`) of an inline Markdown link target (`[text](url)`).
pub const LINK_TARGET_OPEN: char = '(';

/// Opening parenthesis byte (`b'('`) of an inline Markdown link target (`[text](url)`).
pub const LINK_TARGET_OPEN_BYTE: u8 = b'(';

/// Closing parenthesis (`)`) of an inline Markdown link target (`[text](url)`).
pub const LINK_TARGET_CLOSE: char = ')';

/// Closing parenthesis byte (`b')'`) of an inline Markdown link target (`[text](url)`).
pub const LINK_TARGET_CLOSE_BYTE: u8 = b')';

/// Colon prefix (`":"`) following `[label]` in a Markdown reference definition.
pub const LINK_REF_COLON_PREFIX: &str = ":";

/// Colon byte (`b':'`) following `[label]` in a Markdown reference definition.
pub const LINK_REF_COLON_BYTE: u8 = b':';

/// Double-quote character (`"`) enclosing an optional link title.
pub const LINK_TITLE_QUOTE: char = '"';

/// Hash character (`#`) starting an intra-page anchor link.
pub const ANCHOR_LINK_PREFIX: char = '#';

/// Exclamation mark byte (`b'!'`) preceding an inline image link (`![alt](url)`).
pub const IMAGE_LINK_PREFIX_BYTE: u8 = b'!';

/// Opening angle bracket byte (`b'<'`) of a Markdown autolink (`<https://...>`).
pub const AUTOLINK_OPEN_BYTE: u8 = b'<';

/// Closing angle bracket byte (`b'>'`) of a Markdown autolink (`<https://...>`).
pub const AUTOLINK_CLOSE_BYTE: u8 = b'>';

/// HTTPS URL scheme prefix (`"https://"`).
pub const HTTPS_URL_PREFIX: &str = "https://";

/// HTTP URL scheme prefix (`"http://"`).
pub const HTTP_URL_PREFIX: &str = "http://";

/// URL scheme prefixes scanned when protecting inline and autolink URLs.
pub const URL_SCHEMES: [&str; 2] = [HTTPS_URL_PREFIX, HTTP_URL_PREFIX];

/// Blockquote line prefix character (`>`).
pub const BLOCKQUOTE_PREFIX: char = '>';

/// Rust path separator (`"::"`) in intra-doc link targets.
pub const RUST_PATH_SEPARATOR: &str = "::";

/// Underscore byte (`b'_'`) treated as an identifier word character when matching terms.
pub const UNDERSCORE_BYTE: u8 = b'_';

// -----------------------------------------------------------------------------
// Workspace discovery & cargo fmt runner
// -----------------------------------------------------------------------------

/// Cargo manifest filename (`"Cargo.toml"`).
pub const CARGO_TOML_FILE_NAME: &str = "Cargo.toml";

/// Workspace section header in `Cargo.toml` (`"[workspace]"`).
pub const WORKSPACE_SECTION_HEADER: &str = "[workspace]";

/// Package section header in `Cargo.toml` (`"[package]"`).
pub const PACKAGE_SECTION_HEADER: &str = "[package]";

/// Cargo build artifact directory name (`"target"`).
pub const TARGET_DIR_NAME: &str = "target";

/// Prefix character for hidden files and directories (`'.'`).
pub const HIDDEN_PATH_PREFIX: char = '.';

/// Default directory substring excluded from file discovery (`"test_data"`).
pub const DEFAULT_SKIP_FOLDER_MATCH: &str = "test_data";

/// Cargo `CLI` executable name (`"cargo"`).
pub const CARGO_PROGRAM: &str = "cargo";

/// Cargo format subcommand name (`"fmt"`).
pub const FMT_SUBCOMMAND: &str = "fmt";

/// Argument separator passed before file paths (`"--"`).
pub const CARGO_ARGS_SEPARATOR: &str = "--";

// -----------------------------------------------------------------------------
// HTML TOC comment markers
// -----------------------------------------------------------------------------

/// Opening HTML comment marker for generated table-of-contents blocks.
pub const TOC_OPEN_MARKER: &str = "<!-- TOC";

/// Closing HTML comment marker for generated table-of-contents blocks.
pub const TOC_CLOSE_MARKER: &str = "<!-- /TOC";

// -----------------------------------------------------------------------------
// CLI line range delimiters & file extensions
// -----------------------------------------------------------------------------

/// Rust inclusive range delimiter (`"10..=20"`).
pub const RANGE_DELIM_DOT_DOT_EQ: &str = "..=";

/// Rust range delimiter (`"10..20"`, treated as inclusive on the `CLI`).
pub const RANGE_DELIM_DOT_DOT: &str = "..";

/// Colon range delimiter (`"10:20"`).
pub const RANGE_DELIM_COLON: &str = ":";

/// Hyphen range delimiter (`"10-20"`).
pub const RANGE_DELIM_HYPHEN: &str = "-";

/// Supported `CLI` range delimiters in precedence order (`"..="` before `".."`).
pub const RANGE_DELIMITERS: [&str; 4] = [
    RANGE_DELIM_DOT_DOT_EQ,
    RANGE_DELIM_DOT_DOT,
    RANGE_DELIM_COLON,
    RANGE_DELIM_HYPHEN,
];

/// Prefix indicating a negative line number (`"-5"`).
pub const NEGATIVE_NUMBER_PREFIX: char = '-';

/// Rust source file extension (`"rs"`).
pub const RUST_FILE_EXTENSION: &str = "rs";

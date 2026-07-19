// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::Add;

/// Ergonomic configuration struct for initializing an [`EditorBuffer`].
///
/// It holds optional metadata:
/// 1. [`maybe_file_extension`]: Used for syntax highlighting (e.g. `md`).
/// 2. [`maybe_file_path`]: Associated file path for the buffer.
///
/// # Examples
///
/// It supports flexible creation via [`From`] trait implementations and an addition
/// operator [`Add`] DSL. Any of the following types can be passed directly into
/// [`EditorBuffer::new_empty()`] as arguments:
///
/// ```rust
/// use r3bl_tui::{EditorBuffer, FileExtensionToken, FilePathToken};
///
/// // 1. Unit type `()`: no extension, no file path.
/// let buffer = EditorBuffer::new_empty(());
///
/// // 2. `FileExtensionToken`: file extension set, no file path.
/// let buffer = EditorBuffer::new_empty(FileExtensionToken("md"));
///
/// // 3. `FilePathToken`: no extension, file path set.
/// let buffer = EditorBuffer::new_empty(FilePathToken("test.rs"));
///
/// // 4. `+` Operator DSL (either order): both extension and path set.
/// let buffer = EditorBuffer::new_empty(
///     FileExtensionToken("rs") + FilePathToken("src/main.rs"));
/// let buffer = EditorBuffer::new_empty(
///     FilePathToken("src/main.rs") + FileExtensionToken("rs"));
/// ```
///
/// [`Add`]: std::ops::Add
/// [`EditorBuffer::new_empty()`]: crate::EditorBuffer::new_empty
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`FileExtensionToken`]: crate::FileExtensionToken
/// [`FilePathToken`]: crate::FilePathToken
/// [`From`]: std::convert::From
/// [`maybe_file_extension`]: EditorBufferConfig::maybe_file_extension
/// [`maybe_file_path`]: EditorBufferConfig::maybe_file_path
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditorBufferConfig<'a> {
    /// Optional file extension used for syntax highlighting (e.g., `md`).
    pub maybe_file_extension: Option<&'a str>,

    /// Optional file path associated with the editor buffer (e.g., `src/main.rs`).
    pub maybe_file_path: Option<&'a str>,
}

/// Newtype constructor token representing a file extension (e.g., `rs`).
///
/// This is a single-purpose constructor DSL token used at call sites for ergonomic buffer
/// construction and disambiguation. It is not stored directly inside
/// [`EditorBufferConfig`].
///
/// [`EditorBufferConfig`]: crate::EditorBufferConfig
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileExtensionToken<'a>(pub &'a str);

/// Newtype constructor token representing a file path (e.g., `src/main.rs`).
///
/// This is a single-purpose constructor DSL token used at call sites for ergonomic buffer
/// construction and disambiguation. It is not stored directly inside
/// [`EditorBufferConfig`].
///
/// [`EditorBufferConfig`]: crate::EditorBufferConfig
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePathToken<'a>(pub &'a str);

// XMARK: Elegant Constructor DSL.

/// This module implements the "heavy lifting" for the Elegant Constructor DSL Pattern.
///
/// # Architecture: Constructor DSL Tokens vs Storage Types
///
/// 1. **Constructor DSL Tokens** ([`FileExtensionToken`], [`FilePathToken`]):
///    - Single-purpose newtypes created solely for ergonomics and disambiguation at call
///      sites.
///    - Allow overloading the [`Add`] (`+`) operator to compose configuration parameters
///      in any order.
///    - Enable [`From`] / [`Into`] conversions on constructor parameters like
///      [`EditorBuffer::new_empty`].
///
/// 2. **Canonical Config Struct** ([`EditorBufferConfig`]):
///    - Transient intermediate aggregator that holds resolved configuration values.
///    - Stores `Option<&'a str>` directly instead of nesting the newtypes, as field names
///      provide semantic context without double-wrapping overhead.
///
/// 3. **Internal Storage** ([`EditorContent`]):
///    - Owns and stores the final data structures ([`TinyInlineString`],
///      [`InlineString`]).
///
/// [`Add`]: std::ops::Add
/// [`EditorBuffer::new_empty`]: crate::EditorBuffer::new_empty
/// [`EditorBufferConfig`]: crate::EditorBufferConfig
/// [`EditorContent`]: crate::EditorContent
/// [`FileExtensionToken`]: crate::FileExtensionToken
/// [`FilePathToken`]: crate::FilePathToken
/// [`From`]: std::convert::From
/// [`InlineString`]: crate::InlineString
/// [`Into`]: std::convert::Into
/// [`TinyInlineString`]: crate::TinyInlineString
mod impl_elegant_constructor_dsl_pattern {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // Convert a single token type (or unit type) into a config.

    impl From<()> for EditorBufferConfig<'_> {
        fn from((): ()) -> Self {
            Self {
                maybe_file_extension: None,
                maybe_file_path: None,
            }
        }
    }

    impl<'a> From<FileExtensionToken<'a>> for EditorBufferConfig<'a> {
        fn from(FileExtensionToken(ext): FileExtensionToken<'a>) -> Self {
            Self {
                maybe_file_extension: Some(ext),
                maybe_file_path: None,
            }
        }
    }

    impl<'a> From<FilePathToken<'a>> for EditorBufferConfig<'a> {
        fn from(FilePathToken(path): FilePathToken<'a>) -> Self {
            Self {
                maybe_file_extension: None,
                maybe_file_path: Some(path),
            }
        }
    }

    // Combine two tokens with `+` to create a config (in either order).

    impl<'a> Add<FilePathToken<'a>> for FileExtensionToken<'a> {
        type Output = EditorBufferConfig<'a>;

        fn add(self, rhs: FilePathToken<'a>) -> Self::Output {
            EditorBufferConfig::from(self) + EditorBufferConfig::from(rhs)
        }
    }

    impl<'a> Add<FileExtensionToken<'a>> for FilePathToken<'a> {
        type Output = EditorBufferConfig<'a>;

        fn add(self, rhs: FileExtensionToken<'a>) -> Self::Output {
            EditorBufferConfig::from(self) + EditorBufferConfig::from(rhs)
        }
    }

    // Merge two configs with `+` using `Option::or`.

    impl Add for EditorBufferConfig<'_> {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            Self {
                maybe_file_extension: self
                    .maybe_file_extension
                    .or(rhs.maybe_file_extension),
                maybe_file_path: self.maybe_file_path.or(rhs.maybe_file_path),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_editor_buffer_config_dsl() {
        // 1. Default and From<()> conversion.
        let default_config = EditorBufferConfig::default();
        let unit_config: EditorBufferConfig = ().into();
        assert_eq2!(unit_config, default_config);
        assert_eq2!(unit_config.maybe_file_extension, None);
        assert_eq2!(unit_config.maybe_file_path, None);

        // 2. FileExtension conversion.
        let ext_config: EditorBufferConfig = FileExtensionToken("md").into();
        assert_eq2!(
            ext_config,
            EditorBufferConfig {
                maybe_file_extension: Some("md"),
                maybe_file_path: None,
            }
        );

        // 3. FilePath conversion.
        let path_config: EditorBufferConfig = FilePathToken("test.rs").into();
        assert_eq2!(
            path_config,
            EditorBufferConfig {
                maybe_file_extension: None,
                maybe_file_path: Some("test.rs"),
            }
        );

        // 4. FileExtension + FilePath DSL (both orders).
        let combined_a = FileExtensionToken("rs") + FilePathToken("src/main.rs");
        let combined_b = FilePathToken("src/main.rs") + FileExtensionToken("rs");
        let expected = EditorBufferConfig {
            maybe_file_extension: Some("rs"),
            maybe_file_path: Some("src/main.rs"),
        };
        assert_eq2!(combined_a, expected);
        assert_eq2!(combined_b, expected);
        assert_eq2!(combined_a, combined_b);

        // 5. EditorBufferConfig + EditorBufferConfig.
        let config_ext: EditorBufferConfig = FileExtensionToken("rs").into();
        let config_path: EditorBufferConfig = FilePathToken("src/main.rs").into();
        assert_eq2!(config_ext + config_path, expected);
        assert_eq2!(config_path + config_ext, expected);
    }
}

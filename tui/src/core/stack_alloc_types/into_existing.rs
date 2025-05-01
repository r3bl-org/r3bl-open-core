/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

// XMARK: Clever Rust, use of decl macro w/ `tt` to allow any number of arguments.

/// A macro to pad a [crate::InlineString] (which is allocated elsewhere) with a
/// specified string repeated a specified number of times.
///
/// # Arguments
///
/// * `fmt: $acc` - The accumulator to write the padding into. It can be [String],
///   [crate::InlineString], [crate::TinyInlineString], or [std::fmt::Formatter], basically
///   anything that implements [std::fmt::Write].
/// * `pad_str: $pad_str` - The string to use for padding.
/// * `repeat_count: $repeat_count` - The number of times to repeat the padding string.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{pad_fmt, InlineString};
///
/// let mut acc = InlineString::new();
/// pad_fmt!(fmt: acc, pad_str: "-", repeat_count: 5);
/// assert_eq!(acc, "-----");
///
/// use std::fmt::{Debug, Result, Formatter};
/// struct Foo;
/// impl Debug for Foo {
///     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
///         pad_fmt!(
///             fmt: f,
///             pad_str: "X",
///             repeat_count: 3
///         );
///         Ok(())
///     }
/// }
/// assert_eq!(format!("{:?}", Foo), "XXX");
/// ```
#[macro_export]
macro_rules! pad_fmt {
    (
        fmt: $acc:expr,
        pad_str: $pad_str:expr,
        repeat_count: $repeat_count:expr
    ) => {{
        #[allow(clippy::reversed_empty_ranges)]
        for _ in 0..$repeat_count {
            use std::fmt::Write as _;
            _ = write!($acc, "{}", $pad_str);
        }
    }};
}

#[cfg(test)]
mod tests_pad_fmt {
    use crate::InlineString;
    #[test]
    fn test_pad() {
        let mut acc = InlineString::new();
        pad_fmt!(fmt: acc, pad_str: "-", repeat_count: 5);
        assert_eq!(acc, "-----");
    }

    #[test]
    fn test_pad_zero() {
        let mut acc = InlineString::new();
        pad_fmt!(fmt: acc, pad_str: "-", repeat_count: 0);
        assert_eq!(acc, "");
    }

    #[test]
    fn test_pad_multiple() {
        let mut acc = InlineString::new();
        pad_fmt!(fmt: acc, pad_str: "abc", repeat_count: 3);
        // cspell:disable-next-line
        assert_eq!(acc, "abcabcabc");
    }
}

/// This macro is similar to [crate::join!] except that it also receives a
/// [std::fmt::Formatter] in order to write the display output into without allocating
/// anything. It does not return any errors.
///
/// # Arguments
///
/// - `fmt` can be a [String], [crate::InlineString], [crate::TinyInlineString], or
///   [std::fmt::Formatter], basically anything that implements [std::fmt::Write].
/// - `from` is the collection to iterate over.
/// - `each` is the identifier for each item in the collection.
/// - `delim` is the delimiter to insert between items.
/// - `format` is the format to apply to each item. This is whatever you would pass to
///   [format!] or [write!].
#[macro_export]
macro_rules! join_fmt {
    (
        fmt: $fmt:expr,
        from: $collection:expr,
        each: $item:ident,
        delim: $delim:expr,
        format: $($format:tt)*
    ) => {{
        let mut iter = $collection.iter();
        // First item.
        if let Some($item) = iter.next() {
            _ = write!($fmt, $($format)*);
        }
        // Rest of the items.
        for $item in iter {
            _ = write!($fmt, "{}", $delim);
            _ = write!($fmt, $($format)*);
        }
    }};
}

#[cfg(test)]
mod join_fmt_tests {
    struct Foo {
        items: Vec<String>,
    }

    impl std::fmt::Display for Foo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            join_fmt!(
                fmt: f,
                from: self.items,
                each: item,
                delim: ", ",
                format: "'{item}'"
            );
            Ok(())
        }
    }

    #[test]
    fn test_join_fmt() {
        let items = ["apple", "banana", "cherry"];
        let foo = Foo {
            items: items.iter().map(|s| s.to_string()).collect(),
        };
        let result = format!("{foo}");
        assert_eq!(result, "'apple', 'banana', 'cherry'");
    }
}

/// This macro is similar to [crate::join_with_index!] except that it also receives a
/// [std::fmt::Formatter] to write the display output into without allocating
/// anything.
///
/// # Arguments
///
/// * `fmt: $acc` - The accumulator to write the padding into. It can be [String],
///   [crate::InlineString], [crate::TinyInlineString], or [std::fmt::Formatter], basically
///   anything that implements [std::fmt::Write].
/// * `from: $collection` - The collection to iterate over.
/// * `each: $item` - The identifier for each item in the collection.
/// * `index: $index` - The identifier for the index of each item in the collection.
/// * `delim: $delim` - The delimiter to insert between items.
/// * `format: $($format:tt)*` - The format to apply to each item. This is whatever you
///   would pass to [format!] or [write!].
#[macro_export]
macro_rules! join_with_index_fmt {
    (
        fmt: $fmt:expr,
        from: $collection:expr,
        each: $item:ident,
        index: $index:ident,
        delim: $delim:expr,
        format: $($format:tt)*
    ) => {{
        let mut iter = $collection.iter().enumerate();
        // First item.
        if let Some(($index, $item)) = iter.next() {
            _ = write!($fmt, $($format)*);
        }
        // Rest of the items.
        for ($index, $item) in iter {
            _ = write!($fmt, "{}", $delim);
            _ = write!($fmt, $($format)*);
        }
    }};
}

#[cfg(test)]
mod join_with_index_fmt_tests {
    struct Foo {
        items: Vec<String>,
    }

    impl std::fmt::Display for Foo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            join_with_index_fmt!(
                fmt: f,
                from: self.items,
                each: item,
                index: index,
                delim: ", ",
                format: "[{index}]'{item}'"
            );
            Ok(())
        }
    }

    #[test]
    fn test_join_with_index_fmt() {
        let items = ["apple", "banana", "cherry"];
        let foo = Foo {
            items: items.iter().map(|s| s.to_string()).collect(),
        };
        let result = format!("{foo}");
        assert_eq!(result, "[0]'apple', [1]'banana', [2]'cherry'");
    }
}

pub mod read_from_file {
    use std::{fs::File, io::Read, path::PathBuf, str::from_utf8};

    use miette::{Context, IntoDiagnostic};
    use smallstr::SmallString;
    use smallvec::Array;

    use crate::{ok, DEFAULT_READ_BUFFER_SIZE};

    // XMARK: Clever Rust, use of `A` to allow any size `Array` to be passed in.

    /// The generic argument `A` ensures that this function can mutate any size `Array` of
    /// `u8` that it receives. This removes any restrictions on this function knowing the
    /// size of the `Array` it receives.
    ///
    /// However, the caller, of this function, must still use `const` to specify the size
    /// of the `Array` they want to use (since this is stack allocated). It does not
    /// remove this restriction on their side, since it to be stack allocated the
    /// structure must be `Sized`, ie, its size known at compile time.
    ///
    /// The caller can make some reasonable assumptions (based on profiling or the nature
    /// of workloads that their code is used in) to determine what `const` size the
    /// `Array` should be. If it is greater than this, it will spill to the heap, and it
    /// is too small, then some memory will be wasted on the stack.
    pub fn try_read_file_path_into_inline_string<A: Array<Item = u8>>(
        acc: &mut SmallString<A>,
        arg_path: impl Into<PathBuf>,
    ) -> miette::Result<()> {
        // Open the file.
        let file_path: PathBuf = arg_path.into();
        let mut file = File::open(&file_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open file {}", file_path.display()))?;

        // Create a buffer to hold the file contents.
        let mut read_buffer = [0u8; DEFAULT_READ_BUFFER_SIZE];

        // Read the entire file's content in chunks and append to the SmallString.
        loop {
            let num_bytes_read = file.read(&mut read_buffer).into_diagnostic()?;
            if num_bytes_read == 0 {
                break;
            }
            acc.push_str(from_utf8(&read_buffer[..num_bytes_read]).into_diagnostic()?);
        }

        ok!()
    }
}

pub mod write_to_file {
    use std::{fs::File, io::Write, path::PathBuf};

    use miette::IntoDiagnostic as _;

    use crate::CommonResult;

    pub fn try_write_str_to_file(path: &PathBuf, contents: &str) -> CommonResult<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(contents.as_bytes()).into_diagnostic()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests_write_to_file {
    use std::fs;

    use miette::IntoDiagnostic;

    use crate::{create_temp_dir, into_existing::write_to_file::try_write_str_to_file};

    #[test]
    fn test_try_write_file_contents_success() -> miette::Result<()> {
        // 1. Create a temporary directory.
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");
        let file_path = temp_dir.as_path().join("test_output.txt");

        // 2. Define the content to write.
        let content = "Hello, world from test!";

        // 3. Call the function under test.
        let result = try_write_str_to_file(&file_path, content);

        // 4. Assert that the write was successful.
        assert!(result.is_ok());

        // 5. Verify the file content.
        let read_content = fs::read_to_string(&file_path).into_diagnostic()?;
        assert_eq!(read_content, content);

        // 6. Temp dir is automatically cleaned up when `temp_dir` goes out of scope.
        Ok(())
    }

    #[test]
    fn test_try_write_file_contents_invalid_path() {
        // 1. Create a temporary directory (we still need it as a base).
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");

        // 2. Define an invalid path *relative* to the temp dir, pointing to a
        //    subdirectory that won't exist.
        let invalid_path = temp_dir
            .as_path()
            .join("non_existent_sub_dir")
            .join("test_output.txt");
        let content = "This won't be written";

        // 3. Call the function under test.
        let result = try_write_str_to_file(&invalid_path, content);

        // 4. Assert that the write failed (because the parent directory doesn't exist).
        assert!(result.is_err());

        // 5. Temp dir is automatically cleaned up.
    }
}

#[cfg(test)]
mod tests_read_from_file {
    use std::{fs::File, io::Write};

    use crate::{create_temp_dir,
                into_existing::read_from_file::try_read_file_path_into_inline_string,
                DocumentStorage,
                InlineString,
                DEFAULT_DOCUMENT_SIZE};

    #[test]
    fn test_read_tiny_file_into_inline_string() {
        // Create a temporary dir.
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");
        let temp_file_path = temp_dir.join("test_file.txt");

        // Create a temporary file & write some content into it.
        let content = "Hello, world!";
        let mut temp_file_handle =
            File::create(&temp_file_path).expect("Failed to create temp file");
        temp_file_handle
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        // Read the file into InlineString.
        let mut acc = InlineString::new();
        try_read_file_path_into_inline_string(&mut acc, temp_file_path)
            .expect("Failed to read file into InlineString");

        // Verify the content.
        assert_eq!(content, acc.as_str());
    }

    #[test]
    fn test_read_large_file_into_inline_string() {
        // Create a temporary file.
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");
        let temp_file_path = temp_dir.join("test_file.txt");
        let mut temp_file_handle =
            File::create(&temp_file_path).expect("Failed to create temp file");

        // Write some content to the temporary file.
        let content = "A".repeat(DEFAULT_DOCUMENT_SIZE);
        temp_file_handle
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        assert!(temp_file_path.exists());
        let temp_file_handle =
            File::open(&temp_file_path).expect("Failed to open temp file");
        assert!(
            temp_file_handle.metadata().unwrap().len()
                >= DEFAULT_DOCUMENT_SIZE.try_into().unwrap(),
            "File size is not greater than 1MB"
        );

        // Read the file into DocumentStorage.
        let mut acc = DocumentStorage::new();
        try_read_file_path_into_inline_string(&mut acc, temp_file_path)
            .expect("Failed to read file into DocumentStorage");

        // Verify the content.
        assert_eq!(content, acc.as_str());
        assert!(!acc.spilled());
    }

    #[test]
    fn test_read_empty_file_into_inline_string() {
        // Create a temporary file.
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");
        let temp_file_path = temp_dir.join("test_file.txt");
        _ = File::create(&temp_file_path).expect("Failed to create temp file");

        // Read the file into InlineString.
        let mut acc = InlineString::new();
        try_read_file_path_into_inline_string(&mut acc, temp_file_path)
            .expect("Failed to read file into InlineString");

        // Verify the content.
        assert_eq!("", acc.as_str());
    }

    #[test]
    fn test_read_nonexistent_file_into_inline_string() {
        // Attempt to read a nonexistent file.
        let mut acc = InlineString::new();
        let temp_dir = create_temp_dir().expect("Failed to create temp dir");
        let temp_file_path = temp_dir.join("nonexistent_file.txt");
        let result = try_read_file_path_into_inline_string(&mut acc, temp_file_path);

        // Verify that an error is returned.
        assert!(result.is_err());
    }
}

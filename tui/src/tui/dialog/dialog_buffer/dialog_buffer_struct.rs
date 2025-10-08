// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChUnit, DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, InlineString, ItemsOwned, ch,
            fmt_option};
use std::fmt::{Debug, Display, Formatter, Result};

/// Please do not construct this struct directly and use
/// [`new_empty`](DialogBuffer::new_empty) instead.
///
/// Stores the data for a modal dialog. It contains the text content in an
/// [`EditorBuffer`] and a title that is displayed.
#[derive(Clone, PartialEq)]
pub struct DialogBuffer {
    pub editor_buffer: EditorBuffer,
    pub title: InlineString,
    pub maybe_results: Option<ItemsOwned>,
}

impl DialogBuffer {
    #[must_use]
    pub fn get_results_count(&self) -> ChUnit {
        if let Some(ref it) = self.maybe_results {
            ch(it.len())
        } else {
            ch(0)
        }
    }
}

impl DialogBuffer {
    #[must_use]
    pub fn new_empty() -> Self {
        DialogBuffer {
            editor_buffer: EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None),
            title: InlineString::new(),
            maybe_results: None,
        }
    }
}

mod impl_debug {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for DialogBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let maybe_results: &dyn Debug = fmt_option!(&self.maybe_results);
            write!(
                f,
                "DialogBuffer [
  - title: {title}
  - maybe_results: {results:?}
  - editor_buffer.content: {content}
]",
                title = self.title,
                results = maybe_results,
                content = self
                    .editor_buffer
                    .get_as_string_with_comma_instead_of_newlines()
            )
        }
    }
}

/// Efficient Display implementation for telemetry logging.
mod impl_display {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Display for DialogBuffer {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffer. This is used for telemetry reporting, and it is expected
        /// to be fast, since it is called in a hot loop, on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Start with dialog identifier and title.
            write!(f, "dialog:")?;

            // Add title if not empty, otherwise use <untitled> convention.
            if self.title.is_empty() {
                write!(f, "<untitled>")?;
            } else {
                write!(f, "{}", self.title)?;
            }

            // Add results count if available.
            if let Some(ref results) = self.maybe_results {
                write!(f, ":results({})", results.len())?;
            }

            // Delegate to EditorBuffer's efficient Display implementation.
            // This already includes line count and memory size info.
            write!(f, ":{}", self.editor_buffer)?;

            Ok(())
        }
    }
}

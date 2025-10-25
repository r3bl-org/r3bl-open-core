// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::RenderOpCommon;
use std::fmt::{Formatter, Result};

/// Trait for formatting [`RenderOpCommon`] instances in debug output.
///
/// This trait abstracts debug formatting logic, allowing different
/// terminal backends to provide their own specialized debug representations
/// of common render operations.
///
/// # Backend Specialization
///
/// Different terminal backends can provide custom debug output formats for
/// [`RenderOpCommon`] variants, enabling backend-specific insights into
/// rendering operations.
pub trait RenderOpDebugFormat {
    /// Formats the `RenderOpCommon` for debug output.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to the formatter fails.
    fn fmt_debug(&self, this: &RenderOpCommon, f: &mut Formatter<'_>) -> Result;
}

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Conversion helper trait [`IntoErr`] for lifting error-like values into
//! [`miette::Result`].

/// Trait to convert an error-like type into a [`miette::Result`].
pub trait IntoErr {
    /// Converts the type into a [`miette::Result`].
    ///
    /// # Errors
    ///
    /// Always returns an [`Err`] containing the converted error.
    fn into_err<T>(self) -> miette::Result<T>;
}

impl IntoErr for miette::Report {
    fn into_err<T>(self) -> miette::Result<T> { Err(self) }
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::friendly_random_id::generate_friendly_random_id;
use miette::IntoDiagnostic;
use std::{fmt::{Display, Formatter},
          ops::Deref,
          path::Path};

#[derive(Debug)]
pub struct TempDir {
    pub inner: std::path::PathBuf,
}

impl TempDir {
    /// Join a path to the temporary directory.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> std::path::PathBuf {
        self.inner.join(path)
    }
}

/// Create a temporary directory. The directory is automatically deleted when the
/// [`TempDir`] struct is dropped.
///
/// You might want to use the [`crate::try_create_temp_dir_and_cd`!] macro instead,
/// which creates a subdirectory inside the temp dir and changes to that subdirectory.
///
/// # Errors
///
/// Returns an error if:
/// - The temp directory cannot be created due to insufficient permissions
/// - The file system is full
/// - I/O errors occur during directory creation
pub fn try_create_temp_dir() -> miette::Result<TempDir> {
    let root = std::env::temp_dir();
    let new_temp_dir = root.join(generate_friendly_random_id().as_str());
    std::fs::create_dir(&new_temp_dir).into_diagnostic()?;
    Ok(TempDir {
        inner: new_temp_dir,
    })
}

/// Macro to create a temp dir, a sub dir inside it, and change to that sub dir. It
/// returns a tuple containing:
///
/// 1. [`TempDir`] struct that contains the path to the newly created temp dir (the root).
///    Hold on to this and only drop it when you are done with the temp dir (since it will
///    be deleted when dropped).
/// 2. The [`std::path::PathBuf`] to the newly sub dir (inside the newly created temp dir
///    root).
///
/// # Example
///
/// ```no_run
/// # use r3bl_tui::{try_create_temp_dir_and_cd, ok};
/// # fn variant_1() -> miette::Result<()> {
///     let (temp_dir_root, sub_dir_path_buf) = try_create_temp_dir_and_cd!("sub_dir_name");
///     ok!()
/// } // temp_dir_root is dropped here and the temp dir is deleted.
///
/// # fn variant_2() -> miette::Result<()> {
///     let temp_dir_root = try_create_temp_dir_and_cd!();
///     ok!()
/// } // temp_dir_root is dropped here and the temp dir is deleted.
/// ```
///
/// # Warning, this changes the current working directory of the process
///
/// This macro will change the current working directory for current process. This is
/// potentially dangerous, as it can affect other parts of the program that rely on the
/// current working directory. Use with caution. An example of this is when running tests
/// in parallel in `cargo test`. `cargo test` runs all the tests in a single process. This
/// means that when one test changes the current working directory, it affects all other
/// tests that run after it.
#[macro_export]
macro_rules! try_create_temp_dir_and_cd {
    ($sub_dir:expr) => {{
        let temp_dir_root = $crate::try_create_temp_dir()?;
        let sub_dir_path = temp_dir_root.join($sub_dir);
        $crate::try_mkdir(
            &sub_dir_path,
            $crate::MkdirOptions::CreateIntermediateDirectories,
        )?;
        $crate::try_cd(&sub_dir_path)?;
        (temp_dir_root, sub_dir_path)
    }};
    () => {{
        let temp_dir_root = $crate::try_create_temp_dir()?;
        $crate::try_cd(&temp_dir_root)?;
        temp_dir_root
    }};
}

// XMARK: Clever Rust, use of Drop to perform transaction close / end.

/// Automatically delete the temporary directory when the [`TempDir`] struct is dropped.
impl Drop for TempDir {
    fn drop(&mut self) {
        // We don't care about the result of this operation.
        std::fs::remove_dir_all(&self.inner).ok();
    }
}

/// Allow access to the inner [`std::path::Path`] easily when using other APIs.
///
/// Implementing the [Deref] trait that exposes the inner [Path] is useful when using
/// other APIs that expect a [Path] instead of a [`TempDir`], such as:
/// - [`std::path::Path::join`]
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::try_create_temp_dir;
/// let root = try_create_temp_dir().unwrap();
/// let new_dir = root.join("test_set_file_executable");
/// ```
impl Deref for TempDir {
    type Target = std::path::PathBuf;

    fn deref(&self) -> &Self::Target { &self.inner }
}

/// Implement the [Display] trait to allow printing the [`TempDir`] struct.
/// This is useful when debugging or logging using:
/// - [println!]
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::try_create_temp_dir;
/// let root = try_create_temp_dir().unwrap();
/// println!("Temp dir: {}", root);
/// ```
impl Display for TempDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

/// Allow access to the inner [Path] easily when using other APIs.
///
/// Implementing the [`AsRef`] trait that exposes the inner [Path] is useful when using
/// other APIs that expect a [Path] instead of a [`TempDir`], such as:
/// - [`std::fs::create_dir_all`]
/// - [`std::fs::remove_dir_all`]
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::try_create_temp_dir;
/// let root = try_create_temp_dir().unwrap();
/// std::fs::create_dir_all(root.join("test_set_file_executable")).unwrap();
/// std::fs::remove_dir_all(root).unwrap();
/// ```
impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path { &self.inner }
}

#[cfg(test)]
mod tests_temp_dir {
    use super::*;
    use crate::{fg_lizard_green, ok};

    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_macro_try_create_temp_dir_and_cd() -> miette::Result<()> {
        // Create a temp dir and a sub dir inside it, then change to that sub dir.
        {
            let (temp_dir_root, sub_dir) = try_create_temp_dir_and_cd!("test_sub_dir");
            println!(
                "Temp dir root: {}",
                fg_lizard_green(temp_dir_root.inner.display().to_string())
            );
            println!(
                "Subfolder: {}",
                fg_lizard_green(sub_dir.display().to_string())
            );

            assert!(temp_dir_root.inner.exists());
            assert!(sub_dir.exists());

            let copy_of_path = temp_dir_root.inner.clone();

            drop(temp_dir_root);

            assert!(!copy_of_path.exists());
            assert!(!sub_dir.exists());
        }

        // Create a temp dir and change to it.
        {
            let temp_dir_root = try_create_temp_dir_and_cd!();
            println!(
                "Temp dir root: {}",
                fg_lizard_green(temp_dir_root.inner.display().to_string())
            );

            assert!(temp_dir_root.inner.exists());

            let copy_of_path = temp_dir_root.inner.clone();

            drop(temp_dir_root);

            assert!(!copy_of_path.exists());
        }

        ok!()
    }

    #[test]
    fn test_temp_dir() {
        let temp_dir = try_create_temp_dir().unwrap();
        println!(
            "Temp dir: {}",
            fg_lizard_green(temp_dir.inner.display().to_string())
        );

        assert!(temp_dir.inner.exists());
    }

    #[test]
    fn test_temp_dir_join() {
        let temp_dir = try_create_temp_dir().unwrap();
        let expected_prefix = temp_dir.inner.display().to_string();

        let new_sub_dir = temp_dir.join("test_set_file_executable");
        let expected_postfix = new_sub_dir.display().to_string();

        let expected_full_path = new_sub_dir.display().to_string();

        assert!(temp_dir.exists());
        assert!(!new_sub_dir.exists());
        assert!(expected_full_path.starts_with(&expected_prefix));
        assert!(expected_full_path.ends_with(&expected_postfix));
    }

    #[test]
    fn test_temp_dir_drop() {
        let temp_dir = try_create_temp_dir().unwrap();

        let copy_of_path = temp_dir.inner.clone();
        println!(
            "Temp dir: {}",
            fg_lizard_green(copy_of_path.display().to_string())
        );

        drop(temp_dir);

        assert!(!copy_of_path.exists());
    }
}

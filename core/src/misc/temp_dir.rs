/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use std::{fmt::{Display, Formatter},
          ops::Deref,
          path::Path};

use miette::IntoDiagnostic;

use crate::friendly_random_id;

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
/// [TempDir] struct is dropped.
pub fn create_temp_dir() -> miette::Result<TempDir> {
    let root = std::env::temp_dir();
    let new_temp_dir = root.join(friendly_random_id::generate_friendly_random_id());
    std::fs::create_dir(&new_temp_dir).into_diagnostic()?;
    Ok(TempDir {
        inner: new_temp_dir,
    })
}

// XMARK: Clever Rust, use of Drop to perform transactionn close / end.

/// Automatically delete the temporary directory when the [TempDir] struct is dropped.
impl Drop for TempDir {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.inner).unwrap(); }
}

/// Allow access to the inner [std::path::Path] easily when using other APIs.
///
/// Implementing the [Deref] trait that exposes the inner [Path] is useful when using
/// other APIs that expect a [Path] instead of a [TempDir], such as:
/// - [std::path::Path::join]
///
/// # Example
///
/// ```no_run
/// use r3bl_core::create_temp_dir;
/// let root = create_temp_dir().unwrap();
/// let new_dir = root.join("test_set_file_executable");
/// ```
impl Deref for TempDir {
    type Target = std::path::PathBuf;

    fn deref(&self) -> &Self::Target { &self.inner }
}

/// Implement the [Display] trait to allow printing the [TempDir] struct.
/// This is useful when debugging or logging using:
/// - [println!]
///
/// # Example
///
/// ```no_run
/// use r3bl_core::create_temp_dir;
/// let root = create_temp_dir().unwrap();
/// println!("Temp dir: {}", root);
/// ```
impl Display for TempDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

/// Allow access to the inner [Path] easily when using other APIs.
///
/// Implementing the [AsRef] trait that exposes the inner [Path] is useful when using
/// other APIs that expect a [Path] instead of a [TempDir], such as:
/// - [std::fs::create_dir_all]
/// - [std::fs::remove_dir_all]
///
/// # Example
///
/// ```no_run
/// use r3bl_core::create_temp_dir;
/// let root = create_temp_dir().unwrap();
/// std::fs::create_dir_all(root.join("test_set_file_executable")).unwrap();
/// std::fs::remove_dir_all(root).unwrap();
/// ```
impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path { &self.inner }
}

#[cfg(test)]
mod tests_temp_dir {
    use crossterm::style::Stylize as _;

    use super::*;

    #[test]
    fn test_temp_dir() {
        let temp_dir = create_temp_dir().unwrap();
        println!(
            "Temp dir: {}",
            temp_dir.inner.display().to_string().magenta()
        );

        assert!(temp_dir.inner.exists());
    }

    #[test]
    fn test_temp_dir_join() {
        let temp_dir = create_temp_dir().unwrap();
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
        let temp_dir = create_temp_dir().unwrap();

        let copy_of_path = temp_dir.inner.clone();
        println!("Temp dir: {}", copy_of_path.display().to_string().magenta());

        drop(temp_dir);

        assert!(!copy_of_path.exists());
    }
}

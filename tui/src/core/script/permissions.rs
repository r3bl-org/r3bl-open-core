// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{fs,
          path::{Path, PathBuf}};

/// Error returned by [`set_permission`] and [`try_set_file_executable`] when inspecting
/// metadata or updating file mode bits fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum FilePermissionsError {
    /// Failed to query filesystem metadata for `path`.
    #[error("Failed to read metadata for '{}'", path.display())]
    MetadataReadFailed {
        /// Target path passed to [`fs::metadata`].
        path: PathBuf,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Target `path` exists on disk, and is not a regular file (e.g., a directory).
    #[error("Path is not a regular file: '{}'", path.display())]
    NotARegularFile {
        /// Non-file path rejected by [`try_set_file_executable`].
        path: PathBuf,
    },

    /// Failed to apply Unix mode bits (`mode`) to `path`.
    #[error("Failed to set permissions ({mode:#o}) on '{}'", path.display())]
    SetPermissionsFailed {
        /// Target path passed to [`fs::set_permissions`].
        path: PathBuf,

        /// Requested Unix permission mode (e.g., `0o755`).
        mode: u32,

        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },
}

/// This is noop on Windows because Windows does not use the same permission model as
/// Unix-like systems. It is determined by file extension and ACLs (Access Control Lists).
///
/// # Errors
///
/// This function never returns an error on Windows.
#[cfg(target_os = "windows")]
pub fn set_permission(
    _file: impl AsRef<Path>,
    _mode: u32,
) -> Result<(), FilePermissionsError> {
    ok!()
}

/// # Errors
///
/// Returns [`FilePermissionsError::SetPermissionsFailed`] if:
/// - The file does not exist
/// - Insufficient permissions to change the file mode
/// - I/O error occurs while setting permissions
#[cfg(not(target_os = "windows"))]
pub fn set_permission(
    file: impl AsRef<Path>,
    mode: u32,
) -> Result<(), FilePermissionsError> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};
    let path = file.as_ref();
    fs::set_permissions(path, Permissions::from_mode(mode)).map_err(|src_err| {
        FilePermissionsError::SetPermissionsFailed {
            path: path.to_path_buf(),
            mode,
            src_err,
        }
    })
}

/// Sets the file at the specified path to be executable by owner, group, and others.
/// - `bash` equivalent: `chmod +x file`
/// - Eg: `set_file_executable("some_file.sh")`
/// - The `file` must exist and be a file (not a directory).
///
/// # Errors
///
/// Returns [`FilePermissionsError`] if:
/// - The file does not exist ([`FilePermissionsError::MetadataReadFailed`])
/// - The path points to a directory instead of a file
///   ([`FilePermissionsError::NotARegularFile`])
/// - Insufficient permissions or I/O error occurs while setting permissions
///   ([`FilePermissionsError::SetPermissionsFailed`])
pub fn try_set_file_executable(file: impl AsRef<Path>) -> Result<(), FilePermissionsError> {
    let file = file.as_ref();
    let metadata = fs::metadata(file).map_err(|src_err| {
        FilePermissionsError::MetadataReadFailed {
            path: file.to_path_buf(),
            src_err,
        }
    })?;

    if !metadata.is_file() {
        return Err(FilePermissionsError::NotARegularFile {
            path: file.to_path_buf(),
        });
    }

    // Set execute permissions for owner, group, and others on this file. 755 means:
    // - 7 (owner): read (4) + write (2) + execute (1) = 7 (rwx)
    // - 5 (group): read (4) + execute (1) = 5 (r-x)
    // - 5 (others): read (4) + execute (1) = 5 (r-x)
    set_permission(file, 0o755)
}

#[cfg(test)]
mod tests_permissions {
    use super::*;
    use crate::try_create_temp_dir;

    /// This is noop on Windows because Windows does not use the same permission model as
    /// Unix-like systems. It is determined by file extension and ACLs (Access Control
    /// Lists).
    #[cfg(target_os = "windows")]
    pub fn assert_permissions(_permissions: &std::fs::Permissions, _mode: u32) {}

    #[cfg(not(target_os = "windows"))]
    /// # Panics
    ///
    /// Panics if the permissions do not match the expected mode.
    pub fn assert_permissions(permissions: &std::fs::Permissions, expected: u32) {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(permissions.mode() & 0o777, expected);
    }

    #[test]
    fn test_set_file_executable() {
        // Create the root temp dir.
        let root = try_create_temp_dir().expect("conversion error");

        let new_dir = root.join("test_set_file_executable");
        fs::create_dir_all(&new_dir).expect("conversion error");

        let new_file = new_dir.join("test_set_file_executable.sh");
        fs::write(&new_file, "echo 'Hello, World!'").expect("conversion error");

        try_set_file_executable(&new_file).expect("conversion error");

        let metadata = fs::metadata(&new_file).expect("conversion error");
        let lhs = metadata.permissions();

        // Assert that the file has executable permission for owner, group, and others:
        // - The bitwise AND operation (lhs.mode() & 0o777) ensures that only the
        //   permission bits are compared, ignoring other bits that might be present in
        //   the mode.
        // - The assertion checks if the permission bits match 0o755.
        assert_permissions(&lhs, 0o755);
    }

    #[test]
    fn test_set_file_executable_on_non_file() {
        // Create the root temp dir.
        let root = try_create_temp_dir().expect("conversion error");

        let new_dir = root.join("test_set_file_executable_on_non_file");
        fs::create_dir_all(&new_dir).expect("conversion error");

        let result = try_set_file_executable(&new_dir);
        assert!(matches!(
            result,
            Err(FilePermissionsError::NotARegularFile { .. })
        ));
    }

    #[test]
    fn test_set_file_executable_on_non_existent_file() {
        // Create the root temp dir.
        let root = try_create_temp_dir().expect("conversion error");

        let new_dir = root.join("test_set_file_executable_on_non_existent_file");
        fs::create_dir_all(&new_dir).expect("conversion error");

        let non_existent_file = new_dir.join("non_existent_file.sh");
        let result = try_set_file_executable(&non_existent_file);
        assert!(matches!(
            result,
            Err(FilePermissionsError::MetadataReadFailed { .. })
        ));
    }
}

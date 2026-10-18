// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{UpgradeError, archive};
use flate2::read::GzDecoder;
use r3bl_tui::try_set_file_executable;
use std::{env::current_exe,
          fs::{self, File},
          path::{Path, PathBuf}};

// -------------------------------------------------------------------------------------------------
// Top-level Entry Points (Runtime Call-Site Adapters)
// -------------------------------------------------------------------------------------------------

/// Extracts all binaries from `archive_path` into `extract_dir` and replaces executables.
///
/// # Errors
///
/// Returns [`UpgradeError`] if archive extraction or binary swapping fails.
pub fn extract_and_apply_archive(
    archive_path: &Path,
    extract_dir: &Path,
) -> Result<(), UpgradeError> {
    fs::create_dir_all(extract_dir).map_err(|e| UpgradeError::Other(e.to_string()))?;
    let extracted_files =
        archive_extraction::extract_archive_files(archive_path, extract_dir)?;
    if extracted_files.is_empty() {
        return Err(UpgradeError::Other(
            "Downloaded archive contained no files".to_string(),
        ));
    }
    binary_replacement::apply_extracted_binaries(&extracted_files)
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// Archive unpacking and file metadata filtering.
pub mod archive_extraction {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Unpacks a `.tar.gz` archive into `extract_dir` and returns the list of extracted
    /// regular file paths.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Other`] if decompression or extraction fails.
    pub fn extract_archive_files(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<Vec<PathBuf>, UpgradeError> {
        let tar_gz =
            File::open(archive_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
        let tar_decoder = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar_decoder);

        let mut extracted_files = Vec::new();
        let entries = archive
            .entries()
            .map_err(|e| UpgradeError::Other(e.to_string()))?;

        for entry_res in entries {
            let mut entry = entry_res.map_err(|e| UpgradeError::Other(e.to_string()))?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let entry_path = entry
                .path()
                .map_err(|e| UpgradeError::Other(e.to_string()))?;
            let Some(base_name) = entry_path.file_name() else {
                continue;
            };
            let dest_path = extract_dir.join(base_name);
            entry
                .unpack(&dest_path)
                .map_err(|e| UpgradeError::Other(e.to_string()))?;
            try_set_file_executable(&dest_path)
                .map_err(|e| UpgradeError::Other(e.to_string()))?;
            extracted_files.push(dest_path);
        }

        Ok(extracted_files)
    }

    /// Checks whether a file has a non-binary metadata extension (e.g. `.sha256`, `.md`).
    #[must_use]
    pub fn is_non_binary_metadata_file(path: &Path) -> bool {
        path.extension().is_some_and(|ext| {
            archive::NON_BINARY_EXTENSIONS
                .iter()
                .any(|&ignored| ext.eq_ignore_ascii_case(ignored))
        })
    }
}

/// In-place executable replacement and sibling executable updates.
pub mod binary_replacement {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Extracts the [`UTF-8`] filename (`&str`) from a [`Path`].
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn path_file_name_str(path: &Path) -> Option<&str> { path.file_name()?.to_str() }

    /// Extracts the [`UTF-8`] file stem (`&str`) from a [`Path`].
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn path_file_stem_str(path: &Path) -> Option<&str> { path.file_stem()?.to_str() }

    /// Replaces the currently running executable via [`self_replace::self_replace`] and
    /// updates any sibling binaries included in the extracted archive (e.g. when `giti`
    /// upgrades `r3bl-cmdr`, it also updates `edi` and `env-source` in the same
    /// directory).
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Other`] if the current executable path cannot be resolved
    /// or if `self_replace` fails.
    pub fn apply_extracted_binaries(
        extracted_files: &[PathBuf],
    ) -> Result<(), UpgradeError> {
        let current_exe_path =
            current_exe().map_err(|e| UpgradeError::Other(e.to_string()))?;
        let current_exe_name =
            path_file_name_str(&current_exe_path).ok_or_else(|| {
                UpgradeError::Other("Could not determine current binary name".to_string())
            })?;
        let install_dir = current_exe_path.parent().ok_or_else(|| {
            UpgradeError::Other("Could not determine binary parent directory".to_string())
        })?;

        let mut replaced_self = false;

        for extracted_file in extracted_files {
            let Some(file_name) = path_file_name_str(extracted_file) else {
                continue;
            };

            if archive_extraction::is_non_binary_metadata_file(extracted_file) {
                continue;
            }

            let current_stem =
                path_file_stem_str(&current_exe_path).unwrap_or(current_exe_name);
            let extracted_stem = path_file_stem_str(extracted_file).unwrap_or(file_name);

            if file_name.eq_ignore_ascii_case(current_exe_name)
                || extracted_stem.eq_ignore_ascii_case(current_stem)
            {
                self_replace::self_replace(extracted_file).map_err(|e| {
                    UpgradeError::Other(format!("self_replace failed: {e}"))
                })?;
                replaced_self = true;
            } else {
                let sibling_dest = install_dir.join(file_name);
                let sibling_tmp = install_dir
                    .join(format!(".{file_name}.{}", archive::UPGRADE_TMP_SUFFIX));
                if fs::copy(extracted_file, &sibling_tmp).is_ok() {
                    drop(try_set_file_executable(&sibling_tmp));
                    if fs::rename(&sibling_tmp, &sibling_dest).is_err() {
                        drop(fs::copy(&sibling_tmp, &sibling_dest));
                        drop(fs::remove_file(&sibling_tmp));
                    }
                }
            }
        }

        if !replaced_self && extracted_files.len() == 1 {
            self_replace::self_replace(&extracted_files[0])
                .map_err(|e| UpgradeError::Other(format!("self_replace failed: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use flate2::{Compression, write::GzEncoder};

    #[test]
    fn test_extract_tar_gz_archive() {
        let temp_dir = try_create_temp_dir().unwrap();
        let archive_path = temp_dir.join("test_suite.tar.gz");

        {
            let tar_gz_file = File::create(&archive_path).unwrap();
            let enc = GzEncoder::new(tar_gz_file, Compression::default());
            let mut tar_builder = tar::Builder::new(enc);

            let data = b"#!/bin/sh\necho hello\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(u64::try_from(data.len()).unwrap());
            header.set_mode(0o755);
            header.set_cksum();
            tar_builder
                .append_data(&mut header, "giti", &data[..])
                .unwrap();
            tar_builder.finish().unwrap();
        }

        let extract_dir = temp_dir.join("out");
        fs::create_dir_all(&extract_dir).unwrap();
        let extracted =
            archive_extraction::extract_archive_files(&archive_path, &extract_dir)
                .unwrap();
        assert_eq2!(extracted.len(), 1);
        assert!(extracted[0].ends_with("giti"));
        assert_eq2!(fs::read(&extracted[0]).unwrap(), b"#!/bin/sh\necho hello\n");
    }

    #[test]
    fn test_is_non_binary_metadata_file() {
        assert!(archive_extraction::is_non_binary_metadata_file(Path::new(
            "release.sha256"
        )));
        assert!(archive_extraction::is_non_binary_metadata_file(Path::new(
            "README.md"
        )));
        assert!(!archive_extraction::is_non_binary_metadata_file(Path::new(
            "r3bl-cmdr"
        )));
        assert!(!archive_extraction::is_non_binary_metadata_file(Path::new(
            "giti.exe"
        )));
    }

    #[test]
    fn test_path_file_name_and_stem() {
        let p = Path::new("/usr/local/bin/giti");
        assert_eq2!(binary_replacement::path_file_name_str(p), Some("giti"));
        assert_eq2!(binary_replacement::path_file_stem_str(p), Some("giti"));

        let p_with_ext = Path::new("giti.exe");
        assert_eq2!(
            binary_replacement::path_file_name_str(p_with_ext),
            Some("giti.exe")
        );
        assert_eq2!(
            binary_replacement::path_file_stem_str(p_with_ext),
            Some("giti")
        );
    }

    #[test]
    fn test_extract_and_apply_archive_empty_error() {
        let temp_dir = try_create_temp_dir().unwrap();
        let archive_path = temp_dir.join("empty.tar.gz");
        {
            let tar_gz_file = File::create(&archive_path).unwrap();
            let enc = GzEncoder::new(tar_gz_file, Compression::default());
            let mut tar_builder = tar::Builder::new(enc);
            tar_builder.finish().unwrap();
        }

        let extract_dir = temp_dir.join("out");
        let result = extract_and_apply_archive(&archive_path, &extract_dir);
        assert!(matches!(
            result,
            Err(UpgradeError::Other(msg)) if msg.contains("contained no files")
        ));
    }
}

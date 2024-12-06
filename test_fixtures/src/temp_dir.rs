/*
 *   Copyright (c) 2024 R3BL LLC
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

use miette::IntoDiagnostic;
use r3bl_core::friendly_random_id;

pub struct TempDir {
    pub path: std::path::PathBuf,
}

/// Create a temporary directory. The directory is automatically deleted when the
/// [TempDir] struct is dropped.
pub fn create_temp_dir() -> miette::Result<TempDir> {
    let root = std::env::temp_dir();
    let new_temp_dir = root.join(friendly_random_id::generate_friendly_random_id());
    std::fs::create_dir(&new_temp_dir).into_diagnostic()?;
    Ok(TempDir { path: new_temp_dir })
}

impl Drop for TempDir {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.path).unwrap(); }
}

#[cfg(test)]
mod tests {
    use crossterm::style::Stylize as _;

    use super::*;

    #[test]
    fn test_temp_dir() {
        let temp_dir = create_temp_dir().unwrap();
        println!(
            "Temp dir: {}",
            temp_dir.path.display().to_string().magenta()
        );

        assert!(temp_dir.path.exists());
    }

    #[test]
    fn test_temp_dir_drop() {
        let temp_dir = create_temp_dir().unwrap();

        let copy_of_path = temp_dir.path.clone();
        println!("Temp dir: {}", copy_of_path.display().to_string().magenta());

        drop(temp_dir);

        assert!(!copy_of_path.exists());
    }
}

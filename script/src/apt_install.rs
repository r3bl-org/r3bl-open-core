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

use miette::IntoDiagnostic;
use r3bl_core::ok;

use crate::command;

/// Here are some examples of using `dpkg-query` to check if a package is installed:
///
/// ```fish
/// set package_name "openssl"
/// dpkg-query -s $package_name
/// echo $status
/// if test $status -eq 0
///     echo "True if package is installed"
/// else
///     echo "False if package is not installed"
/// end
/// ```
///
/// # Example
///
/// ```no_run
/// use r3bl_script::apt_install::check_if_package_is_installed;
///
/// async fn check() {
///     let package_name = "bash";
///     let is_installed = check_if_package_is_installed(package_name).await.unwrap();
///     assert!(is_installed);
/// }
/// ```
///
/// ```no_run
/// use r3bl_script::apt_install::install_package;
///
/// async fn install() {
///     let package_name = "does_not_exist";
///     assert!(install_package(package_name).await.is_err());
/// }
/// ```
pub async fn check_if_package_is_installed(package_name: &str) -> miette::Result<bool> {
    let output = command!(
        program => "dpkg-query",
        args => "-s", package_name
    )
    .output()
    .await
    .into_diagnostic()?;
    ok!(output.status.success())
}

pub async fn install_package(package_name: &str) -> miette::Result<()> {
    let command = command!(
        program => "sudo",
        args => "apt", "install", "-y", package_name
    )
    .output()
    .await
    .into_diagnostic()?;
    if command.status.success() {
        ok!()
    } else {
        miette::bail!(
            "Failed to install package: {:?} with sudo apt",
            String::from_utf8_lossy(&command.stderr)
        );
    }
}

#[cfg(test)]
mod tests_apt_install {
    use r3bl_core::{TTYResult, is_fully_uninteractive_terminal};

    use super::*;

    #[tokio::test]
    async fn test_check_if_package_is_installed() {
        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }
        let package_name = "bash";
        let is_installed = check_if_package_is_installed(package_name).await.unwrap();
        assert!(is_installed);
    }

    #[tokio::test]
    async fn test_install_package() {
        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }
        let package_name = "does_not_exist";
        assert!(install_package(package_name).await.is_err());
    }
}

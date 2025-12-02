// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{command, ok};
use miette::IntoDiagnostic;

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
/// use r3bl_tui::check_if_package_is_installed;
///
/// async fn check() {
///     let package_name = "bash";
///     let is_installed = check_if_package_is_installed(package_name).await.unwrap();
///     assert!(is_installed);
/// }
/// ```
///
/// ```no_run
/// use r3bl_tui::install_package;
///
/// async fn install() {
///     let package_name = "does_not_exist";
///     assert!(install_package(package_name).await.is_err());
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The `dpkg-query` command is not available (not on a Debian-based system)
/// - The command fails to execute
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

/// # Errors
///
/// Returns an error if:
/// - The `apt` command is not available (not on a Debian-based system)
/// - The user does not have sudo privileges
/// - The package installation fails
/// - Network issues prevent downloading the package
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
    use super::*;
    use crate::{TTYResult, is_output_interactive};

    #[tokio::test]
    async fn test_check_if_package_is_installed() {
        if let TTYResult::IsNotInteractive = is_output_interactive() {
            return;
        }

        let package_name = "bash";
        let is_installed = check_if_package_is_installed(package_name).await.unwrap();
        assert!(is_installed);
    }

    #[tokio::test]
    async fn test_install_package() {
        if let TTYResult::IsNotInteractive = is_output_interactive() {
            return;
        }

        let package_name = "does_not_exist";
        assert!(install_package(package_name).await.is_err());
    }
}

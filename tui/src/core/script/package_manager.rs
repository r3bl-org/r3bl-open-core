// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{command, ok};
use miette::IntoDiagnostic;

/// Supported package manager types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    /// Debian/Ubuntu: apt, dpkg-query
    Apt,
    /// Fedora/RHEL/CentOS: dnf, rpm
    Dnf,
    /// Arch Linux: pacman
    Pacman,
    /// openSUSE: zypper, rpm
    Zypper,
    /// macOS: brew
    Brew,
}

impl PackageManager {
    /// Detect the system's package manager by checking for available commands.
    #[must_use]
    pub fn detect() -> Option<Self> {
        // Check in order of specificity
        if std::process::Command::new("apt-get")
            .arg("--version")
            .output()
            .is_ok()
        {
            Some(PackageManager::Apt)
        } else if std::process::Command::new("dnf")
            .arg("--version")
            .output()
            .is_ok()
        {
            Some(PackageManager::Dnf)
        } else if std::process::Command::new("pacman")
            .arg("-V")
            .output()
            .is_ok()
        {
            Some(PackageManager::Pacman)
        } else if std::process::Command::new("zypper")
            .arg("--version")
            .output()
            .is_ok()
        {
            Some(PackageManager::Zypper)
        } else if std::process::Command::new("brew")
            .arg("--version")
            .output()
            .is_ok()
        {
            Some(PackageManager::Brew)
        } else {
            None
        }
    }

    /// Get the command used to check if a package is installed.
    #[must_use]
    pub fn check_command(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            PackageManager::Apt => ("dpkg-query", &["-s"]),
            PackageManager::Dnf | PackageManager::Zypper => ("rpm", &["-q"]),
            PackageManager::Pacman => ("pacman", &["-Q"]),
            PackageManager::Brew => ("brew", &["list"]),
        }
    }

    /// Get the command used to install a package.
    #[must_use]
    pub fn install_command(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            PackageManager::Apt => ("apt", &["install", "-y"]),
            PackageManager::Dnf => ("dnf", &["install", "-y"]),
            PackageManager::Pacman => ("pacman", &["-S", "--noconfirm"]),
            PackageManager::Zypper => ("zypper", &["install", "-y"]),
            PackageManager::Brew => ("brew", &["install"]),
        }
    }

    /// Whether this package manager requires sudo for installation.
    #[must_use]
    pub fn requires_sudo(&self) -> bool { !matches!(self, PackageManager::Brew) }
}

/// Check if a command is available on the system PATH.
///
/// Uses `which` to check if the given command name resolves to an
/// executable. This works on all Unix-like systems (Linux and macOS)
/// regardless of package manager.
#[must_use]
pub fn is_command_available(command_name: &str) -> bool {
    std::process::Command::new("which")
        .arg(command_name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a package is installed on the system.
///
/// This function detects the system's package manager and uses the appropriate
/// command to check package installation status.
///
/// # Supported Package Managers
///
/// - **Debian/Ubuntu (apt)**: Uses `dpkg-query -s <package>`
/// - **Fedora/RHEL (dnf)**: Uses `rpm -q <package>`
/// - **Arch (pacman)**: Uses `pacman -Q <package>`
/// - **openSUSE (zypper)**: Uses `rpm -q <package>`
/// - **macOS (brew)**: Uses `brew list <package>`
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
/// # Errors
///
/// Returns an error if:
/// - No supported package manager is detected
/// - The command fails to execute
pub async fn check_if_package_is_installed(package_name: &str) -> miette::Result<bool> {
    // Fast path: check if command is available on PATH.
    // This handles macOS system binaries (e.g., /bin/bash) that aren't
    // managed by brew, and provides a quick answer on all platforms.
    if is_command_available(package_name) {
        return ok!(true);
    }

    // Slow path: ask the package manager (handles library packages and
    // packages whose binary name differs from the package name).
    let pkg_mgr = PackageManager::detect()
        .ok_or_else(|| miette::miette!("No supported package manager found"))?;

    let (cmd, base_args) = pkg_mgr.check_command();

    let output = command!(
        program => cmd,
        args => base_args[0], package_name
    )
    .output()
    .await
    .into_diagnostic()?;

    ok!(output.status.success())
}

/// Install a package using the system's package manager.
///
/// This function detects the system's package manager and uses the appropriate
/// command to install the specified package.
///
/// # Supported Package Managers
///
/// - **Debian/Ubuntu (apt)**: Uses `sudo apt install -y <package>`
/// - **Fedora/RHEL (dnf)**: Uses `sudo dnf install -y <package>`
/// - **Arch (pacman)**: Uses `sudo pacman -S --noconfirm <package>`
/// - **openSUSE (zypper)**: Uses `sudo zypper install -y <package>`
/// - **macOS (brew)**: Uses `brew install <package>` (no sudo)
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::install_package;
///
/// async fn install() {
///     let package_name = "tree";
///     install_package(package_name).await.unwrap();
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - No supported package manager is detected
/// - The user does not have sudo privileges (for package managers that require it)
/// - The package installation fails
/// - Network issues prevent downloading the package
pub async fn install_package(package_name: &str) -> miette::Result<()> {
    let pkg_mgr = PackageManager::detect()
        .ok_or_else(|| miette::miette!("No supported package manager found"))?;

    let (cmd, base_args) = pkg_mgr.install_command();

    let command_result = if pkg_mgr.requires_sudo() {
        // Build args: ["apt", "install", "-y", package_name]
        let mut args = vec![cmd];
        args.extend(base_args.iter().copied());
        args.push(package_name);

        command!(
            program => "sudo",
            args => args[0], args[1], args[2], args[3]
        )
        .output()
        .await
        .into_diagnostic()?
    } else {
        // For brew, no sudo needed
        let mut args: Vec<&str> = base_args.to_vec();
        args.push(package_name);

        command!(
            program => cmd,
            args => args[0], args[1]
        )
        .output()
        .await
        .into_diagnostic()?
    };

    if command_result.status.success() {
        ok!()
    } else {
        miette::bail!(
            "Failed to install package '{}' with {}: {:?}",
            package_name,
            cmd,
            String::from_utf8_lossy(&command_result.stderr)
        );
    }
}

/// Get the detected package manager for the current system.
///
/// This is useful for informational purposes or when you need to
/// handle package manager-specific logic.
#[must_use]
pub fn get_package_manager() -> Option<PackageManager> { PackageManager::detect() }

#[cfg(test)]
mod tests_package_manager {
    use super::*;
    use crate::{TTYResult, is_output_interactive};

    #[test]
    fn test_package_manager_detection() {
        // This test will succeed on any supported system
        let pkg_mgr = PackageManager::detect();
        // On a typical development machine, we should find a package manager
        // But in CI/containers, it might not be available, so we don't assert
        if let Some(pm) = pkg_mgr {
            println!("Detected package manager: {pm:?}");
        } else {
            println!("No package manager detected (this is OK in some environments)");
        }
    }

    #[tokio::test]
    async fn test_check_if_package_is_installed() {
        if let TTYResult::IsNotInteractive = is_output_interactive() {
            return;
        }

        // bash should be installed on any Unix-like system
        let package_name = "bash";
        let result = check_if_package_is_installed(package_name).await;

        // Only check if we have a package manager available
        if let Ok(is_installed) = result {
            assert!(is_installed, "bash should be installed");
        }
    }

    #[tokio::test]
    async fn test_install_nonexistent_package() {
        if let TTYResult::IsNotInteractive = is_output_interactive() {
            return;
        }

        let package_name = "this_package_definitely_does_not_exist_12345";
        let result = install_package(package_name).await;

        // This should fail because the package doesn't exist
        assert!(result.is_err());
    }
}

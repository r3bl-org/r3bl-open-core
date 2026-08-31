// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words noconfirm

use crate::{CommandOutputResult, command, ok};
use miette::{Context, IntoDiagnostic};

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
    /// Detects the system's package manager by checking for available commands.
    #[must_use]
    pub fn detect() -> Option<Self> {
        // Check in order of specificity
        let cmd_output_result = std::process::Command::new("apt-get")
            .arg("--version")
            .output();
        if matches!(
            CommandOutputResult::from(cmd_output_result),
            CommandOutputResult::Success(_)
        ) {
            return Some(PackageManager::Apt);
        }

        let cmd_output_result =
            std::process::Command::new("dnf").arg("--version").output();
        if matches!(
            CommandOutputResult::from(cmd_output_result),
            CommandOutputResult::Success(_)
        ) {
            return Some(PackageManager::Dnf);
        }

        let cmd_output_result = std::process::Command::new("pacman").arg("-V").output();
        if matches!(
            CommandOutputResult::from(cmd_output_result),
            CommandOutputResult::Success(_)
        ) {
            return Some(PackageManager::Pacman);
        }

        let cmd_output_result = std::process::Command::new("zypper")
            .arg("--version")
            .output();
        if matches!(
            CommandOutputResult::from(cmd_output_result),
            CommandOutputResult::Success(_)
        ) {
            return Some(PackageManager::Zypper);
        }

        let cmd_output_result =
            std::process::Command::new("brew").arg("--version").output();
        if matches!(
            CommandOutputResult::from(cmd_output_result),
            CommandOutputResult::Success(_)
        ) {
            return Some(PackageManager::Brew);
        }

        None
    }

    /// Gets the command used to check if a package is installed.
    #[must_use]
    pub fn check_command(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            PackageManager::Apt => ("dpkg-query", &["-s"]),
            PackageManager::Dnf | PackageManager::Zypper => ("rpm", &["-q"]),
            PackageManager::Pacman => ("pacman", &["-Q"]),
            PackageManager::Brew => ("brew", &["list"]),
        }
    }

    /// Gets the command used to install a package.
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

/// Checks if a command is available on the system PATH.
///
/// Uses `which` to check if the given command name resolves to an
/// executable. This works on all Unix-like systems (Linux and macOS)
/// regardless of package manager.
#[must_use]
pub fn is_command_available(command_name: &str) -> bool {
    let cmd_output_result = std::process::Command::new("which")
        .arg(command_name)
        .output();
    matches!(
        CommandOutputResult::from(cmd_output_result),
        CommandOutputResult::Success(_)
    )
}

/// Checks if a package is installed on the system.
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
///     let is_installed = check_if_package_is_installed(package_name).await.expect("conversion error");
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

    let cmd_output_result = command!(
        program => cmd,
        args => base_args[0], package_name
    )
    .output()
    .await;

    match CommandOutputResult::from(cmd_output_result) {
        CommandOutputResult::Success(_) => ok!(true),
        CommandOutputResult::NonZeroExit(_) => ok!(false),
        CommandOutputResult::SpawnFailed(err) => Err(err).into_diagnostic(),
    }
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
///     install_package(package_name).await.expect("conversion error");
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

    let cmd_output_result = if pkg_mgr.requires_sudo() {
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
    };

    match CommandOutputResult::from(cmd_output_result) {
        CommandOutputResult::Success(_) => ok!(),
        CommandOutputResult::NonZeroExit(output) => Err(miette::miette!(
            "Failed to install package '{}' with {}: {:?}",
            package_name,
            cmd,
            String::from_utf8_lossy(&output.stderr)
        )),
        CommandOutputResult::SpawnFailed(err) => {
            Err(err).into_diagnostic().wrap_err_with(|| {
                format!(
                    "Failed to spawn installation command for package '{package_name}'"
                )
            })
        }
    }
}

/// Gets the detected package manager for the current system.
///
/// This is useful for informational purposes or when you need to
/// handle package manager-specific logic.
#[must_use]
pub fn get_package_manager() -> Option<PackageManager> { PackageManager::detect() }

#[cfg(test)]
mod tests_package_manager {
    use super::*;

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
        let package_name = "this_package_definitely_does_not_exist_12345";
        let result = install_package(package_name).await;

        // This should fail because the package doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_is_command_available() {
        #[cfg(unix)]
        {
            assert!(is_command_available("sh"));
            assert!(!is_command_available(
                "definitely_nonexistent_binary_xyz_123"
            ));
        }
    }
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::ItemsOwned;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchDeleteDetails {
    pub maybe_deleted_branches: Option<ItemsOwned>,
}

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchNewDetails {
    pub maybe_created_branch: Option<String>,
}

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchCheckoutDetails {
    pub maybe_checked_out_branch: Option<String>,
}

/// Information about command and subcommand that has run successfully. Eg: `giti branch
/// delete` or `giti branch checkout` or `giti branch new`.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CommandRunDetails {
    BranchDelete(BranchDeleteDetails),
    BranchNew(BranchNewDetails),
    BranchCheckout(BranchCheckoutDetails),
    Commit,
    Remote,
    Noop,
}

impl Display for CommandRunDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            CommandRunDetails::BranchDelete(details) => {
                write!(
                    f,
                    " BranchDelete: {A:?}",
                    A = details.maybe_deleted_branches
                )
            }
            CommandRunDetails::BranchNew(details) => {
                write!(f, " BranchNew: {A:?}", A = details.maybe_created_branch)
            }
            CommandRunDetails::BranchCheckout(details) => {
                write!(
                    f,
                    " BranchCheckout: {A:?}",
                    A = details.maybe_checked_out_branch
                )
            }
            Self::Commit => write!(f, " Commit"),
            Self::Remote => write!(f, " Remote"),
            Self::Noop => write!(f, " Noop"),
        }
    }
}

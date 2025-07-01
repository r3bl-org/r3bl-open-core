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

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use r3bl_tui::ItemsOwned;

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

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

use std::process::{Command, Output};

use r3bl_core::ItemsOwned;

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

/// A command is something that is run by `giti` in the underlying OS. This is meant to
/// hold all the possible outcomes of executing a [std::process::Command].
#[derive(Debug)]
pub enum CommandRunResult {
    /// Command was not run (probably because the command would be a no-op).
    DidNotRun(
        /* message */ String,
        /* command specific details */ CommandRunDetails,
    ),

    /// Command ran, and produced success exit code.
    RanSuccessfully(
        /* success message */ String,
        /* command specific details */ CommandRunDetails,
    ),

    /// Command ran, and produced non-zero exit code.
    RanUnsuccessfully(
        /* error message */ String,
        /* command */ Command,
        /* stdout or stderr */ Output,
    ),

    /// Attempt to run the command failed. It never ran.
    FailedToRun(
        /* error message */ String,
        /* command */ Command,
        /* error report */ miette::Report,
    ),
}

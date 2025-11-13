// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InlineString, ItemsOwned};
use crate::script::git::types::constants::git_ui_strings::CURRENT_BRANCH_PREFIX;

/// Information about local git branches:
/// - The currently checked out branch.
/// - List of other local branches (excluding the current one).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalBranchInfo {
    pub current_branch: InlineString,
    pub other_branches: ItemsOwned,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BranchExists {
    Yes,
    No,
}

impl LocalBranchInfo {
    #[must_use]
    pub fn exists_locally(&self, branch_name: &str) -> BranchExists {
        if branch_name == self.current_branch.as_str()
            || self.other_branches.iter().any(|b| b == branch_name)
        {
            BranchExists::Yes
        } else {
            BranchExists::No
        }
    }

    /// ### Input
    /// ```text
    /// "main"
    /// ```
    ///
    /// ### Output
    /// ```text
    /// "(◕‿◕) main"
    /// ```
    #[must_use]
    pub fn mark_branch_current(branch_name: &str) -> InlineString {
        use std::fmt::Write;
        let mut acc = InlineString::new();
        // We don't care about the result of this operation.
        write!(acc, "{} {}", CURRENT_BRANCH_PREFIX, branch_name).ok();
        acc
    }

    /// ### Input
    /// ```text
    /// "(◕‿◕) main"
    /// ```
    ///
    /// ### Output
    /// ```text
    /// "main"
    /// ```
    #[must_use]
    pub fn trim_current_prefix_from_branch(branch: &str) -> &str {
        branch.trim_start_matches(CURRENT_BRANCH_PREFIX).trim()
    }
}

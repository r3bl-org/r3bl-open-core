// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::CommonResult;
use tokio::process::Command;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoStatus {
    Dirty,
    Clean,
}

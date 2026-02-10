// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// UI-related string constants used for display purposes.
/// Prefix used to mark the currently checked-out branch in branch listings.
pub mod git_ui_strings {
    pub const CURRENT_BRANCH_PREFIX: &str = "(◕‿◕)";
}

/// Git program name and command names used across the module.
pub mod git_command_names {
    pub const GIT_PROGRAM: &str = "git";
    pub const GIT_CMD_BRANCH: &str = "branch";
    pub const GIT_CMD_CHECKOUT: &str = "checkout";
    pub const GIT_CMD_STATUS: &str = "status";
    pub const GIT_CMD_ADD: &str = "add";
    pub const GIT_CMD_COMMIT: &str = "commit";
    pub const GIT_CMD_INIT: &str = "init";
    pub const GIT_CMD_CONFIG: &str = "config";
    pub const GIT_CMD_DIFF: &str = "diff";
    pub const GIT_CMD_DIFF_TREE: &str = "diff-tree";
    pub const GIT_CMD_REV_PARSE: &str = "rev-parse";
}

/// Git command flags and arguments used across the module.
pub mod git_command_args {
    pub const GIT_ARG_SHOW_CURRENT: &str = "--show-current";
    pub const GIT_ARG_PORCELAIN: &str = "--porcelain";
    pub const GIT_ARG_GIT_DIR: &str = "--git-dir";
    pub const GIT_ARG_FORMAT: &str = "--format";
    pub const GIT_ARG_REFNAME_SHORT: &str = "%(refname:short)";
    pub const GIT_ARG_CREATE_BRANCH: &str = "-b";
    pub const GIT_ARG_INIT_BRANCH: &str = "-b";
    pub const GIT_ARG_DELETE_FORCE: &str = "-D";
    pub const GIT_ARG_NAME_ONLY: &str = "--name-only";
    pub const GIT_ARG_HEAD: &str = "HEAD";
    pub const GIT_ARG_NO_COMMIT_ID: &str = "--no-commit-id";
    pub const GIT_ARG_RECURSIVE: &str = "-r";
}

/// Git configuration keys used across the module.
pub mod git_config_keys {
    pub const GIT_CONFIG_USER_EMAIL: &str = "user.email";
    pub const GIT_CONFIG_USER_NAME: &str = "user.name";
    pub const GIT_CONFIG_COMMIT_GPGSIGN: &str = "commit.gpgsign";
    pub const GIT_CONFIG_INIT_DEFAULT_BRANCH: &str = "init.defaultBranch";
    pub const GIT_CONFIG_FLAG_LOCAL: &str = "--local";
}

/// Test fixture configuration constants used across test modules.
pub mod test_config {
    pub const TEST_EMAIL: &str = "test@example.com";
    pub const TEST_USER_NAME: &str = "Test User";
    pub const TEST_GPG_SIGN_DISABLED: &str = "false";
    pub const TEST_DEFAULT_BRANCH: &str = "main";
    pub const TEST_INITIAL_COMMIT_MSG: &str = "Initial commit";
    pub const TEST_ENV_ISOLATED_TEST_RUNNER: &str = "ISOLATED_TEST_RUNNER";
}

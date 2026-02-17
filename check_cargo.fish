# Pure Cargo Command Wrappers
#
# Level 1 functions that just run the cargo command and return status code.
# They do NOT handle output formatting or ICE detection.
#
# All cargo commands are wrapped with ionice_wrapper (see script_lib.fish) which applies:
#   nice -n 10:    Lower CPU priority so interactive processes (terminal, IDE) win scheduling.
#   ionice -c2 -n0: Highest I/O priority within best-effort class (no sudo needed).
# This keeps the terminal responsive during long builds (especially --watch-doc full builds).

function check_cargo_check
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo check
end

function check_cargo_build
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo build
end

function check_clippy
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo clippy --all-targets
end

function check_cargo_test
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo test --all-targets -q
end

function check_doctests
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo test --doc -q
end

function check_windows_build
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata
end

# Quick doc check without dependencies (for one-off --doc mode).
# Builds to QUICK staging directory to avoid race conditions with background full builds.
function check_docs_quick
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo doc --no-deps
end

# One-off doc check for normal mode (./check.fish without flags).
# Builds directly to CHECK_TARGET_DIR to avoid conflicts with --watch-doc's staging dirs.
# Uses --no-deps for speed since this is just a verification step.
#
# Key insight: Normal one-off mode and --watch-doc can run simultaneously because they
# use different target directories:
#   - One-off: CHECK_TARGET_DIR (/tmp/roc/target/check)
#   - Watch-doc: staging dirs (/tmp/roc/target/check-doc-staging-*)
#
# Trade-off: During build, the doc folder is temporarily empty (cargo clears it first).
# This is acceptable for one-off mode since users typically wait for completion before
# refreshing the browser.
function check_docs_oneoff
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo doc --no-deps
end

# Full doc build including dependencies (for watch modes).
# Builds to FULL staging directory to avoid race conditions with quick builds.
function check_docs_full
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
    ionice_wrapper timeout $CHECK_TIMEOUT_SECS cargo doc
end

# Formats rustdoc comments on git-changed files.
# With no arguments, cargo rustdoc-fmt automatically targets staged/unstaged changes.
# Also runs cargo fmt on any files it modifies.
function run_rustdoc_fmt
    cargo rustdoc-fmt
end

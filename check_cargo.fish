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
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo check
end

function check_cargo_build
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo build
end

function check_clippy
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo clippy --all-targets
end

function check_cargo_test
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo test --all-targets -q
end

function check_doctests
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo test --doc -q
end

function check_windows_build
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata
end

# Quick doc check without dependencies (for --quick-doc and normal mode).
# Builds to QUICK staging directory; callers rsync to serving dir after success.
function check_docs_quick
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK
    run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS --no-deps
end

# Full doc build with dep-doc caching (for --doc, --full, and watch modes).
# Builds to FULL staging directory to avoid race conditions with quick builds.
#
# Dep-doc caching: If Cargo.lock + rust-toolchain.toml haven't changed since
# the last full build, skips dependency docs (--no-deps) for ~10x speedup.
# The hash is stored in the FULL staging directory, making it resilient to
# serving directory wipes by check_config_changed.
#
# Sets DEP_DOCS_WERE_CACHED global so callers choose the correct sync mode.
function check_docs_full
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
    if dep_docs_are_current $CHECK_TARGET_DIR_DOC_STAGING_FULL
        set -g DEP_DOCS_WERE_CACHED true
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS --no-deps
    else
        set -g DEP_DOCS_WERE_CACHED false
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS
    end
end

# Checks external URLs in git-modified files for link rot.
# Scoped to staged + unstaged changes only (not the whole repo).
# Requires lychee (installed via run.fish install-cargo-tools).
# Config: lychee.toml (repo root) defines exclusions and timeouts.
# Returns 0 if no broken links, 1 if broken links found.
function check_lychee_changed_files
    if not command -v lychee >/dev/null
        echo "lychee not installed (run: fish run.fish install-cargo-tools)"
        return 1
    end

    # Get git-modified files (staged + unstaged vs HEAD).
    set -l changed_files (git diff --name-only HEAD 2>/dev/null)
    if test $status -ne 0
        # No HEAD yet (initial commit).
        set changed_files (git diff --name-only 2>/dev/null)
    end
    # Deduplicate and filter empty strings.
    set changed_files (string match -v '' $changed_files | sort -u)

    if test (count $changed_files) -eq 0
        echo "No changed files to check."
        return 0
    end

    # Wall-clock timeout (seconds) to prevent lychee from blocking --full indefinitely.
    set -l lychee_timeout 120
    timeout $lychee_timeout lychee --no-progress $changed_files
    set -l lychee_status $status
    if test $lychee_status -eq 124
        echo "⚠️  lychee timed out after {$lychee_timeout}s — skipping link check"
        return 0
    end
    return $lychee_status
end

# Formats rustdoc comments on git-changed files.
# With no arguments, cargo rustdoc-fmt automatically targets staged/unstaged changes.
# Also runs cargo fmt on any files it modifies.
function run_rustdoc_fmt
    cargo rustdoc-fmt
end

# Pure Cargo Command Wrappers
#
# Level 1 functions that just run the cargo command and return status code.
# They do NOT handle output formatting or ICE detection.
#
# # Design & Algorithm
#
# These wrapper functions serve as the foundational execution layer for the build system.
# When rewriting these scripts in Rust, the following design constraints and algorithmic
# choices must be preserved:
#
# 1. Strict Priority Inversion & CPU Affinity (`ionice_wrapper`):
#    All cargo commands are wrapped with an `ionice_wrapper` (defined in script_lib.fish) which applies:
#      - `taskset -c <p_cpus>`: Hard CPU affinity to P-cores on Linux hybrid CPUs (prevents jobs from landing on slower E-cores).
#      - `nice -n 10`: Lower CPU priority so interactive processes (terminal, IDE) win scheduling.
#      - `ionice -c2 -n0`: Highest I/O priority within the best-effort class (no sudo needed).
#    This keeps the terminal responsive during heavy compilations and eliminates doc lock stalls on E-cores.
#
# 2. Aggressive Lint Enforcement (`-D warnings`):
#    The `check_clippy` function explicitly appends `-- -D warnings` to the cargo command.
#    This forces clippy to treat warnings as fatal errors (exit code != 0). This is a critical
#    architectural choice to ensure the CI/CD orchestrator (`check_orchestrators.fish`) halts
#    immediately on warnings, preventing them from being buried by subsequent test output or
#    being overlooked by LLM agents relying on exit codes.
#
# 3. Timeout Boundaries:
#    Every cargo process is bounded by `timeout --foreground $CHECK_TIMEOUT_SECS` to prevent
#    zombie processes or infinite hangs during macro expansion or compiler bugs.

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
    ionice_wrapper timeout --foreground $CHECK_TIMEOUT_SECS cargo clippy --all-targets -- -D warnings
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

# Re-generates the star history SVG chart using TypeScript script
function check_star_history
    if not command -v node >/dev/null; or not command -v npx >/dev/null
        echo "❌ Node.js / npx not found in PATH. Run ./bootstrap.sh to install Node.js."
        return 1
    end
    npx -y tsx .github/workflows/generate-star-history.ts
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
# Background Mutual Exclusion:
# Checks is_background_doc_build_running before running cargo doc. If an in-progress
# background --watch-doc build is active, awaits its completion via wait_for_background_doc_build
# instead of spawning a concurrent cargo doc command. This eliminates lock contention
# on /tmp/check-fish-roc/staging-full/doc/.lock.
#
# Sets DEP_DOCS_WERE_CACHED global so callers choose the correct sync mode.
#
# Rust migration: Query shared build daemon/state to await active doc compilation.
function check_docs_full
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL

    # If a full doc build is ALREADY running (in background watch-doc or another terminal),
    # wait for it to finish instead of spawning a second concurrent cargo doc process!
    if is_background_doc_build_running
        set_color yellow
        echo "    ⏳ Full doc build is already in progress. Waiting for completion..."
        set_color normal
        wait_for_background_doc_build
        set -g DEP_DOCS_WERE_CACHED true
        return 0
    end

    # Write current PID so concurrent manual --full or --doc runs coordinate cleanly
    echo %self > $CHECK_FULL_DOC_PID_FILE
    trap "command rm -f $CHECK_FULL_DOC_PID_FILE 2>/dev/null" EXIT INT TERM

    if dep_docs_are_current $CHECK_TARGET_DIR_DOC_STAGING_FULL
        set -g DEP_DOCS_WERE_CACHED true
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS --no-deps
    else
        set -g DEP_DOCS_WERE_CACHED false
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS
    end

    command rm -f $CHECK_FULL_DOC_PID_FILE 2>/dev/null
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
    # Deduplicate, filter empty strings, and remove deleted files (which crash lychee).
    set changed_files (string match -v '' $changed_files | sort -u)
    set -l existing_files
    for f in $changed_files
        if test -e $f
            set -a existing_files $f
        end
    end
    set changed_files $existing_files

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

# Formats git-changed Rust (.rs) files using both cargo fmt and cargo rustdoc-fmt.
# Collects staged, unstaged, and untracked .rs files.
function check_cargo_fmt_changed
    # Get git-modified files (staged + unstaged vs HEAD).
    set -l changed_files (git diff --name-only HEAD 2>/dev/null)
    if test $status -ne 0
        set changed_files (git diff --name-only 2>/dev/null)
    end
    # Include untracked files.
    set -l untracked_files (git ls-files --others --exclude-standard 2>/dev/null)
    set -a changed_files $untracked_files

    # Filter to deduplicated existing .rs files.
    set -l rs_files
    for f in (string match -v '' $changed_files | sort -u)
        if test -f $f; and string match -q "*.rs" $f
            set -a rs_files $f
        end
    end

    if test (count $rs_files) -eq 0
        echo "No changed Rust (.rs) files to format."
        return 0
    end

    echo "🎨 Running cargo fmt on changed files..."
    cargo fmt -- $rs_files
    set -l fmt_status $status

    echo "📚 Running cargo rustdoc-fmt on changed files..."
    cargo rustdoc-fmt $rs_files
    set -l rustdoc_fmt_status $status

    if test $fmt_status -ne 0; or test $rustdoc_fmt_status -ne 0
        return 1
    end
    return 0
end


# ICE & Stale Artifact Detection
#
# Detects Internal Compiler Errors (ICE) by checking for rustc dump files.
#
# When rustc crashes, it creates: rustc-ice-YYYY-MM-DDTHH_MM_SS-PID.txt
# File-based detection is reliable, but stale dump files from interrupted
# previous runs can cause false positives. To prevent this,
# cleanup_stale_ice_files() is called at startup to remove any leftover files.
#
# Note: Detection is only called when cargo commands fail (exit code != 0).
# If commands succeed, there's no ICE to detect.
#
# Recovery is handled by cleanup_for_recovery() in check_recovery.fish.
# Stale build artifacts are detected by detect_stale_build_artifacts() below.

# Removes stale rustc-ice-*.txt dump files left over from previous runs.
# Called at startup to prevent false ICE detection from interrupted sessions.
function cleanup_stale_ice_files
    set -l stale_ice_files (find . -maxdepth 1 -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $stale_ice_files) -gt 0
        log_message "🧹 Removing "(count $stale_ice_files)" stale ICE dump file(s) from previous run"
        for ice_file in $stale_ice_files
            log_message "    - $ice_file"
        end
        command rm -f rustc-ice-*.txt
    end
end

function detect_ice_from_file
    if test (count (find . -maxdepth 1 -name "rustc-ice-*.txt" 2>/dev/null)) -gt 0
        return 0
    end
    return 1
end

# Detects stale build artifacts in captured cargo output.
# When serde_core's build script output (private.rs) gets lost from /tmp while
# Cargo's metadata cache still thinks it exists, cargo commands fail with:
#   error: couldn't read `.../out/private.rs`: No such file or directory
#
# Parameters:
#   $argv[1]: Path to temp file containing captured cargo output
#
# Returns: 0 if stale artifacts detected, 1 otherwise
function detect_stale_build_artifacts
    set -l temp_output $argv[1]
    if grep -qE "couldn't read .*/out/.*: No such file or directory" $temp_output 2>/dev/null
        return 0
    end
    return 1
end

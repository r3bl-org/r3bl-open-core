#!/usr/bin/env fish

# Run commands and track failures
set -l failures

# Run nextest
if not cargo nextest run --all-targets > /dev/null 2>&1
    set -a failures "tests failed 😢"
end

# Run doctests
if not cargo test --doc > /dev/null 2>&1
    set -a failures "doctests failed 😢"
end

# Run doc build
if not cargo doc --no-deps > /dev/null 2>&1
    set -a failures "build failed 😢"
end

# Print results
if test (count $failures) -eq 0
    echo "✅ OK!"
else
    echo (string join ", " $failures)
end

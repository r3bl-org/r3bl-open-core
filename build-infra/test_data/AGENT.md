# Test Data - Do Not Modify

Files in this directory (and its subdirectories) are **golden test fixtures** for `cargo
rustdoc-fmt` validation tests. Each `input/` file has a matching `expected_output/` file that
the test suite compares byte-for-byte.

**Do not** refactor, reformat, rename, or rewrite any file here. Changes will break the
snapshot tests.

If you need to update a golden file, do so intentionally as part of a test change - never as
a side effect of a codebase-wide refactor.

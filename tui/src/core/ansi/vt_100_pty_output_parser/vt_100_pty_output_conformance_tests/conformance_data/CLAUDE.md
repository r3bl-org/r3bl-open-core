# Test Data - Do Not Modify

Files in this directory are **conformance test definitions** for the VT-100 PTY output parser.
They contain precise ANSI escape sequence definitions that the test suite validates against.

**Do not** refactor, reformat, rename, or rewrite any file here. Changes will break the
conformance tests.

If you need to update a definition, do so intentionally as part of a test change - never as a
side effect of a codebase-wide refactor.

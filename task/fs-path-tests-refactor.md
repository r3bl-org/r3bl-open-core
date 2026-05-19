# Task: Reorganize `fs_path.rs` test module

## Overview

The `tests` module in `tui/src/core/script/fs_path.rs:263-727` is a single flat module
containing ~465 lines: 1 helper function, 13 test functions, a sequential dispatcher, a
`generate_isolated_process_test!` invocation, and a `controller_fn`. This is hard to scan
because the test functions, fixtures, and orchestration are all interleaved.

Split the inner `tests` module into sub-modules that group tests by the function they
exercise, following the pattern already established in the RRT `process_isolated_tests/`
directory.

### Current structure (flat)

```
mod tests
├── strip_extended_length_prefix()    -- helper/fixture
├── test_try_directory_exists_*       -- 2 tests for try_directory_exists
├── test_try_file_exists*             -- 3 tests for try_file_exists
├── test_try_pwd*                     -- 2 tests for try_pwd
├── test_try_write()                  -- 1 test for try_write_file
├── test_try_mkdir()                  -- 1 test for try_mkdir
├── test_try_change_directory_*       -- 4 tests for try_cd
├── run_all_fs_path_functions_sequentially_impl()  -- dispatcher
├── generate_isolated_process_test!   -- macro invocation
└── controller_fn()                   -- process isolation controller
```

### Proposed structure

```
mod tests
├── mod fixtures
│   ├── strip_extended_length_prefix()
│   └── controller_fn()
│
├── mod test_directory_exists          -- 2 tests
│   ├── test_try_directory_exists_not_found_error()
│   └── test_try_directory_exists_permissions_errors()  [cfg(unix)]
│
├── mod test_file_exists               -- 3 tests
│   ├── test_try_file_exists()
│   ├── test_try_file_exists_invalid_name_error()  [cfg(unix)]
│   └── test_try_file_exists_permissions_errors()  [cfg(unix)]
│
├── mod test_pwd                       -- 2 tests
│   ├── test_try_pwd()
│   └── test_try_pwd_errors()  [cfg(unix)]
│
├── mod test_write                     -- 1 test
│   └── test_try_write()
│
├── mod test_mkdir                     -- 1 test
│   └── test_try_mkdir()
│
├── mod test_cd                        -- 4 tests
│   ├── test_try_change_directory_happy_path()
│   ├── test_try_change_directory_non_existent()
│   ├── test_try_change_directory_invalid_name()  [cfg(unix)]
│   └── test_try_change_directory_permissions_errors()  [cfg(unix)]
│
├── run_all_fs_path_functions_sequentially_impl()  -- dispatcher (stays at top)
└── generate_isolated_process_test!                -- macro (stays at top)
```

### Key decisions

- **Keep everything inline** in `fs_path.rs` rather than creating a directory. The module
  is ~465 lines - splitting into separate files would be overkill.
- **`fixtures` sub-module** for `strip_extended_length_prefix()` and `controller_fn()`.
  These are not tests - they're shared infrastructure.
- **Group by function-under-test**, not by error category. "What tests cover `try_cd`?" is
  more natural than "what permission tests exist?".
- **Dispatcher and macro stay at the `tests` level** since they orchestrate across all
  sub-modules.
- Each sub-module gets `use super::*;` plus `use super::fixtures::*;`.

## Implementation plan

### Phase 1: Reorganize into inner modules

- [ ] Create `mod fixtures` with `strip_extended_length_prefix()` and `controller_fn()`
- [ ] Create `mod test_directory_exists` with 2 tests
- [ ] Create `mod test_file_exists` with 3 tests
- [ ] Create `mod test_pwd` with 2 tests
- [ ] Create `mod test_write` with 1 test
- [ ] Create `mod test_mkdir` with 1 test
- [ ] Create `mod test_cd` with 4 tests
- [ ] Update dispatcher to use qualified paths (e.g., `test_cd::test_try_change_directory_happy_path()`)

### Phase 2: Verify

- [ ] `./check.fish --check && ./check.fish --clippy && ./check.fish --test`

# Task: Drop #[macro_use] in favor of explicit imports

## Overview

The codebase uses ~50 `#[macro_use]` annotations on module declarations to propagate
`macro_rules!` macros textually. This is a Rust 2015 pattern that is fragile: it depends
on module declaration order, breaks silently when modules are reordered, and the failure
mode ("cannot find macro") gives no hint about the root cause.

Modern Rust (2018+) makes `#[macro_export]` macros available at the crate root. Every
call site can use `use crate::macro_name;` to import them explicitly, regardless of
module position. This is more robust, self-documenting, and doesn't require maintaining
a `#[macro_use]` chain across every intermediate `mod.rs`.

### Scope

- Crate: `tui` (`r3bl_tui`)
- ~50 `#[macro_use]` annotations across the module tree
- Affects all `#[macro_export]` macros: `generate_pty_test!`, `generate_isolated_process_test!`,
  `generate_async_isolated_process_test!`, `tui_color!`, `box_start!`, `box_end!`,
  `retry_until_success_test!`, and others

### Migration strategy

For each `#[macro_use]` annotation:
1. Remove the `#[macro_use]` from the module declaration.
2. Find all unqualified usages of macros defined in that module.
3. Add `use crate::macro_name;` at each call site.
4. Verify compilation after each module's migration.

### Risk

Low. This is a mechanical refactor - the macros themselves don't change, only how they
are brought into scope. Each step is independently verifiable with `cargo check --tests`.

## Implementation plan

### Phase 1: Inventory

- [ ] Generate a complete list of all `#[macro_use]` annotations and which macros they
      propagate (grep for `#[macro_use]` and `#[macro_export]`)
- [ ] Map each macro to its call sites (grep for `macro_name!`)
- [ ] Identify any non-`#[macro_export]` macros that use `#[macro_use]` for crate-internal
      visibility only (these need `pub(crate)` or a different approach)

### Phase 2: Migrate leaf modules first

- [ ] Start with modules that define macros but have few consumers
- [ ] Remove `#[macro_use]`, add explicit imports at call sites
- [ ] Verify with `cargo check -p r3bl_tui --tests` after each module

### Phase 3: Migrate core modules

- [ ] Migrate heavily-used macros (`tui_color!`, `box_start!`, etc.)
- [ ] These will have many call sites - use find-and-replace

### Phase 4: Cleanup and verify

- [ ] Remove all remaining `#[macro_use]` annotations
- [ ] Verify full build: `./check.fish --full`

### Phase 5: Update skills to prevent regression

- [ ] Update `.claude/skills/organize-modules/SKILL.md` to add a section on macro
      module organization: use `use crate::macro_name;` for `#[macro_export]` macros,
      do NOT use `#[macro_use]` on module declarations
- [ ] Update `.claude/skills/organize-modules/examples.md` to add an example showing
      the correct pattern for a module that defines `#[macro_export]` macros (define
      in submodule, import with `use crate::` at call sites)
- [ ] Update `.claude/skills/write-documentation/SKILL.md` if it references
      `#[macro_use]` anywhere (check and remove/replace)
- [ ] Add a note to `CLAUDE.md` under "Rust Code Guidelines" that `#[macro_use]` is
      not used in this codebase - use explicit imports instead

---
name: safe-renaming
description: Safely execute rename requests and mass refactoring using file-by-file native edits, semantic AST tools, and optional BTRFS staging verification.
---

## When to Use This Skill
Activate this skill whenever the user asks for rename requests (e.g., "Rename X to Y across the codebase", "tree-wide rename", "safely rename ..."), or any renaming of variables, fields, functions, parameters, or types across files in the codebase.

## Mandate: No Scripts of Any Kind
Scripts of ANY kind (shell scripts like `perl`, `sed`, `awk`, `python`, `bash`, `fish`, or custom/disposable Rust scripts compiled via `rustc`) are **STRICTLY PROHIBITED** for performing code refactoring or modifications. All changes must be performed file-by-file using native file-editing tools (`replace_file_content` / `multi_replace_file_content`) combined with AST-level semantic tools (`rust-analyzer`).

## Workflow: Safe Staged Refactoring Protocol

To ensure safety during multi-file refactoring, follow this protocol:

### Step 1: Optional BTRFS Staging Copy
For large changes, use `cp --reflink=auto` to create a lightweight, near-instant BTRFS CoW snapshot copy of the workspace in `~/Downloads/rename-staging/`.

```bash
# Ensure any previous staging directory is clean
rm -rf ~/Downloads/rename-staging

# Create BTRFS reflink clone
cp --reflink=auto -r ~/github/roc ~/Downloads/rename-staging
```

### Step 2: Systematic File-by-File Native Edits
Perform the refactoring file-by-file using native file editing tools (`replace_file_content` / `multi_replace_file_content`) and semantic AST tools:
1. Search and inspect target occurrences using AST references or grep.
2. Carefully apply edits line-by-line or chunk-by-chunk to preserve surrounding rustdoc, formatting, and comments.
3. Beware of local variable shadowing (e.g. `let vp_width = ...` shadowing `vp_width(...)`).

### Step 3: Incremental Validation
After modifying a batch of 3-5 files, run validation commands:
```bash
./check.fish --check
./check.fish --clippy
./check.fish --test
./check.fish --quick-doc
```

### Step 4: Final Verification
Run full workspace verification on the live repository to confirm 100% clean compilation, zero lint warnings, and all tests passing:
```bash
./check.fish --full
```

## Pitfalls & Safeguards
- **Substrings in Trait/Method names**: Ensure identifiers are properly bounded and not partially matching unrelated symbols.
- **Module Shadowing**: Watch out for local variable names (`let vp_width = ...`) that shadow constructor helper functions (`vp_width(...)`).
- **Doc Comment Integrity**: Ensure plain English prose in doc comments is not inadvertently changed when updating code symbols.
- **No Scripts**: Never reach for sed/awk/python/rust scripts to automate text replacements.

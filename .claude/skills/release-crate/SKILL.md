---
name: release-crate
description: Publish a crate release to crates.io with changelog, git tag, and GitHub release. Use when releasing a new version of any workspace crate.
---

# Crate Release

## When to Use

- When the user says "release", "publish", or invokes `/release <crate>`
- The argument is the crate directory name: `build-infra`, `tui`, `cmdr`,
  `analytics_schema`
  - The crate name begins with `r3bl-` prefix for binary crates and `r3bl_` prefix for
    library crates - the prefix is applied to the folder name of the crate (e.g.,
    `r3bl_tui` crate is in `tui` folder, and `r3bl-cmdr` crate is in the `cmdr` folder).
  - Always look at the Cargo.toml file of the crate to verify the crate name and version
    before invoking the release skill.

## Prerequisites

Before starting a release, verify:

- All code changes are complete and tested
- `./check.fish --full` passes (or equivalent checks have been run recently)
- You know the new version number (ask the user if not specified)

## Release Workflow

Follow these steps in order. Reference `docs/release-guide.md` for canonical examples.

### Step 1. Determine crate and version

- Identify the crate directory from the argument
- Read `<crate>/Cargo.toml` to find the current version
- Determine the new version (ask the user if not specified)
- Identify if this is a **binary crate** (`build-infra`, `cmdr`) or **library crate** (`tui`, `analytics_schema`)

### Step 2. Bump version in `<crate>/Cargo.toml`

- Update the `version` field to the new version

### Step 3. Update `CHANGELOG.md`

- Add the new version entry under the crate's section (before existing entries)
- Follow the existing format: `### vX.Y.Z (YYYY-MM-DD)` with summary, Fixed/Added/Changed sections
- Update the TOC at the top of the file to include the new entry

### Step 4. Update `docs/release-guide.md`

- Update the version number in the script block for the crate being released
- Update the git commit and tag lines to reflect the new version

### Step 5. Run build, test, docs, clippy, fmt

```bash
./check.fish --full
```

Or if `check.fish` is unavailable, run manually:

```bash
cd <crate>
cargo build && cargo test && cargo doc --no-deps && cargo clippy --fix --allow-dirty --allow-staged && cargo fmt --all
```

### Step 6. Generate README

```bash
cd <crate> && cargo readme > README.md
```

Note: If `cargo-readme` is not installed, skip this step and inform the user.

### Step 7. Dry-run publish

```bash
cd <crate> && cargo publish --dry-run --allow-dirty --no-verify
```

The `--no-verify` flag skips re-compilation of the packaged tarball, which fails with the
`wild` linker configured in `.cargo/config.toml`. The build/test/clippy checks in step 5
already verify correctness.

Verify the dry-run succeeds before proceeding.

### Step 8. Ask user permission before publishing

**IMPORTANT:** Use `AskUserQuestion` to ask the user for explicit permission before running
`cargo publish`. This is a non-reversible action that publishes to crates.io.

Present:
- The crate name and version being published
- A summary of changes from the changelog entry
- Ask: "Ready to publish to crates.io?"

### Step 9. Git commit and tag

```bash
git add -A
git commit -m "vX.Y.Z-<crate>"
git tag -a vX.Y.Z-<crate> -m "vX.Y.Z-<crate>"
```

Note: Do NOT use `-S` for signing - follow the project's git workflow conventions.

### Step 10. Publish to crates.io

```bash
cd <crate> && cargo publish --no-verify
```

### Step 11. Push to remote

```bash
git push && git push --tags
```

### Step 12. Create GitHub release

Use `gh release create` with release notes following the structure in `docs/release-guide.md`:

```bash
gh release create vX.Y.Z-<crate> --title "vX.Y.Z-<crate>" --notes "$(cat <<'EOF'
<release notes>
EOF
)"
```

**Release notes structure:**

```markdown
[Crate description]. Install with `cargo install <crate-name>`.

- tool-highlight - Brief description.

## vX.Y.Z (YYYY-MM-DD)

[One-liner summary from CHANGELOG]

**Fixed:**

- Item 1

**Added:**

- Item 1

## Full Changelog

- [crate vX.Y.Z](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#anchor)
```

**Crate-specific notes:**
- **binary crates** (`build-infra`, `cmdr`): Include install instructions (`cargo install <crate>`)
- **library crates** (`tui`): No install instructions needed

### Step 13. Install binary (binary crates only)

For binary crates (`build-infra`, `cmdr`):

```bash
cargo install --path <crate> --force
```

## Checklist Summary

1. [ ] Version bumped in `Cargo.toml`
2. [ ] `CHANGELOG.md` updated (entry + TOC)
3. [ ] `docs/release-guide.md` updated (version in script block)
4. [ ] Build/test/docs/clippy/fmt pass
5. [ ] README generated
6. [ ] Dry-run publish succeeds
7. [ ] **User permission obtained** for `cargo publish`
8. [ ] Git commit + tag created
9. [ ] Published to crates.io
10. [ ] Pushed to remote (commits + tags)
11. [ ] GitHub release created
12. [ ] Binary installed (if applicable)

## Related Commands

- `/release` - Explicitly invokes this skill

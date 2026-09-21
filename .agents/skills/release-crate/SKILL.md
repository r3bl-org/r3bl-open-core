---
name: release-crate
description: Publish a crate release to crates.io with changelog, standalone release notes, git tag, and GitHub release. Use when releasing a new version of any workspace crate.
---

# Crate Release

## When to Use

- When the user says "release", "publish", or invokes `/release <crate>`
- The argument is the crate directory name: `build-infra`, `tui`, `cmdr`,
  `rust-analyzer-mcp-server`, `analytics_schema`
  - The crate name begins with `r3bl-` prefix for binary crates and `r3bl_` prefix for
    library crates - the prefix is applied to the folder name of the crate (e.g.,
    `r3bl_tui` crate is in `tui` folder, `r3bl-cmdr` crate is in the `cmdr` folder, and
    `r3bl-rust-analyzer-mcp-server` is in the `rust-analyzer-mcp-server` folder).
  - Always check the `Cargo.toml` file of the crate to verify the crate name and current version
    before executing the release workflow.

## Prerequisites

Before starting a release, verify:

- All code changes are complete and tested
- Local branch is `main` and working tree is clean
- `./check.fish --full` (or `./check.fish --check` + `./check.fish --test`) passes cleanly
- You know the new version number (ask the user if not specified)

---

## Standardized Discoverability Intro Blocks

Every release note must begin with a standardized introductory block to maximize discoverability across social and community channels (Hacker News, Reddit `r/rust`, LinkedIn, GitHub Releases):

### `r3bl_tui` (Core Library)
```markdown
> **r3bl_tui** is a fully async, immediate-mode TUI framework for Rust inspired by React, Elm, and web technologies. It features flexbox layouts, CSS-like styling, reactive state architecture, a custom Markdown renderer with syntax highlighting, gradient colors, emoji/grapheme clustering, modal dialogs, mouse support, async non-blocking readline, and diff-based rendering optimized for SSH. Built with zero-copy gap buffers, SIMD-friendly offscreen buffers (`Flat2DArray`), zero-allocation ANSI string generation, and VT100 PTY multiplexing primitives.

Add to `Cargo.toml`:
```toml
[dependencies]
r3bl_tui = "<version>"
```
```

### `r3bl-cmdr` (CLI & Productivity Apps)
```markdown
> **r3bl-cmdr** is a suite of fast, fully async TUI & CLI developer productivity tools built on `r3bl_tui`.
> 
> - 🌿 **`giti`**: Interactive Git CLI with visual branch selection and streamlined commit workflows.
> - 📝 **`edi`**: Terminal Markdown editor featuring syntax highlighting, gradient colors, emoji support, SSH-optimized diff-rendering, and a high-performance zero-copy gap buffer.
> - ⚡ **`env-source`**: Cross-platform environment loader that evaluates shell scripts across POSIX sh, Fish, PowerShell, and cmd.exe without blocking.

Install with:
```bash
cargo install r3bl-cmdr --force
```
```

### `r3bl-build-infra` (Tooling & Formatting)
```markdown
> **r3bl-build-infra** provides developer tools and utilities for Rust projects and documentation automation.
> 
> - 📐 **`cargo-rustdoc-fmt`**: CLI tool that formats Markdown tables and automatically converts inline code references into clean reference-style intra-doc links with technical term linking.

Install with:
```bash
cargo install r3bl-build-infra --force
```
```

### `r3bl-rust-analyzer-mcp-server` (AI Coding Agent MCP Server)
```markdown
> **r3bl-rust-analyzer-mcp-server** is a high-performance Model Context Protocol (MCP) server for `rust-analyzer`. Built with pure Rust standard library threads (no async runtime overhead) to provide lightning-fast AST code navigation, type hover, definition lookup, code actions, and compiler diagnostics directly to AI coding agents (Claude, Antigravity, Cursor, etc.).

Install with:
```bash
cargo install r3bl-rust-analyzer-mcp-server --force
```
```

---

## Release Workflow

Follow these steps in exact sequential order. Reference `docs/release-guide.md` for canonical examples.

### Step 1. Determine Crate and Version

- Identify the crate directory from the argument: `tui`, `build-infra`, `cmdr`, `rust-analyzer-mcp-server`, `analytics_schema`
- Read `<crate>/Cargo.toml` to find the current version
- Determine the new version (ask the user if not specified)
- Identify if this is a **binary crate** (`build-infra`, `cmdr`, `rust-analyzer-mcp-server`) or **library crate** (`tui`, `analytics_schema`)

### Step 2. Bump Version in `<crate>/Cargo.toml`

- Update the `version` field to the new version
- If releasing a dependent crate, update its internal dependency on other workspace crates (e.g., `r3bl_tui = { path = "../tui", version = "X.Y.Z" }`)

### Step 3. Update `CHANGELOG.md` with Bi-Directional Link

- Add the new version entry under the crate's section in `CHANGELOG.md` (before existing entries)
- Add the bi-directional link callout to the top of the version header pointing to the GitHub Release page:
  ```markdown
  ### vX.Y.Z (YYYY-MM-DD)
  > 🔗 **Release Notes & Migration Guide**: [vX.Y.Z-<crate>](https://github.com/r3bl-org/r3bl-open-core/releases/tag/vX.Y.Z-<crate>)
  ```
- Document summary, Breaking Changes, Added, Fixed, and Performance sections
- Update the Table of Contents at the top of `CHANGELOG.md` to include the new entry

### Step 4. Create Standalone Release Notes in `docs/release-notes/<crate>/vX.Y.Z.md`

- Ensure directory exists: `mkdir -p docs/release-notes/<crate>`
- Create `docs/release-notes/<crate>/vX.Y.Z.md` containing:
  1. Standardized Discoverability Intro block (from above)
  2. Motivation and "Why" behind the release
  3. Migration Guide (for major breaking releases with before/after snippets)
  4. Performance & Benchmark Highlights (with developerlife.com article/video links if applicable)
  5. Bi-directional link back to `CHANGELOG.md`:
     ```markdown
     ## 📄 Full Changelog
     - [<crate> vX.Y.Z Changelog Entry](https://github.com/r3bl-org/r3bl-open-core/blob/vX.Y.Z-<crate>/CHANGELOG.md#anchor)
     ```

### Step 5. Update `docs/release-guide.md`

- Update the version number in the script block for the crate being released
- Update the git commit and tag lines to reflect the new version

### Step 6. Run Build, Test, Docs, Clippy, Fmt

```bash
cargo update --workspace
./check.fish --fmt
./check.fish --clippy
./check.fish --quick-doc
./check.fish --test
```

### Step 7. Generate README

```bash
cd <crate> && cargo readme > README.md && cd ..
```

*Note: Only run this for crates where `src/lib.rs` is the Single Source of Truth (SSOT) for `README.md` (`tui`, `build-infra`, `cmdr`). Do NOT run this for `rust-analyzer-mcp-server` where `README.md` is hand-crafted and maintained directly for crates.io.*

### Step 8. Dry-Run Publish

```bash
cd <crate> && cargo publish --dry-run --allow-dirty --no-verify && cd ..
```

The `--no-verify` flag skips re-compilation of the packaged tarball, which fails with the `wild` linker configured in `.cargo/config.toml`. The build/test/clippy checks in Step 6 already verify correctness.

Verify that the dry-run succeeds before proceeding.

### Step 9. Ask User Permission Before Publishing

**CRITICAL:** Use `AskUserQuestion` (or request confirmation) before running `cargo publish`. This is a non-reversible action that publishes to crates.io.

Present:
- The crate name and version being published
- A summary of changes from the changelog entry
- Ask: "Ready to publish to crates.io?"

### Step 10. Git Commit and Tag

Stage only the modified files for this crate release explicitly (avoid blanket `git add -A`):

```bash
git add <modified-crate-files>
git commit -m "vX.Y.Z-<crate>"
git tag -a vX.Y.Z-<crate> -m "vX.Y.Z-<crate>"
```

*Note: Do NOT use `-S` for signing - follow the project's git workflow conventions.*

### Step 11. Publish to crates.io

```bash
cd <crate> && cargo publish --no-verify --allow-dirty && cd ..
```

### Step 12. Verify Crates.io Registry Index (For Multi-Crate Releases)

If downstream workspace crates depend on this newly published version:
- Verify that `cargo search <crate>` reports the new version before proceeding to publish dependent crates.

### Step 13. Push to Remote

```bash
git push origin main && git push origin vX.Y.Z-<crate>
```

### Step 14. Create GitHub Release via `--notes-file`

```bash
gh release create vX.Y.Z-<crate> --title "vX.Y.Z-<crate>" --notes-file docs/release-notes/<crate>/vX.Y.Z.md
```

### Step 15. Install Binary Locally (Binary Crates Only)

For binary crates (`build-infra`, `cmdr`, `rust-analyzer-mcp-server`):

```bash
cargo install --path <crate> --force
```

---

## Checklist Summary

1. [ ] Version bumped in `<crate>/Cargo.toml` (and dependency versions updated)
2. [ ] `CHANGELOG.md` updated (entry with tag-pinned release notes link + TOC)
3. [ ] `docs/release-notes/<crate>/vX.Y.Z.md` created with intro, highlights, and changelog link
4. [ ] `docs/release-guide.md` updated (version in script block)
5. [ ] Build/test/docs/clippy/fmt pass cleanly
6. [ ] README generated via `cargo readme`
7. [ ] Dry-run publish succeeds
8. [ ] **User permission obtained** for `cargo publish`
9. [ ] Git commit + tag created (`vX.Y.Z-<crate>`)
10. [ ] Published to crates.io
11. [ ] Crates.io propagation verified (`cargo search <crate>`) if other crates depend on it
12. [ ] Pushed to remote (`main` + tag)
13. [ ] GitHub release created using `--notes-file`
14. [ ] Binary installed locally via `cargo install --path <crate> --force` (if applicable)

## Related Commands

- `/release` - Explicitly invokes this skill

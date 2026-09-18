<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Release Guide](#release-guide)
  - [Full workflow](#full-workflow)
  - [Overview of the release process](#overview-of-the-release-process)
    - [Step 1. Build and publish to crates.io](#step-1-build-and-publish-to-cratesio)
    - [Step 2. Make a GitHub release from the tag](#step-2-make-a-github-release-from-the-tag)
      - [Release notes structure](#release-notes-structure)
      - [Key elements](#key-elements)
      - [Canonical examples](#canonical-examples)
      - [Crate-specific notes](#crate-specific-notes)
  - [Community Sharing & Social Channels (Hacker News, Reddit, LinkedIn)](#community-sharing--social-channels-hacker-news-reddit-linkedin)
    - [Reddit (r/rust)](#1-reddit-rrust)
    - [Hacker News (Show HN / Link)](#2-hacker-news-show-hn--link)
    - [LinkedIn & Social Media](#3-linkedin--social-media)
  - [Deprecated workflow for archived crates](#deprecated-workflow-for-archived-crates)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Release Guide

## Full workflow

```bash
cd analytics_schema
# 1. Update version in Cargo.toml (for self) and this file
# 2. Update CHANGELOG.md (don't forget to update TOC)
# 3. Run "Dependi: Update All dependencies to the latest version" in vscode
#    w/ the Cargo.toml file open. Don't use `cargo-edit`
#    <https://github.com/killercup/cargo-edit> and `cargo upgrade`.
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged; cargo fmt --all
# Generate the crates.io landing page for this crate
cargo readme > README.md
cargo publish --dry-run --allow-dirty --no-verify
git add -A
git commit -S -m "v0.0.3-analytics_schema"
git tag -a v0.0.3-analytics_schema -m "v0.0.3-analytics_schema"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd tui
# 1. Update version in Cargo.toml (for self, optionally for dep: `r3bl_analytics_schema`)
#    and this file
# 2. Update CHANGELOG.md (don't forget to update TOC)
# 3. Run "Dependi: Update All dependencies to the latest version" in vscode
#    w/ the Cargo.toml file open. Don't use `cargo-edit`
#    <https://github.com/killercup/cargo-edit> and `cargo upgrade`.
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged; cargo fmt --all
# Generate the crates.io landing page for this crate
cargo readme > README.md
cargo publish --dry-run --allow-dirty --no-verify
git add -A
git commit -S -m "v0.7.8-tui"
git tag -a v0.7.8-tui -m "v0.7.8-tui"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd cmdr
# 1. Update version in Cargo.toml (for self, optionally for deps: `r3bl_tui`, `r3bl_analytics_schema`)
#    and this file
# 2. Update CHANGELOG.md (don't forget to update TOC)
# 3. Run "Dependi: Update All dependencies to the latest version" in vscode
#    w/ the Cargo.toml file open. Don't use `cargo-edit`
#    <https://github.com/killercup/cargo-edit> and `cargo upgrade`.
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged; cargo fmt --all
# Generate the crates.io landing page for this crate
cargo readme > README.md
cargo publish --dry-run --allow-dirty --no-verify
git add -A
git commit -S -m "v0.0.26-cmdr"
git tag -a v0.0.26-cmdr -m "v0.0.26-cmdr"
cargo publish --no-verify --allow-dirty
# TODO: Test release on clean machine with `spawny r3bl-cmdr` (see task/pending/build_infra_spawny.md)
git push ; git push --tags # Push tags & commits
cd ..

cd build-infra
# 1. Update version in Cargo.toml (for self, and for deps: `r3bl_tui`)
#    and this file
# 2. Update CHANGELOG.md (don't forget to update TOC)
# 3. Run "Dependi: Update All dependencies to the latest version" in vscode
#    w/ the Cargo.toml file open. Don't use `cargo-edit`
#    <https://github.com/killercup/cargo-edit> and `cargo upgrade`.
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged; cargo fmt --all
# Generate the crates.io landing page for this crate
cargo readme > README.md
cargo publish --dry-run --allow-dirty --no-verify
git add -A
git commit -S -m "v0.0.5-build-infra"
git tag -a v0.0.5-build-infra -m "v0.0.5-build-infra"
cargo publish --no-verify --allow-dirty
# TODO: Test release on clean machine with `spawny r3bl-build-infra` (see task/pending/build_infra_spawny.md)
git push ; git push --tags # Push tags & commits
cd ..

# Push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags

# Finally, make a GitHub release for each tag that you have created
# and copy the release notes from the CHANGELOG.md file.
# <https://github.com/r3bl-org/r3bl-open-core/releases/new>
```

## Overview of the release process

This is a lengthy and repetitive process. The following steps have to be applied repeatedly to all
the crates in the project. Look at the [full workflow](#full-workflow) section for the imperative
instructions on how to do this. The steps below are the algorithm that has to be applied repeatedly
to each crate in the project.

### Step 1. Build and publish to crates.io

Starting at the root folder of the project, eg `~/github/r3bl-open-core/`, the following steps are
applied to each crate (`tui`, `cmdr`, `analytics_schema`):

1. Update the version in `Cargo.toml`.
2. Make sure to run the "Crates: Update all dependencies of the Cargo.toml" action in VSCode for
   each `Cargo.toml` file in the `~/github/r3bl-open-core/` folder. You can run
   `run.nu upgrade-deps` to see which crates need to be updated.
   - This will update all the dependencies in all the crates in addition to updating all the
     remaining `Cargo.toml` in the other crates so that `run.nu build` runs.
   - Run `run.nu full-build` to make sure everything builds.
3. Make a git commit eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git add -A ; git commit -S -m "vX.Y.Z-core"`.
4. Make a git tag eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git tag -a vX.Y.Z-core -m "vX.Y.Z-core"`.
5. Update the `CHANGELOG.md` with all the new updates.
6. Run `cargo publish --dry-run --no-verify` in the crate folder. This will perform a dry run of
   publishing the crate to crates.io. The `--no-verify` flag skips re-compilation of the packaged
   tarball, which fails with the `wild` linker configured in `.cargo/config.toml`. The actual
   build/test/clippy checks in step 5 already verify correctness.
7. Then run `cargo publish --no-verify --allow-dirty` in the crate folder. This will publish the crate to
   crates.io.

### Step 2. Make a GitHub release from the tag

Then, push the git commit and tag to the remote repo: `git push ; git push --tags`.

Finally, for the tag, make a GitHub release using the standalone release note file in
`docs/release-notes/<crate>/vX.Y.Z.md`. The release notes contain the standardized discoverability
intro, highlights, migration guides, and a link back to the `CHANGELOG.md` entry.

#### Release notes creation & publication

1. Create `docs/release-notes/<crate>/vX.Y.Z.md`:

```markdown
> [Standardized Crate Discoverability Intro Block]. Install with `cargo install <crate>`.

- 📝 **tool-name** - Brief tool description.

## vX.Y.Z (YYYY-MM-DD)

[One-liner summary from CHANGELOG]

**Fixed:**

- Item 1
- Item 2

**Added:**

- Item 1

## Coming Soon 🚀

[Optional - roadmap items if applicable]

## Full Changelog

- [crate vX.Y.Z](https://github.com/r3bl-org/r3bl-open-core/blob/vX.Y.Z-<crate>/CHANGELOG.md#anchor)
```

2. Create the GitHub release via `--notes-file`:

```bash
gh release create vX.Y.Z-<crate> --title "vX.Y.Z-<crate>" --notes-file docs/release-notes/<crate>/vX.Y.Z.md
```

#### Key elements

| Element              | Description                                                              |
| -------------------- | ------------------------------------------------------------------------ |
| Crate description    | Same text used across ALL releases for the crate (from CHANGELOG header) |
| Install instructions | Inline with description: `Install with \`cargo install <crate>\``        |
| Tool highlight       | Emoji + bold tool name + brief description                               |
| Version section      | Copy from CHANGELOG with `**Fixed:**` / `**Added:**` headers             |
| Coming Soon          | Optional roadmap section (used for `build-infra`)                        |
| Full Changelog       | Link to the specific version anchor in CHANGELOG.md                      |

#### Canonical examples

Use these releases as style guides:

| Crate       | Example                                                                                          | Notes                                   |
| ----------- | ------------------------------------------------------------------------------------------------ | --------------------------------------- |
| tui         | [v0.7.7-tui](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.7.7-tui)                 | Library crate - no install instructions |
| cmdr        | [v0.0.25-cmdr](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.25-cmdr)             | Binary crate with install instructions  |
| build-infra | [v0.0.1-build-infra](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.1-build-infra) | Binary crate with Coming Soon section   |

#### Crate-specific notes

- **cmdr**: Binary crate - include install instructions
- **tui**: Library crate - no install instructions needed, release is for notification only
- **build-infra**: Binary crate - include install instructions and Coming Soon section

#### URL pinning in CHANGELOG.md

When writing CHANGELOG.md entries, use SHA-pinned or release-tag-pinned URLs instead of `main`
branch URLs. URLs pointing to `main` break when files or directories are moved, renamed, or
deleted in later commits -- but pinned URLs are immutable snapshots that remain valid as a
historical record.

- **Tag-pinned** (preferred when a release tag exists):
  `https://github.com/r3bl-org/r3bl-open-core/blob/v0.7.7-tui/tui/README.md`
- **SHA-pinned** (when no tag is available):
  `https://github.com/r3bl-org/r3bl-open-core/blob/abc1234/path/to/file`
- **Avoid**:
  `https://github.com/r3bl-org/r3bl-open-core/blob/main/path/to/file`

## Community Sharing & Social Channels (Hacker News, Reddit, LinkedIn)

> [!NOTE]
> _This Week in Rust_ (TWiR) no longer accepts crate/library release submissions.
> Announcements should be shared directly across developer communities: **Reddit (`r/rust`)**,
> **Hacker News**, and **LinkedIn**.

After publishing a release, share the release announcement using the standalone release notes in
`docs/release-notes/<crate>/vX.Y.Z.md` (which already contain standardized discoverability intro
blocks, value propositions, code samples, benchmark metrics, and deep-dive links).

### 1. Reddit (`r/rust`)

- **Title format**: `[Release] <crate_name> vX.Y.Z: <Catchy One-Line Summary>`
  - Example: `[Release] r3bl_tui v0.8.0: Async TUI library with 2D Canvas/Viewport coords, Flat2DArray SIMD layout, and 0ms ESC handling`
- **Body content**: Copy the content from `docs/release-notes/<crate>/vX.Y.Z.md` (including the
  standardized discoverability intro block, migration guide, performance uplift stats, and links to
  articles/videos on `developerlife.com`).

### 2. Hacker News (Show HN / Link)

- **Title format**: `Show HN: <crate_name> vX.Y.Z - <Concise Value Proposition>`
  - Example: `Show HN: R3BL TUI v0.8.0 - Async Rust TUI with Flat2DArray SIMD layout and zero-cost typestate safety`
- **URL**: Link to the GitHub Release page
  (`https://github.com/r3bl-org/r3bl-open-core/releases/tag/vX.Y.Z-<crate>`) or architectural
  deep-dive article on `developerlife.com`.

### 3. LinkedIn & Social Media

- Share high-signal highlights:
  - Key performance metrics (e.g., 2.3x rendering speedup, +-98% jitter elimination).
  - Type-safety research foundations (Stanford CS 242 / FUNARCH typestate patterns).
  - Links to the `developerlife.com` articles and YouTube video walkthroughs.

## Deprecated workflow for archived crates

This used to be apply to the crates that are currently archived in
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo.

```bash
cd tuify
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.2.0-tuify"
git tag -a "v0.2.0-tuify" -m "v0.2.0-tuify"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd terminal_async
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.6.0-terminal_async"
git tag -a v0.6.0-terminal_async -m "v0.6.0-terminal_async"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd ansi_color
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.7.0-ansi_color"
git tag -a v0.7.0-ansi_color -m "v0.7.0-ansi_color"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd core
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.10.0-core"
git tag -a v0.10.0-core -m "v0.10.0-core"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd macro
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.10.0-macro"
git tag -a v0.10.0-macro -m "v0.10.0-macro"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd test_fixtures
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.1.0-test_fixtures"
git tag -a "v0.1.0-test_fixtures" -m "v0.1.0-test_fixtures"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..


cd simple_logger
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.1.4-simple_logger"
git tag -a v0.1.4-simple_logger -m "v0.1.4-simple_logger"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd redux
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.2.8-redux"
git tag -a v0.2.8-redux -m "v0.2.8-redux"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

cd utils
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty --no-verify
cargo readme > README.md
git add -A
git commit -S -m "v0.9.16-utils"
git tag -a v0.9.16-utils -m "v0.9.16-utils"
cargo publish --no-verify --allow-dirty
git push ; git push --tags # Push tags & commits
cd ..

# Finally, push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags
```

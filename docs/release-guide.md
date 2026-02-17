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
  - [This Week in Rust submission](#this-week-in-rust-submission)
    - [Link text format](#link-text-format)
    - [Structure](#structure)
    - [Length guidelines](#length-guidelines)
    - [Personality words](#personality-words)
    - [Abbreviations](#abbreviations)
    - [Example PR](#example-pr)
  - [Deprecated workflow for archived crates](#deprecated-workflow-for-archived-crates)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Release Guide

## Full workflow

```sh
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
git commit -S -m "v0.0.4-build-infra"
git tag -a v0.0.4-build-infra -m "v0.0.4-build-infra"
cargo publish --no-verify
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
7. Then run `cargo publish --no-verify` in the crate folder. This will publish the crate to
   crates.io.

### Step 2. Make a GitHub release from the tag

Then, push the git commit and tag to the remote repo: `git push ; git push --tags`.

Finally, for the tag, make a GitHub release and use the tag that you just created. No binary
artifacts are uploaded to the GitHub release, only the tag and the information from the
`CHANGELOG.md` file is used to create the release notes. The purpose of the GitHub release is to
notify users that a new release is available. This is useful if the user has signed up for GitHub
notifications for the repository.

#### Release notes structure

Every release follows a consistent structure. Use `gh release create` or `gh release edit`:

```markdown
[Crate description - same across all releases for this crate]. Install with `cargo install <crate>`.

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

- [crate vX.Y.Z](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#anchor)
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

## This Week in Rust submission

After publishing a release, submit it to [This Week in Rust](https://this-week-in-rust.org/) for
visibility in the Rust community. Submit a PR to the
[this-week-in-rust repo](https://github.com/rust-lang/this-week-in-rust) adding entries to the
current draft file under `### Project/Tooling Updates`.

### Link text format

Use descriptive one-liners, not just version numbers:

```markdown
- [r3bl_tui v0.7.7: modern async TUI lib — readline, md editor, flexbox, SSH-optimized rendering](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.7.7-tui)
- [r3bl-cmdr v0.0.25: TUI productivity apps - giti (git helper) and edi (beautiful md editor)](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.25-cmdr)
- [r3bl-build-infra v0.0.1: cargo-rustdoc-fmt — prettier md tables and ref-style links](https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.1-build-infra)
```

### Structure

```
crate vX.Y.Z: category — feature, feature, feature
```

- **Category first**: "modern async TUI lib", "TUI productivity apps", "cargo-rustdoc-fmt"
- **Em dash separator** (—): Cleaner than "with" or "and"
- **Feature list**: Comma-separated, most important first

### Length guidelines

| Range  | Assessment                                    |
| ------ | --------------------------------------------- |
| 9-50   | Too terse — says nothing about the crate      |
| 75-95  | Sweet spot — informative but scannable        |
| 96-160 | Acceptable — TWiR has entries up to 160 chars |

Check existing entries in recent TWiR issues for reference. Our entries should fit comfortably in
the middle of the range (75-95 chars).

### Personality words

Add personality without being cheesy:

| Word           | Effect                              |
| -------------- | ----------------------------------- |
| "modern"       | Signals fresh approach, not legacy  |
| "beautiful"    | Evocative, appeals to aesthetics    |
| "prettier"     | Playful nod to the famous formatter |
| "productivity" | Aspirational, implies value         |

### Abbreviations

| Do                 | Don't                     |
| ------------------ | ------------------------- |
| "md" (in features) | "markdown" (wastes space) |
| "lib"              | "library"                 |
| "ref-style"        | "reference-style"         |
| spell out "with"   | "w/" (too informal)       |

### Example PR

See [PR #7555](https://github.com/rust-lang/this-week-in-rust/pull/7555) for a complete example.

## Deprecated workflow for archived crates

This used to be apply to the crates that are currently archived in
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo.

```sh
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
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
cargo publish --no-verify
git push ; git push --tags # Push tags & commits
cd ..

# Finally, push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags
```

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Release Guide](#release-guide)
  - [Full workflow](#full-workflow)
  - [Overview of the release process](#overview-of-the-release-process)
    - [Step 1. Build and publish to crates.io](#step-1-build-and-publish-to-cratesio)
    - [Step 2. Make a GitHub release from the tag](#step-2-make-a-github-release-from-the-tag)
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
cargo publish --dry-run --allow-dirty
git add -A
git commit -S -m "v0.0.3-analytics_schema"
git tag -a v0.0.3-analytics_schema -m "v0.0.3-analytics_schema"
cargo publish
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
cargo publish --dry-run --allow-dirty
git add -A
git commit -S -m "v0.7.7-tui"
git tag -a v0.7.7-tui -m "v0.7.7-tui"
cargo publish
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
cargo publish --dry-run --allow-dirty
git add -A
git commit -S -m "v0.0.25-cmdr"
git tag -a v0.0.25-cmdr -m "v0.0.25-cmdr"
cargo publish
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
cargo publish --dry-run --allow-dirty
git add -A
git commit -S -m "v0.0.1-build-infra"
git tag -a v0.0.1-build-infra -m "v0.0.1-build-infra"
cargo publish
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
6. Run `cargo publish --dry-run` in the crate folder. This will perform a dry run of publishing the
   crate to crates.io.
7. Then run `cargo publish` in the crate folder. This will publish the crate to crates.io.

### Step 2. Make a GitHub release from the tag

Then, push the git commit and tag to the remote repo: `git push ; git push --tags`.

Finally, for the tag, make a GitHub release and use the tag that you just created. This only applies
to the `tui` and `cmdr` crates. No binary artifacts are uploaded to the GitHub release, only the tag
and the information from the `CHANGELOG.md` file is used to create the release notes. The purpose of
the GitHub release is to notify users that a new release is available. This is useful if the user
has signed up for GitHub notifications for the repository. The release notes should include the
following:

- For `cmdr` include instructions that the user can use to install the crate using
  `cargo install r3bl-cmdr`.
- For `tui` there are no installation instructions, since it is a library crate. The release is just
  a way for users to be notified by GitHub that a new release is available.

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
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.2.0-tuify"
git tag -a "v0.2.0-tuify" -m "v0.2.0-tuify"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd terminal_async
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.6.0-terminal_async"
git tag -a v0.6.0-terminal_async -m "v0.6.0-terminal_async"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd ansi_color
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.7.0-ansi_color"
git tag -a v0.7.0-ansi_color -m "v0.7.0-ansi_color"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd core
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.10.0-core"
git tag -a v0.10.0-core -m "v0.10.0-core"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd macro
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.10.0-macro"
git tag -a v0.10.0-macro -m "v0.10.0-macro"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd test_fixtures
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.1.0-test_fixtures"
git tag -a "v0.1.0-test_fixtures" -m "v0.1.0-test_fixtures"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..


cd simple_logger
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.1.4-simple_logger"
git tag -a v0.1.4-simple_logger -m "v0.1.4-simple_logger"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd redux
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.2.8-redux"
git tag -a v0.2.8-redux -m "v0.2.8-redux"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd utils
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.9.16-utils"
git tag -a v0.9.16-utils -m "v0.9.16-utils"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

# Finally, push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags
```

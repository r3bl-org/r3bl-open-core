---
Title: Guide to cutting a release and publishing it to crates.io
Date: 2022-11-06
---

## Cut a release and publish it to crates.io
<a id="markdown-cut-a-release-and-publish-it-to-crates.io" name="cut-a-release-and-publish-it-to-crates.io"></a>

This is a lengthy and repetitive process. The following steps have to be applied repeatedly to all
the crates in the project.

Starting at the root folder of the project, eg `~/github/r3bl-open-core/`, the following
steps are applied to each crate (`simple_logger`, `ansi_color`, `core`, `macro`, `redux`,
`tui`, `tuify`, `analytics_schema` and "public" / self):

1. Update the version in `Cargo.toml`.
2. Make sure to run the "Crates: Update all dependencies of the Cargo.toml" action in VSCode for
   each `Cargo.toml` file in the `~/github/r3bl-open-core/` folder. You can run `run.nu upgrade-deps`
   to see which crates need to be updated.
   - This will update all the dependencies in all the crates in addition to updating all the
     remaining `Cargo.toml` in the other crates so that `run.nu build` runs.
   - Run `run.nu full-build` to make sure everything builds.
3. Make a git commit eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git add -A ; git commit -S -m "vX.Y.Z-core"`.
4. Make a git tag eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git tag -a vX.Y.Z-core -m "vX.Y.Z-core"`.
5. Update the `CHANGELOG.md` with all the new updates.

Once this phase is complete, then it is time to perform a dry run and then publish to crates.io.
Again starting at the root folder of the project, eg `~/github/r3bl-open-core/`, the following steps
are applied to each crate (`ansi_color`, `core`, `macro`, `redux`, `tui`, `tuify`, and self):

1. Run `cargo publish --dry-run` in the crate folder. This will perform a dry run of publishing the
   crate to crates.io.
2. Then run `cargo publish` in the crate folder. This will publish the crate to crates.io.

Finally, push the git commit and tag to the remote repo: `git push ; git push --tags`.

## Example of full workflow
<a id="markdown-example-of-full-workflow" name="example-of-full-workflow"></a>

```sh
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
git commit -S -m "v0.9.10-macro"
git tag -a v0.9.10-macro -m "v0.9.10-macro"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd tuify
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.5.9-tui"
git tag -a v0.5.9-tui -m "v0.5.9-tui"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd cmdr
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Don't forget to update `r3bl-base` to have the same `UPDATE_IF_NOT_THIS_VERSION`
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.0.16-cmdr"
git tag -a v0.0.16-cmdr -m "v0.0.16-cmdr"
cargo publish
# 2) Don't forget to test the release on a clean machine by running `cargo install r3bl-cmdr`
# You can do this using `cd cmdr && nu run build-release-in-docker`
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
git commit -S -m "v0.0.3-test_fixtures"
git tag -a "v0.0.3-test_fixtures" -m "v0.0.3-test_fixtures"
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
git commit -S -m "v0.5.7-terminal_async"
git tag -a v0.5.7-terminal_async -m "v0.5.7-terminal_async"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

cd analytics_schema
# Update version in Cargo.toml and this file
# Update CHANGELOG.md
# Run "Dependi: Update All dependencies to the latest version" in vscode w/ the Cargo.toml file open
# - instead of using `cargo-edit` https://github.com/killercup/cargo-edit and the `cargo upgrade` command
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged
cargo publish --dry-run --allow-dirty
cargo readme > README.md
git add -A
git commit -S -m "v0.0.2-analytics_schema"
git tag -a v0.0.2-analytics_schema -m "v0.0.2-analytics_schema"
cargo publish
git push ; git push --tags # Push tags & commits
cd ..

# Finally, push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags
```

## Deprecated workflow for crates moved to [r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repo

```sh
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

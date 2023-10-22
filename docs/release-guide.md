---
Title: Guide to cutting a release and publishing it to crates.io
Date: 2022-11-06
---

## Cut a release and publish it to crates.io

This is a lengthy and repetitive process. The following steps have to be applied repeatedly to all
the crates in the project.

Starting at the root folder of the project, eg `~/github/r3bl_rs_utils/`, the following steps are
applied to each crate (`simple_logger`, `ansi_color`, `core`, `macro`, `redux`, `tui`, `tuify`, and
"public" / self):

1. Update the version in `Cargo.toml`.
2. Make sure to run the "Crates: Update all dependencies of the Cargo.toml" action in VSCode for
   each `Cargo.toml` file in the `~/github/r3bl_rs_utils/` folder. You can run `run.nu upgrade-deps`
   to see which crates need to be updated.
   - This will update all the dependencies in all the crates in addition to updating all the
     remaining `Cargo.toml` in the other crates so that `run.nu build` runs.
   - Run `run.nu full-build` to make sure everything builds.
3. Make a git commit eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git add -A ; git commit -m "vX.Y.Z-core"`.
4. Make a git tag eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git tag -a vX.Y.Z-core -m "vX.Y.Z-core"`.
5. Update the `CHANGELOG.md` with all the new updates.

Once this phase is complete, then it is time to perform a dry run and then publish to crates.io.
Again starting at the root folder of the project, eg `~/github/r3bl_rs_utils/`, the following steps
are applied to each crate (`ansi_color`, `core`, `macro`, `redux`, `tui`, `tuify`, and self):

1. Run `cargo publish --dry-run` in the crate folder. This will perform a dry run of publishing the
   crate to crates.io.
2. Then run `cargo publish` in the crate folder. This will publish the crate to crates.io.

Finally, push the git commit and tag to the remote repo: `git push ; git push --tags`.

## Example of full workflow

```sh
cd ~/github/r3bl_rs_utils/

cd ansi_color
# Update cargo.toml version number manually
# Update CHANGELOG.md
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.6.9-ansi_color"
git tag -a v0.6.9-ansi_color -m "v0.6.9-ansi_color"
cargo publish
cd ..

cd simple_logger
# Update cargo.toml version number manually
# Update CHANGELOG.md
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.1.2-simple_logger"
git tag -a v0.1.2-simple_logger -m "v0.1.2-simple_logger"
cargo publish
cd ..

cd core
# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.9.7-core"
git tag -a v0.9.7-core -m "v0.9.7-core"
cd ..

cd tuify
# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.1.20-tuify"
git tag -a v0.1.20-tuify -m "v0.1.20-tuify"
cd ..

cd macro
# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.9.6-macro"
git tag -a v0.9.6-macro -m "v0.9.6-macro"
cd ..

cd redux
# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.2.5-redux"
git tag -a v0.2.5-redux -m "v0.2.5-redux"
cd ..

cd tui
# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.3.6-tui"
git tag -a v0.3.6-tui -m "v0.3.6-tui"
cd ..

# Update cargo.toml version number manually
cargo build; cargo test; cargo doc
git add -A
git commit -m "v0.9.9-public"
git tag -a v0.9.9-public -m "v0.9.9-public"

# Don't forget to publish to crates.io
cd simple_logger; cargo publish; cd ..
cd ansi_color; cargo publish; cd ..
cd core; cargo publish; cd ..
cd macro; cargo publish; cd ..
cd redux; cargo publish; cd ..
cd tui; cargo publish; cd ..
cd tuify; cargo publish; cd ..
cargo publish

# Finally, push the git commit and tag to the remote repo
git tag -l --sort=-creatordate # Check the tags
git push ; git push --tags
```

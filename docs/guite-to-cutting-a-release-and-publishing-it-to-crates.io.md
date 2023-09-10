---
Title: Guide to cutting a release and publishing it to crates.io
Date: 2022-11-06
---

## Cut a release and publish it to crates.io

This is a lengthy and repetitive process. The following steps have to be applied repeatedly to all
the crates in the project.

Starting at the root folder of the project, eg `~/github/r3bl_rs_utils/`, the following steps are
applied to each crate (`ansi_color`, `core`, `macro`, `redux`, `tui`, `tuify`, and "public" / self):

1. Update the version in `Cargo.toml`.
2. Run the script `./upgrade-deps.fish` in the `~/github/r3bl_rs_utils/` folder.
   - This will update all the dependencies in all the crates in addition to updating all the
     remaining `Cargo.toml` in the other crates so that `./build.fish` runs.
   - Run `./build.fish` to make sure everything builds.
3. Make a git commit eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git add -A ; git commit -m "vX.Y.Z-core"`.
4. Make a git tag eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git tag -a vX.Y.Z-core -m "vX.Y.Z-core"`.

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
# Update cargo.toml version number manually, lets say to 0.6.2
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd ansi_color
git add -A
git commit -m "v0.6.2-ansi_color"
git tag -a v0.6.2-ansi_color -m "v0.6.2-ansi_color"
cd ..

cd core
# Update cargo.toml version number manually, lets say to 0.9.3
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd core
git add -A
git commit -m "v0.9.3-core"
git tag -a v0.9.3-core -m "v0.9.3-core"
cd ..

cd macro
# Update cargo.toml version number manually, lets say to 0.9.3
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd macro
git add -A
git commit -m "v0.9.3-macro"
git tag -a v0.9.3-macro -m "v0.9.3-macro"
cd ..

cd redux
# Update cargo.toml version number manually, lets say to 0.2.3
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd redux
git add -A
git commit -m "v0.2.3-redux"
git tag -a v0.2.3-redux -m "v0.2.3-redux"
cd ..

cd tui
# Update cargo.toml version number manually, lets say to 0.3.4
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd tui
git add -A
git commit -m "v0.3.4-tui"
git tag -a v0.3.4-tui -m "v0.3.4-tui"
cd ..

cd tuify
# Update cargo.toml version number manually, lets say to 0.1.1
cd .. ; ./upgrade-deps.fish ; ./build.fish ; cd tuify
git add -A
git commit -m "v0.1.1-tuify"
git tag -a v0.1.1-tuify -m "v0.1.1-tuify"
cd ..

# Update cargo.toml version number manually, lets say to 0.9.8
./upgrade-deps.fish ; ./build.fish
git add -A
git commit -m "v0.9.8-public"
git tag -a v0.9.8-public -m "v0.9.8-public"

# Don't forget to publish to crates.io
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

## Current release status as of Sep 10 2022

| Crate      | Version           | Status                                       |
| ---------- | ----------------- | -------------------------------------------- |
| ansi_color | v0.6.2-ansi_color | https://crates.io/crates/r3bl_ansi_color     |
| core       | v0.9.3-core       | https://crates.io/crates/r3bl_rs_utils_core  |
| macro      | v0.9.3-macro      | https://crates.io/crates/r3bl_rs_utils_macro |
| redux      | v0.2.3-redux      | https://crates.io/crates/r3bl_redux          |
| tui        | v0.3.4-tui        | https://crates.io/crates/r3bl_tui            |
| tuify      | v0.1.1-tuify      | https://crates.io/crates/r3bl_tuify          |
| public     | v0.9.8-public     | https://crates.io/crates/r3bl_rs_utils       |

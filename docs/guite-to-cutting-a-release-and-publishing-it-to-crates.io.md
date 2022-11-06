# Guide to cutting a release and publishing it to crates.io

This is a lengthy and repetitive process. The following steps have to be applied repeatedly to all
the crates in the project.

Starting at the root folder of the project, eg `~/github/r3bl_rs_utils/`, the following steps are
applied to each crate (`core`, `macro`, `redux`, `tui`, and "public" / self):

1. Update the version in `Cargo.toml`.
2. Run the script `./upgrade-deps.fish` in the `~/github/r3bl_rs_utils/` folder. This will update
   all the dependencies in all the crates.
3. Make a git commit eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git commit -m 'v0.1.0-core'`.
4. Make a git tag eg `vX.Y.Z-$crate` where `$crate` is the name of the crate, and `vX.Y.Z` is the
   [semver](https://semver.org/) version number. Eg: `git tag -a v0.1.0-core -m 'v0.1.0-core'`.

Once this phase is complete, then it is time to perform a dry run and then publish to crates.io.
Again starting at the root folder of the project, eg `~/github/r3bl_rs_utils/`, the following steps
are applied to each crate (`core`, `macro`, `redux`, `tui`, and self):

1. Run `cargo publish --dry-run` in the crate folder. This will perform a dry run of publishing the
   crate to crates.io.
2. Then run `cargo publish` in the crate folder. This will publish the crate to crates.io.

Finally, push the git commit and tag to the remote repo: `git push ; git push --tags`.

# Current status Nov 6 2022

| Crate  | Version | Status                                       |
| ------ | ------- | -------------------------------------------- |
| core   | 0.8.3   | https://crates.io/crates/r3bl_rs_utils_core  |
| macro  | 0.8.3   | https://crates.io/crates/r3bl_rs_utils_macro |
| redux  | 0.1.3   | https://crates.io/crates/r3bl_rs_utils_redux |
| tui    | 0.1.3   | https://crates.io/crates/r3bl_rs_utils_tui   |
| public | 0.1.3   | https://crates.io/crates/r3bl_rs_utils       |

# Crate Release

Invoke the `release-crate` skill to publish a crate release following the release guide.
The argument should be the crate's folder name (e.g., `build-infra`, `tui`, `cmdr`).

The crate name begins with `r3bl-` prefix for binary crates and `r3bl_` prefix for library
crates - the prefix is applied to the folder name of the crate (e.g., `r3bl_tui` crate is
in `tui` folder, and `r3bl-cmdr` crate is in the `cmdr` folder). Always look at the
Cargo.toml file of the crate to verify the crate name and version before invoking the
release skill.

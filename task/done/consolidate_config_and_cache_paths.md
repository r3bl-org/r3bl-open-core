# Task: Consolidate R3BL `.config` and `.cache` Directory Paths (`consolidate_config_and_cache_paths`)

## Overview

Unify all R3BL persistent configuration and machine-scoped cache/state files under a
single `"r3bl"` umbrella folder inside the platform's configuration (`dirs::config_dir`)
and cache (`dirs::cache_dir`) directories, managed by a single centralized path resolver
module (`cmdr/src/common/r3bl_file_paths.rs`).

---

## Directory & File Hierarchy

```text
~/.config/r3bl/                                  <- Machine-agnostic portable config (safe to sync as-is across machines)
└── <crate_name>/                                <- e.g., ~/.config/r3bl/r3bl-cmdr/
    ├── edi.jsonc                                <- Per-binary JSONC config (future)
    └── giti.jsonc                               <- Per-binary JSONC config (future)

~/.cache/r3bl/                                   <- Machine-scoped state/cache (regenerable; in backups, namespace by host e.g. `nazmul-desktop.local/.cache/r3bl/...`)
├── proxy_machine_id                             <- Machine-wide analytics proxy ID (not product-specific)
└── <crate_name>/                                <- e.g., ~/.cache/r3bl/r3bl-cmdr/
    └── self_upgrade_last_check                  <- Per-crate 0-byte `mtime` cooldown timestamp file
```

### Architectural Rationale: `.config/r3bl` vs `.cache/r3bl`

1. **`~/.config/r3bl/<crate_name>/*` (`dirs::config_dir`) — Portable User Configuration**:
    - Scoped by crate/product name (`<crate_name>`, such as `r3bl-cmdr`), with per-binary
      `.jsonc` configuration files (e.g., `edi.jsonc`, `giti.jsonc`).
    - Completely machine-agnostic: safe to copy, sync, or check into dotfiles repositories
      **as-is** across multiple machines without colliding machine identity or state.

2. **`~/.cache/r3bl/...` (`dirs::cache_dir`) — Machine-Scoped Runtime State & Cache**:
    - Holds machine-specific or ephemeral state (`proxy_machine_id`,
      `<crate_name>/self_upgrade_last_check`).
    - Because these files are tied to a specific machine, they must **not** be synced flat
      as-is across multiple machines.
    - In backup/restore workflows, `~/.cache/r3bl/...` **can** be backed up and restored,
      provided it is namespaced by the machine's hostname in the backup archive (for
      example, `<backup_root>/nazmul-desktop.local/.cache/r3bl/...`).
    - If a machine starts fresh without a restored cache, all files in `~/.cache/r3bl/`
      safely regenerate on demand.

---

## Implementation Plan

### Step 1: Create Centralized Path Resolver (`cmdr/src/common/r3bl_file_paths.rs`)

- [x] Create `cmdr/src/common/r3bl_file_paths.rs` and re-export it in `cmdr/src/common/mod.rs`
      (which is re-exported at the root of `r3bl_cmdr`).
- [x] Define path constants in `r3bl_file_paths`:
    - `R3BL_TOP_LEVEL_DIR: &str = "r3bl"`
    - `PROXY_MACHINE_ID_FILENAME: &str = "proxy_machine_id"`
    - `SELF_UPGRADE_LAST_CHECK_FILENAME: &str = "self_upgrade_last_check"`
    - `CONFIG_FILE_EXT: &str = "jsonc"`
- [x] Implement `.config/r3bl` path resolution functions:
    - `try_get_r3bl_config_dir() -> Option<PathBuf>` (`<config_dir>/r3bl`)
    - `try_get_crate_config_dir(crate_name: &str) -> Option<PathBuf>`
      (`<config_dir>/r3bl/<crate_name>`)
    - `try_get_bin_config_file_path(crate_name: &str, bin_name: &str) -> Option<PathBuf>`
      (`<config_dir>/r3bl/<crate_name>/<bin_name>.jsonc`)
- [x] Implement `.cache/r3bl` path resolution functions:
    - `try_get_r3bl_cache_dir() -> Option<PathBuf>` (`<cache_dir>/r3bl`)
    - `try_get_crate_cache_dir(crate_name: &str) -> Option<PathBuf>`
      (`<cache_dir>/r3bl/<crate_name>`)
    - `try_get_proxy_machine_id_file_path() -> Option<PathBuf>`
      (`<cache_dir>/r3bl/proxy_machine_id`)
    - `try_get_self_upgrade_last_check_file_path(crate_name: &str) -> Option<PathBuf>`
      (`<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check`)
- [x] Add unit tests in `cmdr/src/common/r3bl_file_paths.rs` verifying all path suffixes and
      structures.

### Step 2: Migrate `analytics_client` (`proxy_machine_id`)

- [x] Delete `cmdr/src/analytics_client/config_folder.rs` and remove
      `pub mod config_folder;` from `cmdr/src/analytics_client/mod.rs`.
- [x] Update `cmdr/src/analytics_client/proxy_machine_id.rs` to use
      `r3bl_file_paths::try_get_proxy_machine_id_file_path()`, ensuring its parent directory
      (`<cache_dir>/r3bl/`) is created before writing `proxy_machine_id`.

### Step 3: Migrate `self_upgrade` Cooldown Path & Update Rustdocs

- [x] Update `cmdr/src/self_upgrade/constants.rs` (`check_cooldown_cache` module and its
      rustdocs) to reference `r3bl_file_paths` constants and document the `.config/r3bl` vs
      `.cache/r3bl` backup/namespacing model.
- [x] Update `cmdr/src/self_upgrade/check/cooldown.rs`
      (`resolve_cooldown_timestamp_file_path` and unit test
      `test_default_cache_dir_path_structure`) to resolve
      `<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check` via
      `r3bl_file_paths::try_get_self_upgrade_last_check_file_path`.
- [x] Update `cmdr/src/self_upgrade/config/upgrade_config.rs` rustdoc on
      `CooldownPolicy::DefaultCacheDir` to reflect the new path structure.
- [x] Update test fixture filenames in
      `cmdr/src/self_upgrade/upgrade_journey_public_api.rs` if any reference the old
      `.last_check` convention.

### Step 4: Verification & Mandatory Manual Review

- [x] Run `./check.fish --check` and targeted unit tests (`cargo test -p r3bl-cmdr -q`).
- [x] **Mandatory manual review**:
    - [x] `task/consolidate_config_and_cache_paths.md`
    - [x] `cmdr/src/common/r3bl_file_paths.rs`
    - [x] `cmdr/src/common/mod.rs`
    - [x] `cmdr/src/analytics_client/mod.rs`
    - [x] `cmdr/src/analytics_client/proxy_machine_id.rs`
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/check/cooldown.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_config.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey_public_api.rs`

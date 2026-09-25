# PRD: r3bl-cmdr Package Translation Utility (`pkg-translate`)

## Overview

`pkg-translate` is a core developer utility in **`r3bl-cmdr`** that translates canonical package identifiers into distro-native and Desktop Environment (DE)-aware package specifications. It replaces the legacy Fish/Bash package resolution scripts and centralizes package registry queries into a fast, type-safe Rust binary.

---

## Architectural Role

- **Crate**: `r3bl-cmdr` (binary: `pkg-translate`).
- **Engine**: Shared resolution and detection logic resides in `r3bl_tui::script::package_manager`.
- **Purpose**:
  - Provides instant CLI lookup: `pkg-translate <canonical-name> [--distro <distro>] [--de <de>]`.
  - Supports batch querying for automation scripts and fresh-install bootstrapping.
  - Automatically detects the running host distro and active desktop environment when flags are omitted.

---

## Specification: Distro + Desktop Environment Matrix

### Problem Definition
Historically, package definitions only mapped by distribution (`debian`, `fedora`, `arch`). However, desktop environment choices (KDE Plasma, GNOME, System76 COSMIC) dictate different system tools, image viewers, terminal file pickers, and shell plugins:
- An image viewer on KDE is `gwenview`, on GNOME is `loupe`, and on COSMIC is `cosmic-files` or `feh`.
- Some packages are universal (e.g. `git`, `curl`), some are distro-only (e.g. `build-essential` vs `base-devel`), and some are strictly DE-dependent.

### Package Categories

| Category | Example | Behavior |
| :--- | :--- | :--- |
| **Universal** | `curl`, `git` | Identical package name across all distros, no DE dependency. |
| **Distro-Specific** | `build-essential` vs `base-devel` | Varies by distro, no DE dependency. |
| **DE-Specific** | `image-viewer` (`gwenview`, `loupe`) | Varies by DE, and may also vary across distros. |
| **DE-Agnostic Apps** | `firefox`, `code` | Standard GUI apps that work across any DE. |

### Configuration Schema (JSONC)

Registries stored in `~/.config/r3bl-cmdr/packages/` (with compiled-in fallbacks) support both flat distro specs and nested DE mappings:

```jsonc
{
  // Universal / Distro-only mapping
  "build-tools": {
    "debian": "build-essential",
    "fedora": "gcc gcc-c++ make",
    "arch": "base-devel"
  },

  // Distro + Desktop Environment Matrix
  "image-viewer": {
    "debian": {
      "kde": "gwenview",
      "gnome": "loupe",
      "cosmic-de": "feh",
      "_default": "feh"
    },
    "fedora": {
      "kde": "gwenview",
      "gnome": "loupe",
      "cosmic-de": "feh",
      "_default": "feh"
    },
    "arch": {
      "kde": "gwenview",
      "gnome": "loupe",
      "cosmic-de": "feh",
      "_default": "feh"
    }
  }
}
```

---

## CLI Interface & Subcommands

```text
pkg-translate
  ├── resolve <package> [--distro <d>] [--de <de>] [--format <plain|json>]
  ├── list              [--category <name>]
  ├── search <query>
  └── detect            (prints detected host distro and desktop environment)
```

### Examples

```bash
# Auto-detects current distro and DE
pkg-translate resolve image-viewer

# Explicit query for another target environment
pkg-translate resolve image-viewer --distro arch --de kde

# Machine-readable output for scripts
pkg-translate resolve build-tools --format json
```

---

## Desktop Environment Detection Logic

The detection engine in `r3bl_tui::script::package_manager` inspects:
1. `XDG_CURRENT_DESKTOP` (e.g., `KDE`, `GNOME`, `COSMIC`).
2. `DESKTOP_SESSION`.
3. Fallback to `_default` in the schema if no specific DE is detected or matched.

---

## Implementation Plan

### Phase 1: Registry Parser & Models in `r3bl_tui::script`
- [ ] Implement strongly-typed serde models supporting both string and nested DE specs (`Untagged` / `PackageSpec` enum).
- [ ] Build JSONC comment stripping and parser.
- [ ] Add runtime DE detection (`detect_desktop_environment()`).

### Phase 2: `pkg-translate` CLI in `r3bl-cmdr`
- [ ] Add `[[bin]]` entry in `cmdr/Cargo.toml`.
- [ ] Implement `resolve`, `list`, `search`, and `detect` subcommands.
- [ ] Add shell-friendly output formatting (`--format plain|json`).

### Phase 3: Integration & Shell Wrappers
- [ ] Add Fish wrapper function in `~/scripts/fish/core/05-pkg-install.fish`.
- [ ] Verify translation against existing `~/scripts/config/packages/*.jsonc` files.
- [ ] Validate across Ubuntu, Fedora, and Arch test environments.

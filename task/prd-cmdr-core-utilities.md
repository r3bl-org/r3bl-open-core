# PRD: r3bl-cmdr Core Utilities & Script Engine Staging

## Overview

This project systematically replaces legacy Fish shell script utilities in `~/scripts/` with Rust-based core system utilities integrated directly into **`r3bl-cmdr`** and backed by the shared scripting engine in **`r3bl_tui::script`**.

### Architectural Alignment

R3BL software is organized into two explicit tiers:
- **Open Source (`roc`)**:
  - **Library**: `r3bl_tui` (framework, input/rendering pipeline, and `r3bl_tui::script` engine).
  - **Utilities**: `r3bl-cmdr` (core utilities like `compress`, `decompress`, `pkg-translate`, `env-source`, `rclone`, and interactive TUI apps like `giti`, `edi`, `nfs-manager`) and `r3bl-build-infra` (`cargo-rustdoc-fmt`, `spawny`).
- **Closed Source / Proprietary (`r3bl-base`)**:
  - **Products**: End-to-end commercial solutions such as `backup-buddy` (multi-tier storage orchestration), `media-buddy` (home media repatriation and modernization suite), and `rc`.

```text
┌─────────────────────────────────────────────────────────────┐
│                 Proprietary Tier: r3bl-base                 │
│  • backup-buddy (3-tier storage, env-save/load, recovery)   │
│  • media-buddy (disc rip, audiophile remux, AV1, HDR/DV)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ uses
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 Open Source Utilities: ROC                  │
│  • r3bl-cmdr:                                               │
│    - Core Utils: compress, decompress, env-source, rclone,  │
│                  pkg-translate                              │
│    - Interactive Apps: edi, giti, nfs-manager               │
│  • r3bl-build-infra: spawny, cargo-rustdoc-fmt              │
└──────────────────────────────┬──────────────────────────────┘
                               │ uses
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Open Source Library: ROC                   │
│  • r3bl_tui (rendering, terminal I/O, PTY, TUI components)  │
│  • r3bl_tui::script (process execution, compression engine, │
│                      git operations, package manager utils) │
└─────────────────────────────────────────────────────────────┘
```

---

## Vision

### Current State
- Sophisticated Fish shell scripts in `~/scripts/`.
- 235+ Fish functions across categories (compression, git, cargo, environment, system).
- Cross-distro testing validated in systemd-nspawn containers.

### Target State
- **No separate `coreutils` crate**: `r3bl-cmdr` is the single home for all open-source developer CLI tools and TUI apps built on `r3bl_tui`.
- **Core Engine in `r3bl_tui::script`**: Foundational logic (command running, package management, git ops, fs ops, compression) resides in `r3bl_tui::script`.
- **Binaries in `r3bl-cmdr/src/bin/`**:
  - Existing: `edi` (markdown editor), `giti` (interactive git), `env-source` (cross-platform shell environment loader).
  - Core utilities: `compress`, `decompress`, `pkg-translate`, `rclone`, `nfs-manager`.
- **Staging & Graduation**: New functionality iterates in `r3bl_cmdr` library modules first; once stable and generic, it graduates into `r3bl_tui::script`.

---

## Migration Philosophy: Rolling Release Model

Inspired by Arch Linux's rolling release approach:
- **Continuous delivery**: Each Rust binary is deployed as soon as it's validated.
- **Wrapper-based handoff**: Fish functions delegate to the Rust binary when available via PATH or explicit executable checks.
- **Aggressive cleanup**: Once a Rust binary is validated, delete the procedural Fish implementation.
- **Incremental value**: Backups and shell scripts immediately benefit from Rust speed and type safety without waiting for full migration.

---

## CLI Utilities Architecture

### 1. `compress` & `decompress` (Pilot A)
- **Goal**: Fast, memory-safe compression and decompression replacing Fish `compress` and `decompress` functions.
- **Formats**: Zstandard (`.tar.zst`), Gzip (`.tar.gz`), XZ (`.tar.xz`), Bzip2 (`.tar.bz2`).
- **Engine**: Implemented in `r3bl_tui::script::compression`.
- **User Experience**: Employs `r3bl_tui` rendering for modern progress spinners, cancel-handling (`Ctrl+C`), and interactive format selection if arguments are omitted.
- **Immediate Value**: Used across all backup routines (`backup-buddy`, local scripts) and directory bundling.

### 2. Configuration & Discovery
- **Config Directory**: `~/.config/r3bl-cmdr/`.
- **Format**: JSONC (JSON with comments) for human readability.
- **Discovery Order**:
  1. `--config <path>` (Explicit CLI override).
  2. `~/.config/r3bl-cmdr/<tool>.jsonc` (User configuration).
  3. Static compiled-in defaults.

---

## Fish Integration Pattern

Fish functions take precedence over PATH executables. The wrapper pattern enables seamless delegation:

```fish
function compress --argument-names archiveFilepath --argument-names rootFolder --argument-names subfolderToCompress
    # Delegate to Rust binary if available
    if command -q compress
        command compress $argv
        return $status
    end

    # Legacy Fish implementation fallback during transition
    # (Deleted once Rust binary is verified)
end
```

---

## Implementation Plan

### Phase 1: Core Compression Primitives in `r3bl_tui::script`
- [ ] Add `r3bl_tui::script::compression` module supporting Zstandard and tar archives.
- [ ] Implement stream-based compression and decompression with progress reporting.
- [ ] Support cancellation via `tokio::select!` / `cancellation_token`.

### Phase 2: `compress` and `decompress` Binaries in `r3bl-cmdr`
- [ ] Add `[[bin]]` entries for `compress` and `decompress` in `cmdr/Cargo.toml`.
- [ ] Implement CLI argument parsing with `clap` (derive).
- [ ] Connect `r3bl_tui` interactive spinners and progress bars.
- [ ] Add interactive format/tier picker when invoked without arguments.

### Phase 3: Verification & Integration
- [ ] Verify compression and round-trip extraction across multi-gigabyte directories.
- [ ] Benchmark against system `tar` + `zstd`.
- [ ] Deploy wrappers into `~/scripts/fish/core/10-utils.fish`.
- [ ] Verify test suite passes inside `spawny` containers.

# PRD: r3bl-cmdr Rclone Sync & Quota Manager (`rclone-sync`)

## 1. Overview

`rclone-sync` is a core utility in **`r3bl-cmdr`** that consolidates the disparate bash scripts (`rclone-sync.sh`, `rclone-status.sh`, `rclone-probe.sh`, `test-sync.sh`) into a single, type-safe Rust binary. It orchestrates high-volume file transfers to Google Drive while enforcing mandatory API rate limits, monitoring rolling 750GB quotas, and providing live terminal observability.

---

## 2. Core Problem & Rationale

Managing multi-terabyte cloud backups to Google Drive presents two critical challenges that generic tools do not handle:
1. **API Transaction Limits (Rate-Limiting vs Bandwidth)**:
   - Google enforces a strict metadata transaction limit (files created, modified, queried per second).
   - Without enforcing `--tpslimit 10 --tpslimit-burst 20`, `rclone` triggers a silent API soft-ban, causing transfer speeds to drop to 0 B/s and ETA to balloon to infinity without failing the process.
2. **750GB Daily Upload Cap**:
   - Google Drive enforces a hard 750GB rolling 24-hour upload window.
   - Hitting this limit returns Exit Code 7. A specialized tool must catch this exit code, notify the user, pause transfers, and run periodic "canary" probes to detect when quota frees up.

---

## 3. Architecture & Functional Requirements

### 3.1 Crate Location
- **Crate**: `r3bl-cmdr` (binary: `rclone-sync`).
- **Engine**: Shared process supervisor and log parsing in `r3bl_tui::script::rclone`.

### 3.2 CLI Commands

```text
rclone-sync
  ├── sync     <src> <dest> [--full-speed] [--dry-run] [--filters <path>]
  ├── status   (checks active sync PID, memory, bandwidth, and quota)
  ├── probe    (canary test: uploads tiny file to verify quota status)
  └── quota    (queries and displays Google Drive storage metrics)
```

### 3.3 Functional Capabilities
- **Throttled vs Full Speed**:
  - Default (Throttled): Limits bandwidth to avoid saturating domestic uplinks.
  - `--full-speed`: Removes bandwidth limits while **strictly preserving** `--tpslimit 10 --tpslimit-burst 20` to prevent API bans.
- **Canary Quota Prober**:
  - Periodically uploads and deletes a 1KB sentinel file to Google Drive.
  - Automatically triggers desktop notifications (`notify-send` / `notify-rust`) when quota reset is detected.
- **Process Supervisor & Observability**:
  - Streams real-time progress using `r3bl_tui` styled spinners/progress widgets.
  - Writes structured log records to `~/.local/state/r3bl-cmdr/rclone.log`.
  - Checks for conflicting sync daemons (e.g. `insync`) before launching bulk sync.

---

## 4. Implementation Plan

### Phase 1: Core Process Engine in `r3bl_tui::script`
- [ ] Implement wrapper around `rclone` process execution with argument sanitization.
- [ ] Build log streaming and bandwidth parser.
- [ ] Implement canary upload/deletion probe routine.

### Phase 2: CLI Interface in `r3bl-cmdr`
- [ ] Add `[[bin]]` entry in `cmdr/Cargo.toml`.
- [ ] Implement `sync`, `status`, `probe`, and `quota` subcommands with `clap`.
- [ ] Connect `r3bl_tui` live terminal progress view.

### Phase 3: Integration & Testing
- [ ] Test rate-limiting enforcement on large directories.
- [ ] Verify Exit Code 7 quota exhaustion handling and retry loop.
- [ ] Provide Fish wrapper aliases in `~/scripts/fish/commands/utils/`.

# PRD: r3bl-cmdr NFS Mesh Manager TUI (`nfs-manager`)

## 1. Overview

As the local development fleet grows (`nazmul-desktop`, `nazmul-laptop`, `nazmul-ext`), manually configuring bidirectional high-speed NFS mounts (such as `/media/nazmul/INT_HDD` and `/media/nazmul/NAS`) alongside mesh network folders (`~/Downloads/vm-share`) is error-prone. 

`nfs-manager` is an interactive TUI app in **`r3bl-cmdr`** built with **`r3bl_tui`** to visualize, audit, and orchestrate NFS exports and client automounts across machines without manual `/etc/exports` or `/etc/fstab` editing.

---

## 2. Architecture & Design

### 2.1 Crate Location
- **Crate**: `r3bl-cmdr` (binary: `nfs-manager`).
- **Engine**: Core network file system parsing and SSH orchestration in `r3bl_tui::script::nfs`.

### 2.2 Core Engine (`r3bl_tui::script::nfs`)
- **State Parsing**: Parses existing `/etc/exports` and `/etc/fstab` using structured models and regex.
- **FSID Registry**: Maintains a shared JSONC registry (`~/.config/r3bl-cmdr/nfs-registry.jsonc`) to prevent Btrfs/ZFS UUID collisions by guaranteeing unique `fsid` integer allocations across the fleet.
- **SSH Orchestration**: Uses SSH multiplexing to execute systemd reloads, `umount`, `rmdir`, `mkdir`, and `exportfs` on remote nodes without exiting the interactive TUI.

### 2.3 Interactive TUI Frontend
- **Fleet Dashboard**: Split-pane view showing:
  - Active exports hosted on the current machine.
  - Active client mounts connected to remote hosts.
- **Creation Wizard**: Step-by-step interactive form:
  1. Select local source directory (e.g. `/media/nazmul/INT_SSD`).
  2. Select remote target machine (e.g. `nazmul-laptop.local`).
  3. Enter target mount point (e.g. `/media/nazmul/INT_SSD`).
  4. Automatically computes the next available safe `fsid`.
- **Validation & Dry-Run Engine**: Shows colorized diffs of proposed changes to `/etc/exports` and `/etc/fstab` (`systemd.automount` payload) for confirmation before applying.

---

## 3. Implementation Plan

### Phase 1: Engine & Registry Models in `r3bl_tui::script`
- [ ] Implement parser and serializer for `/etc/exports` entries.
- [ ] Implement parser and serializer for `/etc/fstab` `systemd.automount` lines.
- [ ] Build `fsid` registry allocator backed by `~/.config/r3bl-cmdr/nfs-registry.jsonc`.

### Phase 2: Interactive TUI in `r3bl-cmdr`
- [ ] Add `[[bin]]` entry in `cmdr/Cargo.toml` for `nfs-manager`.
- [ ] Build split-pane dashboard component showing local exports and remote mounts.
- [ ] Implement creation wizard using `r3bl_tui` input components.
- [ ] Add dry-run confirmation modal.

### Phase 3: SSH Orchestration & Fleet Verification
- [ ] Implement remote command runner via SSH connection pooling.
- [ ] Verify export reload (`exportfs -ra`) and systemd daemon reload.
- [ ] Test end-to-end mount creation between desktop and laptop.

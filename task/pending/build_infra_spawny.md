# Plan: Clean-Machine Release Testing with systemd-nspawn

## Overview

Add a `spawny` subcommand to `r3bl-build-infra` that tests crate releases on
clean Linux machines using systemd-nspawn (replacing Docker).

## Why systemd-nspawn over Docker?

- **Native Linux containers**: No daemon, no Docker Desktop licensing concerns
- **Faster startup**: ~1s vs ~3-5s for Docker
- **Zygote pattern**: Snapshot clean state, restore instantly between tests
- **Multi-distro**: Test on Ubuntu, Fedora, Arch simultaneously
- **Already proven**: `~/scripts/tests/` infrastructure is battle-tested

## Reference Implementation

The existing fish scripts in `~/scripts/tests/` provide:

```
~/scripts/tests/
├── setup.fish       # Create Ubuntu/Fedora/Arch machines
├── run.fish         # Run tests, shell access, machine control
├── teardown.fish    # Delete machines
├── status.fish      # Show machine status
└── lib/
    ├── nspawn.fish       # Core nspawn operations
    ├── ensure-prereqs.fish
    └── test-helpers.fish
```

Key concepts:
- **Machines**: Full Linux installs in `/var/tmp/test-machines/<distro>-test/`
- **Zygotes**: Snapshots in tmpfs (`/tmp/zygotes/`) for instant restore
- **Bind mounts**: Share host directories into container

## Proposed Commands

```bash
# Test a crate release on all distros (parallel)
spawny r3bl-cmdr

# Test on specific distro
spawny r3bl-cmdr --distro ubuntu

# Test local build (not crates.io)
spawny r3bl-cmdr --local

# Setup machines (one-time)
spawny --setup

# Teardown machines
spawny --teardown

# Interactive shell for debugging
spawny --shell ubuntu
```

## Implementation Plan

### Phase 1: Core Infrastructure (Rust port of nspawn.fish)

1. **`nspawn` module** in `build-infra/src/`
   - Machine state detection (exists, running, registered)
   - Machine lifecycle (start, stop, create, delete)
   - Command execution inside containers
   - Zygote snapshot/restore

2. **Distro definitions**
   ```rust
   enum Distro { Ubuntu, Fedora, Arch }

   struct DistroConfig {
       name: &'static str,
       image_url: &'static str,
       package_manager: PackageManager,
       rust_install_commands: Vec<&'static str>,
   }
   ```

3. **Prerequisites check**
   - systemd-nspawn installed
   - Running as root (or with sudo)
   - Sufficient disk space

### Phase 2: Machine Setup

1. **Download cloud images**
   - Ubuntu: cloud-images.ubuntu.com
   - Fedora: Fedora Cloud raw images
   - Arch: archlinux-bootstrap tarball

2. **Bootstrap each machine**
   - Extract image
   - Install: fish, curl, git, build-essential/base-devel
   - Install Rust via rustup
   - Create test user
   - Create zygote snapshot

### Phase 3: Release Testing

1. **Test flow**
   ```
   restore_from_zygote(distro)
   start_machine(distro)
   run_in_machine(distro, "cargo install <crate>")
   run_in_machine(distro, "<binary> --version")  # Smoke test
   run_in_machine(distro, "<binary> --help")
   stop_machine(distro)
   report_results()
   ```

2. **Parallel execution**
   - Test all 3 distros simultaneously
   - Aggregate results at end

3. **Output**
   ```
   Testing r3bl-cmdr v0.0.25 on clean machines...

   ✅ ubuntu: cargo install succeeded (42s)
      giti --version: giti 0.0.25
      edi --version: edi 0.0.25

   ✅ fedora: cargo install succeeded (38s)
      giti --version: giti 0.0.25
      edi --version: edi 0.0.25

   ✅ arch: cargo install succeeded (35s)
      giti --version: giti 0.0.25
      edi --version: edi 0.0.25

   All 3 distros passed!
   ```

### Phase 4: Integration with Release Workflow

1. **Update release-guide.md TODOs**
   - `docs/release-guide.md` already has TODO comments pointing to spawny
   - Once spawny is implemented, replace the TODOs with actual instructions

2. **Add to check.fish** (optional)
   - `check.fish --spawny` runs after successful build

## Technical Details

### systemd-nspawn Command

```bash
# Start machine with bind mounts
sudo systemd-nspawn \
    --machine=ubuntu-test \
    --directory=/var/tmp/test-machines/ubuntu-test \
    --bind=/home/user/github/roc:/workspace:ro \
    --boot

# Run command in running machine
sudo machinectl shell ubuntu-test /bin/bash -c "cargo install r3bl-cmdr"

# Stop machine
sudo machinectl stop ubuntu-test
```

### Zygote Pattern

```bash
# Create zygote (after setup, before any tests)
sudo btrfs subvolume snapshot /var/tmp/test-machines/ubuntu-test /tmp/zygotes/ubuntu-test
# Or with rsync if not btrfs:
sudo rsync -a --delete /var/tmp/test-machines/ubuntu-test/ /tmp/zygotes/ubuntu-test/

# Restore from zygote (before each test)
sudo rsync -a --delete /tmp/zygotes/ubuntu-test/ /var/tmp/test-machines/ubuntu-test/
```

## Dependencies

- `systemd-nspawn` (part of systemd, available on all major distros)
- `machinectl` (part of systemd)
- Root access (or passwordless sudo for specific commands)

## Files to Create

```
build-infra/src/
├── spawny/
│   ├── mod.rs
│   ├── cli_args.rs
│   ├── nspawn.rs         # Core nspawn operations
│   ├── distro.rs         # Distro configs
│   ├── machine.rs        # Machine lifecycle
│   ├── test_runner.rs    # Test execution
│   └── setup.rs          # Machine creation
└── bin/
    └── spawny.rs
```

## Success Criteria

1. `spawny r3bl-cmdr` works on a fresh Fedora host
2. Tests run in parallel across Ubuntu, Fedora, Arch
3. Clean restore between tests (zygote pattern)
4. Clear pass/fail output with timing
5. `--local` flag tests from local build instead of crates.io

## Future Enhancements

- macOS support via Lima/Colima VMs
- Windows support via WSL2
- Integration with CI (GitHub Actions with systemd-nspawn)
- Test matrix: Rust stable vs nightly

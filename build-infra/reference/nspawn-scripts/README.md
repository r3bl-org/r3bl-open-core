# systemd-nspawn Reference Scripts

These fish scripts are copied from `~/scripts/tests/` and serve as the reference
implementation for `spawny`.

## Files

| File | Purpose |
|------|---------|
| `setup.fish` | Create Ubuntu/Fedora/Arch machines with Rust toolchain |
| `run.fish` | Run tests, shell access, machine lifecycle control |
| `teardown.fish` | Delete machines and clean up |
| `status.fish` | Show machine status |
| `lib/nspawn.fish` | Core nspawn operations (paths, state, execution) |
| `lib/ensure-prereqs.fish` | Check for required tools |
| `lib/test-helpers.fish` | Test assertion helpers |

## Usage (Original)

```bash
# One-time setup
./setup.fish

# Run tests
./run.fish                    # All tests, all distros
./run.fish ubuntu             # All tests, one distro
./run.fish shell ubuntu       # Interactive shell

# Cleanup
./teardown.fish
```

## Porting to Rust

These scripts define the behavior that `spawny` should replicate:

1. **Machine creation** (`setup.fish`)
   - Download cloud images
   - Bootstrap with packages + Rust
   - Create zygote snapshots

2. **Test execution** (`run.fish`, `lib/nspawn.fish`)
   - Restore from zygote
   - Start machine
   - Run commands inside container
   - Report results

3. **Cleanup** (`teardown.fish`)
   - Stop machines
   - Delete directories

The Rust implementation should produce identical behavior, allowing `~/scripts/tests/`
to eventually call `spawny` instead of these fish scripts.

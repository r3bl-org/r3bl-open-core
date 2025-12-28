#!/usr/bin/env fish

# Clean up systemd-nspawn containers and cached downloads
#
# Usage: ./cleanup.fish [--dry-run]

set -l options 'dry-run'
argparse $options -- $argv

# Container names (must match create-containers.fish)
set UBUNTU_CONTAINER "cmdr-ubuntu"
set FEDORA_CONTAINER "cmdr-fedora"
set ARCH_CONTAINER "cmdr-arch"

set containers \
    "/var/lib/machines/$UBUNTU_CONTAINER" \
    "/var/lib/machines/$FEDORA_CONTAINER" \
    "/var/lib/machines/$ARCH_CONTAINER"

set cached_images \
    "/tmp/Fedora-Container-Base-Generic-"'*'".tar.xz" \
    "/tmp/archlinux-bootstrap-x86_64.tar.zst"

echo "=== Cleanup r3bl-cmdr Test Containers ==="
echo ""

# Check containers
set found_containers
for container in $containers
    if test -d $container
        set found_containers $found_containers $container
        set size (sudo du -sh $container 2>/dev/null | cut -f1)
        echo "Container: $container ($size)"
    end
end

# Check cached images
set found_images
for pattern in $cached_images
    for img in (eval "ls $pattern 2>/dev/null")
        if test -f $img
            set found_images $found_images $img
            set size (du -sh $img | cut -f1)
            echo "Cached:    $img ($size)"
        end
    end
end

if test (count $found_containers) -eq 0 -a (count $found_images) -eq 0
    echo "Nothing to clean up."
    exit 0
end

echo ""

if set -q _flag_dry_run
    echo "[Dry run] Would delete:"
    for item in $found_containers $found_images
        echo "  $item"
    end
    echo ""
    echo "Run without --dry-run to actually delete."
    exit 0
end

# Confirm deletion
echo "This will delete the above containers and cached images."
read -P "Continue? [y/N] " confirm

if test "$confirm" != "y" -a "$confirm" != "Y"
    echo "Cancelled."
    exit 0
end

# Delete containers
for container in $found_containers
    echo "Removing $container..."
    sudo rm -rf $container
end

# Delete cached images
for img in $found_images
    echo "Removing $img..."
    rm -f $img
end

echo ""
echo "Cleanup complete."

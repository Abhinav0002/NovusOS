#!/bin/bash
set -euo pipefail

DISK_IMG="${1:-disk.img}"
SIZE_MB=64

echo "[mkimage] Creating ${SIZE_MB}MB FAT32 disk image: ${DISK_IMG}"

dd if=/dev/zero of="${DISK_IMG}" bs=1m count=${SIZE_MB} 2>/dev/null

# Format as FAT32 using newfs_msdos (macOS)
if command -v newfs_msdos &>/dev/null; then
    DISK_DEV=$(hdiutil attach -nomount "${DISK_IMG}")
    DISK_DEV=$(echo "${DISK_DEV}" | awk '{print $1}')
    newfs_msdos -F 32 -v CUSTOMOS "${DISK_DEV}"

    MOUNT_DIR=$(mktemp -d)
    mount -t msdos "${DISK_DEV}" "${MOUNT_DIR}"

    echo "Hello from custom-os filesystem!" > "${MOUNT_DIR}/hello.txt"
    mkdir -p "${MOUNT_DIR}/docs"
    echo "This is a test file in a subdirectory." > "${MOUNT_DIR}/docs/readme.txt"

    umount "${MOUNT_DIR}"
    hdiutil detach "${DISK_DEV}"
    rmdir "${MOUNT_DIR}"
elif command -v mkfs.fat &>/dev/null; then
    mkfs.fat -F 32 -n CUSTOMOS "${DISK_IMG}"
    if command -v mcopy &>/dev/null; then
        echo "Hello from custom-os filesystem!" | mcopy -i "${DISK_IMG}" - ::hello.txt
        mmd -i "${DISK_IMG}" ::docs
        echo "This is a test file in a subdirectory." | mcopy -i "${DISK_IMG}" - ::docs/readme.txt
    fi
else
    echo "[mkimage] No FAT32 formatting tool found (need newfs_msdos or mkfs.fat)"
    exit 1
fi

echo "[mkimage] Done: ${DISK_IMG}"

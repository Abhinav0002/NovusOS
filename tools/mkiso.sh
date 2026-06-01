#!/bin/bash
set -e

EFI_BINARY="target/aarch64-unknown-uefi/release/novusos.efi"
OUTPUT="novusos.img"

echo "Creating NovusOS bootable disk image..."

if ! [ -f "$EFI_BINARY" ]; then
    echo "Error: $EFI_BINARY not found. Run 'make build-novusos' first."
    exit 1
fi

# Create a DMG with FAT32 filesystem using hdiutil
WORK_DIR=$(mktemp -d)

hdiutil create -fs FAT32 -size 64m -volname NOVUSOS \
    -o "$WORK_DIR/novusos" 2>/dev/null

# Mount and add EFI binary
MOUNT_INFO=$(hdiutil attach "$WORK_DIR/novusos.dmg" 2>/dev/null)
MOUNT_POINT=$(echo "$MOUNT_INFO" | grep -o '/Volumes/[^"]*' | head -1)

if [ -n "$MOUNT_POINT" ]; then
    mkdir -p "$MOUNT_POINT/efi/boot"
    cp "$EFI_BINARY" "$MOUNT_POINT/efi/boot/BOOTAA64.EFI"
    printf '\\efi\\boot\\BOOTAA64.EFI\r\n' > "$MOUNT_POINT/startup.nsh"
    sync
    hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
fi

# Convert to raw format (for QEMU -cdrom and VMware)
hdiutil convert "$WORK_DIR/novusos.dmg" -format UDRW -o "$WORK_DIR/novusos_raw" 2>/dev/null || true

if [ -f "$WORK_DIR/novusos_raw.dmg" ]; then
    cp "$WORK_DIR/novusos_raw.dmg" "$OUTPUT"
else
    cp "$WORK_DIR/novusos.dmg" "$OUTPUT"
fi

rm -rf "$WORK_DIR"

SIZE=$(du -h "$OUTPUT" | cut -f1 | xargs)
echo "Created: $OUTPUT ($SIZE)"
echo ""
echo "Boot options:"
echo "  QEMU:   make run-novusos  (uses VVFAT ESP directory)"
echo "  QEMU:   make run-iso      (uses disk image)"
echo "  VMware: Attach $OUTPUT as a hard disk or CD/DVD"

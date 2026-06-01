#!/bin/bash
set -e

EFI_BINARY="target/aarch64-unknown-uefi/release/novusos.efi"
VM_DIR="$HOME/Virtual Machines.localized/NovusOS.vmwarevm"
VMDK_FILE="$VM_DIR/novusos.vmdk"
WORK_DIR=$(mktemp -d)

echo "Creating NovusOS VMware virtual disk..."

if ! [ -f "$EFI_BINARY" ]; then
    echo "Error: $EFI_BINARY not found. Run 'make build-novusos' first."
    exit 1
fi

mkdir -p "$VM_DIR"

# Create a FAT32 DMG with the EFI binary
hdiutil create -fs FAT32 -size 128m -volname NOVUSOS \
    -o "$WORK_DIR/novusos" 2>/dev/null

# Mount and copy EFI binary
MOUNT_INFO=$(hdiutil attach "$WORK_DIR/novusos.dmg" 2>/dev/null)
MOUNT_POINT=$(echo "$MOUNT_INFO" | grep -o '/Volumes/[^ ]*' | head -1)

if [ -z "$MOUNT_POINT" ]; then
    echo "Error: Could not mount disk image"
    rm -rf "$WORK_DIR"
    exit 1
fi

mkdir -p "$MOUNT_POINT/efi/boot"
cp "$EFI_BINARY" "$MOUNT_POINT/efi/boot/BOOTAA64.EFI"
printf '\\efi\\boot\\BOOTAA64.EFI\r\n' > "$MOUNT_POINT/startup.nsh"
sync

hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true

# Convert DMG to raw
RAW_IMG="$WORK_DIR/novusos.raw"
hdiutil convert "$WORK_DIR/novusos.dmg" -format UDRW -o "$WORK_DIR/novusos_rw" 2>/dev/null
if [ -f "$WORK_DIR/novusos_rw.dmg" ]; then
    dd if="$WORK_DIR/novusos_rw.dmg" of="$RAW_IMG" bs=1m 2>/dev/null
else
    dd if="$WORK_DIR/novusos.dmg" of="$RAW_IMG" bs=1m 2>/dev/null
fi

# Convert raw to VMDK
qemu-img convert -f raw -O vmdk "$RAW_IMG" "$VMDK_FILE"

rm -rf "$WORK_DIR"

SIZE=$(du -h "$VMDK_FILE" | cut -f1 | xargs)
echo "Created: $VMDK_FILE ($SIZE)"

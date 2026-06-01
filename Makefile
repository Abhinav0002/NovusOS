KERNEL_ELF = target/aarch64-unknown-none/release/kernel
KERNEL_BIN = target/kernel.bin
DISK_IMG = disk.img

QEMU = qemu-system-aarch64
QEMU_ARGS = -machine virt,gic-version=3 -cpu cortex-a72 -m 256M -nographic -serial mon:stdio
QEMU_DEVICES = -drive file=$(DISK_IMG),format=raw,if=none,id=disk0 \
	-device virtio-blk-device,drive=disk0 \
	-netdev user,id=net0 \
	-device virtio-net-device,netdev=net0

NOVUSOS_EFI = target/aarch64-unknown-uefi/release/novusos.efi
UEFI_FW = /opt/homebrew/share/qemu/edk2-aarch64-code.fd
NOVUSOS_IMG = novusos.img

.PHONY: build run run-bare debug clean clippy disk build-novusos run-novusos iso run-iso

build:
	cargo build --release

$(KERNEL_BIN): build
	rust-objcopy --strip-all -O binary $(KERNEL_ELF) $(KERNEL_BIN)

disk:
	./tools/mkimage.sh $(DISK_IMG)

run: $(KERNEL_BIN) $(DISK_IMG)
	$(QEMU) $(QEMU_ARGS) $(QEMU_DEVICES) -kernel $(KERNEL_BIN)

run-bare: $(KERNEL_BIN)
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN)

debug: $(KERNEL_BIN) $(DISK_IMG)
	$(QEMU) $(QEMU_ARGS) $(QEMU_DEVICES) -kernel $(KERNEL_BIN) -S -s

clean:
	cargo clean
	rm -f $(KERNEL_BIN)

clippy:
	cargo clippy --release

build-novusos:
	cd novusos && cargo build --release
	@mkdir -p esp/efi/boot
	cp $(NOVUSOS_EFI) esp/efi/boot/BOOTAA64.EFI

run-novusos: build-novusos
	$(QEMU) -machine virt -cpu cortex-a72 -m 256M \
		-drive if=pflash,format=raw,readonly=on,file=$(UEFI_FW) \
		-drive format=raw,file=fat:rw:esp \
		-device ramfb \
		-device qemu-xhci \
		-device usb-kbd

iso: build-novusos
	./tools/mkiso.sh

run-iso: iso
	$(QEMU) -machine virt -cpu cortex-a72 -m 256M \
		-drive if=pflash,format=raw,readonly=on,file=$(UEFI_FW) \
		-drive file=$(NOVUSOS_IMG),format=raw,if=virtio \
		-device ramfb \
		-device qemu-xhci \
		-device usb-kbd

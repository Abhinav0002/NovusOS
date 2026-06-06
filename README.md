# NovusOS — Full-Route Boot

A bare-metal AArch64 operating system that takes full hardware control by calling UEFI `ExitBootServices()`. A separate UEFI bootloader stub hands off framebuffer, memory map, and control to the kernel — no boot services remain active at runtime.

![Architecture](https://img.shields.io/badge/Architecture-AArch64-blue)
![Language](https://img.shields.io/badge/Language-Rust-orange)
![License](https://img.shields.io/badge/License-MIT-green)
![Boot](https://img.shields.io/badge/Boot-ExitBootServices-critical)

> **Branch:** `full-route-boot` — this branch implements the full-route boot path. The `main` branch retains the original UEFI application approach where boot services stay active.

---

## How It Works

NovusOS uses a **two-binary architecture**: a UEFI bootloader stub and a bare-metal kernel are compiled as separate binaries for different targets, linked by a shared boot protocol crate.

```
bootloader (aarch64-unknown-uefi)         boot-info (no_std lib)
  UEFI entry                               #[repr(C)] structs:
  → Find GOP, select best mode               BootInfo
  → Capture FramebufferInfo                   FramebufferInfo
  → Get UEFI memory map                       MemoryRegion
  → Copy kernel to 0x40080000
  → Place BootInfo at 0x40070000
  → ExitBootServices()
  → Disable MMU, flush caches
  → Jump to kernel
        ↓
kernel (aarch64-unknown-none)
  boot.S → kernel_main(x0)
  → Detect BootInfo magic in x0
  → GIC, MMU, PMM from BootInfo
  → Init framebuffer from BootInfo
  → Spawn GUI task
  → VirtIO input for keyboard
  → Interactive graphical shell
```

### Boot Sequence Detail

1. **UEFI stage** — The bootloader runs as a standard UEFI application. It opens the Graphics Output Protocol (GOP), selects the best available mode (preferring 1024x768), and records the framebuffer base address, dimensions, stride, and pixel format.

2. **Memory map** — The UEFI memory map is translated into a compact array of `MemoryRegion` structs (max 128). Each region is tagged as Usable, Reserved, Kernel, Framebuffer, or BootInfo.

3. **Kernel load** — The kernel binary is embedded in the bootloader at compile time via `include_bytes!()` and copied to the fixed load address `0x40080000`.

4. **ExitBootServices** — All UEFI boot services are terminated. From this point, the OS owns all hardware.

5. **Trampoline** — Inline assembly disables the MMU (clears SCTLR_EL1 M/C/I bits), invalidates TLB and I-cache, then branches to the kernel entry point with x0 pointing to BootInfo.

6. **Kernel init** — The kernel detects the BootInfo magic (`0x4E4F_5655_5342_4F4F`) in x0 and takes the UEFI boot path. If x0 contains a DTB pointer instead, the original DTB boot path runs unchanged.

## Project Structure

```
NovusOS/
├── boot-info/                    # Shared boot protocol crate
│   ├── Cargo.toml
│   └── src/lib.rs                # BootInfo, FramebufferInfo, MemoryRegion
├── bootloader/                   # UEFI bootloader stub
│   ├── Cargo.toml
│   ├── .cargo/config.toml        # target = aarch64-unknown-uefi
│   └── src/main.rs               # GOP → memmap → ExitBootServices → jump
├── kernel/
│   ├── Cargo.toml
│   ├── linker.ld
│   └── src/
│       ├── main.rs               # Dual boot: BootInfo vs DTB detection
│       ├── gui/                  # Graphical desktop (ported from novusos)
│       │   ├── mod.rs            # init() + gui_main_task event loop
│       │   ├── framebuffer.rs    # Pixel ops using boot-info PixelFormat
│       │   ├── font.rs           # 8x16 bitmap font (128 ASCII glyphs)
│       │   ├── console.rs        # Text console with scrolling
│       │   ├── desktop.rs        # Gradient background + status bar
│       │   ├── window.rs         # Terminal window with title bar
│       │   ├── shell.rs          # Line editor
│       │   ├── commands.rs       # 10 built-in commands (PSCI reboot/shutdown)
│       │   ├── keyboard.rs       # KeyEvent enum, delegates to VirtIO input
│       │   ├── timer.rs          # Uptime via CNTVCT_EL0
│       │   └── color.rs          # Color palette
│       ├── drivers/virtio/
│       │   ├── input.rs          # VirtIO input driver (device ID 18)
│       │   └── ...               # Existing block + net drivers
│       ├── mm/
│       │   ├── mod.rs            # init_from_boot_info() for UEFI path
│       │   ├── pmm.rs            # mark_region_used() for BootInfo regions
│       │   └── ...
│       └── ...                   # Existing kernel subsystems
├── novusos/                      # Original UEFI app (unchanged)
├── Cargo.toml                    # Workspace: kernel, novusos, boot-info, bootloader
└── Makefile                      # build-bootloader, run-full targets
```

## Quick Start

### Prerequisites

- Rust nightly toolchain
- QEMU with AArch64 support (`qemu-system-aarch64`)
- EDK2 UEFI firmware (`edk2-aarch64-code.fd`)
- `rust-objcopy` (from `llvm-tools-preview`)

```bash
rustup target add aarch64-unknown-none aarch64-unknown-uefi

# macOS
brew install qemu

# Ubuntu/Debian
sudo apt install qemu-system-arm ovmf
```

### Build & Run

```bash
# Full-route boot (ExitBootServices + bare-metal kernel + GUI)
make build-bootloader     # Builds kernel.bin, then bootloader EFI
make run-full             # Boots via UEFI → ExitBootServices → kernel GUI

# Original DTB boot (serial console, no GUI)
make run                  # Unchanged — boots kernel directly via QEMU -kernel

# Original UEFI app (boot services stay active)
make run-novusos          # Unchanged — boots novusos UEFI application
```

### Build Order

`make build-bootloader` handles the dependency chain automatically:

1. `cargo build --release` compiles the kernel ELF
2. `rust-objcopy` strips it to `target/kernel.bin`
3. The bootloader embeds `kernel.bin` via `include_bytes!()` and compiles as a UEFI PE binary
4. The resulting `BOOTAA64.EFI` is placed in `esp/efi/boot/`

## QEMU Differences

| Flag | `run` (DTB) | `run-novusos` (UEFI app) | `run-full` (full-route) |
|------|-------------|--------------------------|-------------------------|
| Machine | `virt,gic-version=3` | `virt` | `virt,gic-version=3` |
| Boot | `-kernel kernel.bin` | UEFI firmware + ESP | UEFI firmware + ESP |
| Display | Serial only | `ramfb` | `ramfb` |
| Keyboard | N/A | `qemu-xhci` + `usb-kbd` | `virtio-keyboard-device` |
| GIC | Kernel driver | UEFI handles it | Kernel driver |
| Boot services | N/A | Active at runtime | Terminated |

The key difference from `run-novusos`: `gic-version=3` is required because the kernel has its own GIC driver, and `virtio-keyboard-device` replaces USB keyboard since VirtIO is simpler to drive from bare metal.

## Key Design Decisions

**Why two binaries?** UEFI applications target `aarch64-unknown-uefi` (PE format, Microsoft x64 ABI) while the kernel targets `aarch64-unknown-none` (ELF, bare-metal ABI). These are fundamentally different compilation targets that cannot share a single binary.

**Why `include_bytes!()`?** Embedding the kernel in the bootloader at compile time avoids needing a UEFI filesystem driver to load it at runtime. The bootloader simply copies the embedded bytes to the kernel's load address.

**Why VirtIO input instead of USB?** The kernel already has VirtIO infrastructure (virtqueue, MMIO transport). A VirtIO input driver is ~200 lines. A USB HID stack (xHCI controller + USB protocol + HID parser) would be 2500+ lines with no reusable foundation.

**Why PSCI for reboot/shutdown?** After `ExitBootServices()`, UEFI runtime services may or may not be available (platform-dependent, requires runtime mapping). ARM PSCI is the standard way to reset/power-off on `virt` machines — a single `hvc` instruction.

**Dual boot detection:** The kernel checks x0 on entry. BootInfo magic is a 64-bit value (`0x4E4F_5655_5342_4F4F`). DTB magic is `0xD00DFEED` (32-bit at offset 0). No collision possible, so the same kernel binary boots correctly in both modes.

## Shell Commands

Once the GUI is running, the terminal accepts the same commands as the UEFI desktop:

```
novus> help
  help          Show this help message
  ver           Show version information
  echo <text>   Print text
  clear         Clear the terminal
  mem           Show memory information
  uptime        Show system uptime
  cpuinfo       Show CPU information
  color r g b   Change desktop background
  reboot        Restart the system (PSCI)
  shutdown      Power off the system (PSCI)
```

## Debugging

```bash
# Serial output from bootloader trampoline (writes 'B', 'I', 'K' to UART)
make run-full

# GDB: break at kernel entry
qemu-system-aarch64 ... -S -s
# In another terminal:
gdb-multiarch target/aarch64-unknown-none/release/kernel -ex "target remote :1234" -ex "b *0x40080000"

# QEMU exception tracing
qemu-system-aarch64 ... -d int,cpu_reset
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

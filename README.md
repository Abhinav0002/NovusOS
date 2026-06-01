# NovusOS

A bare-metal operating system for AArch64 (ARM64) featuring both a traditional kernel with full OS subsystems and a graphical UEFI desktop environment.

![Architecture](https://img.shields.io/badge/Architecture-AArch64-blue)
![Language](https://img.shields.io/badge/Language-Rust-orange)
![License](https://img.shields.io/badge/License-MIT-green)
![Boot](https://img.shields.io/badge/Boot-UEFI%20%2B%20Raw-informational)

---

## Overview

NovusOS is a dual-component operating system project:

- **Kernel** — A monolithic bare-metal kernel running on QEMU `virt` with Cortex-A72. Implements boot initialization, exception handling (GICv3), MMU with 4-level page tables, heap allocation, preemptive multitasking, VirtIO block/network drivers, FAT32 + RamFS filesystems, a TCP/IP network stack, system calls, and userspace process execution.

- **NovusOS Desktop** — A graphical UEFI application that boots from an EFI System Partition on both QEMU and VMware Fusion. Features a pixel-level framebuffer with embedded bitmap font, gradient desktop with status bar, bordered terminal window, and an interactive command shell.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     NovusOS Project                             │
├─────────────────────────────┬───────────────────────────────────┤
│      Bare-Metal Kernel      │       UEFI Graphical Desktop      │
│                             │                                   │
│  ┌───────────────────────┐  │  ┌─────────────────────────────┐  │
│  │ Userspace (EL0)       │  │  │ GUI Desktop                 │  │
│  │  └─ init process      │  │  │  ├─ Gradient background     │  │
│  ├───────────────────────┤  │  │  ├─ Status bar (uptime/RAM)  │  │
│  │ Syscall Interface     │  │  │  └─ Terminal window          │  │
│  ├───────────────────────┤  │  ├─────────────────────────────┤  │
│  │ Network Stack         │  │  │ Interactive Shell            │  │
│  │  TCP/UDP/ICMP/ARP/IP  │  │  │  └─ 10 built-in commands    │  │
│  ├───────────────────────┤  │  ├─────────────────────────────┤  │
│  │ Filesystem Layer      │  │  │ Console (fmt::Write)        │  │
│  │  FAT32 + RamFS + VFS  │  │  │  └─ 8x16 bitmap font       │  │
│  ├───────────────────────┤  │  ├─────────────────────────────┤  │
│  │ Device Drivers        │  │  │ UEFI Services               │  │
│  │  VirtIO Block + Net   │  │  │  GOP / Input / Memory       │  │
│  ├───────────────────────┤  │  └─────────────────────────────┘  │
│  │ Scheduler             │  │                                   │
│  │  Preemptive RR        │  │  Target: aarch64-unknown-uefi    │
│  ├───────────────────────┤  │  Boot: EFI System Partition       │
│  │ Memory Management     │  │  Platforms: QEMU + VMware Fusion  │
│  │  PMM + VMM + Heap     │  │                                   │
│  ├───────────────────────┤  │                                   │
│  │ Exceptions + GICv3    │  │                                   │
│  ├───────────────────────┤  │                                   │
│  │ Boot (Assembly)       │  │                                   │
│  └───────────────────────┘  │                                   │
│                             │                                   │
│  Target: aarch64-unknown-   │                                   │
│          none               │                                   │
│  Boot: QEMU -kernel         │                                   │
└─────────────────────────────┴───────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust nightly toolchain (`rustup` will auto-install from `rust-toolchain.toml`)
- QEMU with AArch64 support (`qemu-system-aarch64`)
- EDK2 UEFI firmware (`edk2-aarch64-code.fd`)
- `rust-objcopy` (from `llvm-tools-preview`)

```bash
# Install Rust targets
rustup target add aarch64-unknown-none aarch64-unknown-uefi

# macOS (Homebrew)
brew install qemu

# Ubuntu/Debian
sudo apt install qemu-system-arm ovmf
```

### Build & Run

```bash
# ── Bare-Metal Kernel ──
make build                   # Build kernel ELF
make run                     # Boot kernel in QEMU (serial console)

# ── NovusOS Graphical Desktop ──
make build-novusos           # Build UEFI application
make run-novusos             # Boot in QEMU with UEFI + GUI

# ── Create Bootable Image ──
make iso                     # Create novusos.img for VMware
make run-iso                 # Boot from disk image in QEMU
```

## Kernel Features

The bare-metal kernel implements 10 phases of OS development:

| Phase | Component | Description |
|-------|-----------|-------------|
| 1 | Boot + UART | Assembly stub, BSS init, PL011 serial driver |
| 2 | Exceptions | Vector table, GICv3, ARM generic timer (10ms tick) |
| 3 | Memory | Bitmap PMM, 4-level page tables, MMU with identity + higher-half mapping |
| 4 | Heap | Linked-list allocator with coalescing, enables `alloc` crate |
| 5 | Scheduler | Preemptive round-robin with assembly context switch |
| 6 | Drivers | VirtIO MMIO transport, split virtqueue, block + network drivers |
| 7 | Filesystem | VFS layer, FAT32 read-only driver, RAM filesystem |
| 8 | Network | Ethernet, ARP, IPv4, ICMP, UDP, TCP state machine |
| 9 | Syscalls | SVC-based interface: write, read, open, close, yield, getpid, sleep |
| 10 | Userspace | ELF loader, user page tables, EL1→EL0 transition via `eret` |

## NovusOS Desktop

The graphical desktop runs as a UEFI application (boot services remain active):

| Component | Description |
|-----------|-------------|
| Framebuffer | GOP-based, volatile pixel writes, BGR/RGB format detection |
| Font | Embedded 8x16 bitmap, 128 ASCII glyphs, bit-blit rendering |
| Console | `fmt::Write` implementation with scrolling and word wrap |
| Keyboard | UEFI `SimpleTextInput` polling via `with_stdin()` |
| Desktop | Vertical gradient background, 24px status bar with live uptime |
| Window | Terminal with title bar, border, shadow, dark content area |
| Shell | Line editor with 10 built-in commands |

### Shell Commands

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
  reboot        Restart the system
  shutdown      Power off the system
```

## Project Structure

```
NovusOS/
├── Cargo.toml                  # Workspace configuration
├── Makefile                    # Build and run targets
├── rust-toolchain.toml         # Nightly toolchain + targets
├── .cargo/config.toml          # Kernel cross-compilation
├── kernel/
│   ├── Cargo.toml
│   ├── linker.ld               # Kernel memory layout
│   ├── init.elf                # Embedded userspace binary
│   └── src/
│       ├── main.rs             # Kernel entry point
│       ├── console.rs          # print!/println! macros
│       ├── arch/aarch64/       # Boot, exceptions, GIC, timer
│       ├── mm/                 # PMM, VMM, heap allocator
│       ├── sync/               # SpinLock with interrupt masking
│       ├── sched/              # Tasks, scheduler, process loader
│       ├── drivers/            # UART, VirtIO block/net
│       ├── fs/                 # VFS, FAT32, RamFS
│       ├── net/                # TCP/IP stack
│       └── syscall/            # System call interface
├── novusos/
│   ├── Cargo.toml
│   ├── .cargo/config.toml      # UEFI target override
│   └── src/
│       ├── main.rs             # UEFI entry + event loop
│       ├── framebuffer.rs      # GOP pixel operations
│       ├── font.rs             # 8x16 bitmap font data
│       ├── console.rs          # Graphical text console
│       ├── keyboard.rs         # Input polling
│       ├── desktop.rs          # Background + status bar
│       ├── window.rs           # Terminal window
│       ├── shell.rs            # Line editor
│       ├── commands.rs         # Built-in commands
│       ├── timer.rs            # Uptime (CNTVCT_EL0)
│       ├── memory.rs           # UEFI memory map
│       └── color.rs            # Color palette
├── userspace/init/             # EL0 init process
├── tools/                      # Build scripts
└── docs/                       # Technical documentation
```

## VMware Fusion (Apple Silicon)

1. Build the bootable image: `make iso`
2. Create a new VM → **Other 64-bit ARM**
3. Attach `novusos.img` as the hard disk
4. Boot — UEFI firmware auto-detects `EFI/BOOT/BOOTAA64.EFI`

## Documentation

Full technical documentation is available in the [`docs/`](docs/) directory:

- [Custom-OS Technical Documentation](docs/Custom-OS-Technical-Documentation.pdf) — 92-page reference covering all 10 kernel phases
- [NovusOS Technical Documentation](docs/NovusOS-Technical-Documentation.pdf) — 39-page reference covering the graphical UEFI desktop

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

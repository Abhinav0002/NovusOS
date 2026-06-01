# Architecture

This document provides a technical deep-dive into the NovusOS project architecture.

## System Overview

NovusOS consists of two independent components that share a Cargo workspace:

```
                    ┌──────────────────────────────┐
                    │     Cargo Workspace Root      │
                    └──────┬───────────┬────────────┘
                           │           │
              ┌────────────▼──┐   ┌────▼────────────┐
              │    kernel/    │   │    novusos/      │
              │               │   │                  │
              │ aarch64-      │   │ aarch64-         │
              │ unknown-none  │   │ unknown-uefi     │
              │               │   │                  │
              │ Raw binary    │   │ PE/COFF binary   │
              │ QEMU -kernel  │   │ EFI boot         │
              └───────────────┘   └──────────────────┘
```

## Kernel Architecture

The kernel is a monolithic design running entirely at Exception Level 1 (EL1). All subsystems share a single address space.

### Boot Sequence

```
QEMU loads kernel.bin at 0x40080000
         │
         ▼
    _start (boot.S)
         │
         ├── Save DTB pointer (x0)
         ├── Park secondary cores (MPIDR_EL1)
         ├── Zero BSS section
         ├── Enable FP/SIMD (CPACR_EL1.FPEN = 0b11)
         ├── Set stack pointer (64KB boot stack)
         │
         ▼
    kernel_main (main.rs)
         │
         ├── Phase 1: UART init → serial console
         ├── Phase 2: Exception vectors → GICv3 → Timer (10ms tick)
         ├── Phase 3: PMM → VMM → MMU enable
         ├── Phase 4: Heap allocator (1MB from PMM)
         ├── Phase 5: Scheduler init
         ├── Phase 6: VirtIO device probe → block + net drivers
         ├── Phase 7: FAT32 mount → RamFS mount
         ├── Phase 8: Network stack init (IP: 10.0.2.15)
         ├── Phase 9: Syscall test
         ├── Phase 10: Load init.elf → jump to EL0
         │
         ▼
    Idle loop (WFI with scheduler)
```

### Memory Layout

```
Physical Address Space (256 MB):
┌─────────────────────────────────────────┐ 0x50000000
│                                         │
│              Free Pages                 │
│          (managed by PMM)               │
│                                         │
├─────────────────────────────────────────┤ ~0x400CA000
│         Kernel Heap (1 MB)              │
├─────────────────────────────────────────┤ ~0x400C0000
│      Kernel Image (.text + .data)       │
├─────────────────────────────────────────┤ 0x40080000
│           Reserved / DTB                │
├─────────────────────────────────────────┤ 0x40000000
│                                         │
│           Device MMIO                   │
│   GIC: 0x08000000  UART: 0x09000000    │
│   VirtIO: 0x0A000000 (32 slots)        │
│                                         │
└─────────────────────────────────────────┘ 0x00000000

Virtual Address Space (48-bit, 4KB granule):
┌─────────────────────────────────────────┐
│  0xFFFF_0000_4008_0000+                 │
│       Kernel (higher-half via TTBR1)    │
├─────────────────────────────────────────┤
│  0xFFFF_0000_0000_0000+                 │
│       MMIO mirror (higher-half)         │
├─────────────────────────────────────────┤
│  0x7FFF_C000 - 0x8000_0000             │
│       User stack (16 KB, TTBR0)         │
├─────────────────────────────────────────┤
│  0x0040_0000+                           │
│       User code (ELF segments, TTBR0)   │
├─────────────────────────────────────────┤
│  0x0000_0000 - 0x4FFF_FFFF             │
│       Identity-mapped (boot, TTBR0)     │
└─────────────────────────────────────────┘
```

### Interrupt Flow

```
Hardware IRQ
    │
    ▼
GICv3 Distributor (GICD)
    │
    ▼
GICv3 Redistributor (GICR) ──► CPU Interface (ICC registers)
    │
    ▼
Exception Vector Table (VBAR_EL1)
    │
    ├── IRQ from current EL ──► handle_irq()
    │                               │
    │                               ├── Read IAR (interrupt ID)
    │                               ├── Write EOI (before dispatch!)
    │                               │
    │                               ├── INTID 27 (Timer) ──► re-arm + schedule()
    │                               ├── INTID 33 (UART)  ──► read char
    │                               └── INTID 48+ (VirtIO) ──► driver handler
    │
    └── SVC from lower EL ──► handle_sync()
                                    │
                                    └── ESR_EL1.EC == 0x15 ──► syscall dispatch
```

### Scheduler Design

```
Timer IRQ (every 10ms)
    │
    ▼
schedule()
    │
    ├── Current task → move to back of run queue
    ├── Next Ready task → pop from front
    │
    ▼
context_switch(prev, next)    [assembly]
    │
    ├── Save x19-x30, SP to prev->context
    ├── Load x19-x30, SP from next->context
    └── ret (resumes via loaded LR)

New task startup:
    context_switch restores LR = task_entry_trampoline
         │
         ▼
    task_entry_trampoline     [assembly]
         │
         ├── msr DAIFClr, #0b0010   (unmask IRQs)
         └── br x19                  (call task function)
```

### Network Stack Layers

```
┌──────────────────────────────────────┐
│            Application               │
│    TCP echo (8080) / UDP echo (7)    │
├──────────────────────────────────────┤
│      TCP                │    UDP     │
│  State machine:         │  Stateless │
│  LISTEN → ESTABLISHED   │  port map  │
│  → FIN_WAIT → CLOSED    │            │
├──────────────────────────────────────┤
│              ICMP                    │
│         Echo reply                   │
├──────────────────────────────────────┤
│              IPv4                    │
│  Checksum, TTL=64, routing           │
├──────────────────────────────────────┤
│              ARP                     │
│  BTreeMap cache, request/reply       │
├──────────────────────────────────────┤
│           Ethernet                   │
│  Frame parse, EtherType dispatch     │
├──────────────────────────────────────┤
│        VirtIO-net driver             │
│  RX/TX virtqueues, MAC from config   │
└──────────────────────────────────────┘
```

## NovusOS Desktop Architecture

The UEFI desktop runs as a boot services application — it never calls `ExitBootServices()`.

### Execution Model

```
UEFI Firmware
    │
    ├── Load PE/COFF from ESP: EFI/BOOT/BOOTAA64.EFI
    ├── Call efi_main()
    │
    ▼
NovusOS Application
    │
    ├── uefi::helpers::init()     (allocator, logger, panic handler)
    ├── Locate GOP protocol       (framebuffer access)
    ├── Set video mode             (1024x768 preferred)
    ├── Draw desktop               (gradient + status bar + window)
    ├── Initialize shell
    │
    ▼
Event Loop
    │
    ├── poll_key() via with_stdin()
    │       │
    │       ├── Char → echo to console
    │       ├── Enter → execute command
    │       └── Backspace → erase char
    │
    ├── tick counter → update status bar (uptime)
    │
    └── stall(1ms) → repeat
```

### Display Compositing

```
┌─ Screen ──────────────────────────────────────────┐
│ ┌─ Status Bar (24px, STATUS_BG) ───────────────┐  │
│ │ NovusOS v1.0  |  RAM: 256 MB  |  Uptime: ... │  │
│ └──────────────────────────────────────────────-┘  │
│                                                    │
│   ┌─ Shadow (3px offset, BLACK) ──────────────┐   │
│   │                                            │   │
│ ┌─┤─ Terminal Window ────────────────────────┐ │   │
│ │ │ Title Bar (TITLE_BAR)     "Terminal" [x]│ │   │
│ │ ├──────────────────────────────────────────┤ │   │
│ │ │                                          │ │   │
│ │ │  Console content area (CONTENT_BG)       │ │   │
│ │ │  8x16 font rendering                    │ │   │
│ │ │  novus> _                                │ │   │
│ │ │                                          │ │   │
│ │ └──────────────────────────────────────────┘ │   │
│ │   └──────────────────────────────────────────┘   │
│                                                    │
│         Gradient Background (DARK_BLUE → MED_BLUE) │
└────────────────────────────────────────────────────┘
```

### Module Dependency Graph

```
main.rs
  ├── framebuffer.rs ◄── color.rs
  ├── font.rs ◄── framebuffer.rs, color.rs
  ├── console.rs ◄── font.rs, framebuffer.rs, color.rs
  ├── keyboard.rs (UEFI SimpleTextInput)
  ├── desktop.rs ◄── font.rs, framebuffer.rs, color.rs
  ├── window.rs ◄── font.rs, framebuffer.rs, color.rs
  ├── shell.rs ◄── commands.rs, console.rs, keyboard.rs
  ├── commands.rs ◄── console.rs, timer.rs, memory.rs, desktop.rs
  ├── timer.rs (CNTVCT_EL0 / CNTFRQ_EL0)
  └── memory.rs (UEFI boot::memory_map)
```

## Build System

```
make build          →  cargo build --release (kernel)
                       rust-objcopy → target/kernel.bin

make build-novusos  →  cd novusos && cargo build --release
                       cp novusos.efi → esp/efi/boot/BOOTAA64.EFI

make run            →  qemu-system-aarch64 -kernel kernel.bin
                       (serial console, no graphics)

make run-novusos    →  qemu-system-aarch64 -pflash edk2-aarch64-code.fd
                       -drive fat:rw:esp -device ramfb -device usb-kbd
                       (UEFI boot, graphical output)
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Monolithic kernel | Simplicity for a learning/demo OS; all subsystems share address space |
| GICv3 over GICv2 | ICC system registers are cleaner than MMIO-only GICv2 interface |
| VirtIO legacy (v1) | QEMU virt machine presents v1 devices by default |
| UEFI app (not ExitBootServices) | Avoids ~3000 lines of USB HID + interrupt controller driver code |
| Embedded bitmap font | No filesystem dependency; constant-time glyph lookup |
| Integer gradient | No floating-point dependency; works on any AArch64 without FPU config |
| Single-threaded desktop | UEFI boot services are not reentrant; polling model is sufficient |

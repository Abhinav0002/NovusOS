# Changelog

All notable changes to this project are documented in this file.

## [1.0.0] - 2025-05-23

### Kernel

#### Added
- Assembly boot stub with DTB save, BSS zeroing, FP/SIMD enable, and stack setup
- PL011 UART driver with `fmt::Write` for serial console output
- AArch64 exception vector table with 16 branch-to-stub handlers
- GICv3 interrupt controller: Distributor, Redistributor, ICC system registers
- ARM generic timer with 10ms tick interval for preemptive scheduling
- Bitmap physical memory manager (65,536 pages / 256 MB)
- 4-level page table VMM with 48-bit VA, 4KB granule, identity + higher-half mapping
- Linked-list heap allocator with block coalescing (1 MB)
- SpinLock with DAIF interrupt masking
- Preemptive round-robin scheduler with assembly context switch
- Task entry trampoline for IRQ unmasking on new task startup
- VirtIO MMIO transport with device discovery (32 slots)
- Split virtqueue implementation (legacy v1 + modern v2)
- VirtIO block driver with 3-descriptor chain I/O
- VirtIO network driver with pre-populated RX buffers
- VFS abstraction with longest-prefix mount matching
- FAT32 read-only driver with LFN support
- RAM filesystem mounted at /tmp
- Ethernet frame layer with EtherType dispatch
- ARP with BTreeMap-based cache
- IPv4 with internet checksum and subnet routing
- ICMP echo reply
- UDP with port dispatch table
- TCP state machine (LISTEN through CLOSED)
- SVC-based system call interface (write, read, open, close, yield, getpid, sleep)
- ELF loader with user page table creation
- EL1-to-EL0 transition via eret
- Userspace init process with SVC wrappers

### NovusOS Desktop

#### Added
- UEFI application entry point with GOP framebuffer initialization
- Video mode selection (1024x768 preferred, highest-resolution fallback)
- Framebuffer driver with volatile writes, stride-aware addressing, BGR/RGB detection
- Embedded 8x16 bitmap font with 128 ASCII glyphs
- Graphical console with `fmt::Write`, scrolling, word wrap, and tab alignment
- Keyboard input via UEFI SimpleTextInput polling
- Interactive shell with 256-byte line buffer and backspace editing
- Desktop with vertical gradient background (integer interpolation)
- 24px status bar with live uptime, RAM, and resolution display
- Terminal window with title bar, border, close button, and shadow
- Built-in commands: help, ver, echo, clear, mem, uptime, cpuinfo, color, reboot, shutdown
- Uptime tracking via AArch64 CNTVCT_EL0 / CNTFRQ_EL0
- UEFI memory map query for RAM reporting
- CPU identification via MIDR_EL1 decode
- Bootable image creation scripts (mkiso.sh, mkvmdk.sh)
- QEMU UEFI boot with EDK2 firmware and VVFAT ESP
- VMware Fusion ARM64 compatibility

### Documentation
- 92-page kernel technical documentation (PDF)
- 39-page NovusOS desktop technical documentation (PDF)
- ReportLab-based PDF generator scripts

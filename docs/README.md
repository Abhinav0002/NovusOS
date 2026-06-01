# NovusOS Technical Documentation

This directory contains comprehensive technical documentation for the NovusOS project.

## Documents

### [Custom-OS Technical Documentation](Custom-OS-Technical-Documentation.pdf)
Complete reference for the bare-metal AArch64 kernel covering all 10 implementation phases:
- Boot sequence and assembly initialization
- Exception handling with GICv3 interrupt controller
- 4-level page table MMU with identity and higher-half mapping
- Bitmap physical memory manager and linked-list heap allocator
- Preemptive round-robin scheduler with assembly context switch
- VirtIO MMIO device drivers (block storage + networking)
- Virtual filesystem with FAT32 and RAM filesystem
- TCP/IP network stack (Ethernet, ARP, IPv4, ICMP, UDP, TCP)
- SVC-based system call interface
- ELF loader and userspace process execution
- Detailed bug analysis with root causes and fixes

### [NovusOS Technical Documentation](NovusOS-Technical-Documentation.pdf)
Complete reference for the UEFI graphical desktop environment:
- UEFI application architecture and boot protocol
- GOP framebuffer initialization and pixel operations
- Embedded 8x16 bitmap font and graphical console
- Keyboard input via UEFI SimpleTextInput protocol
- GUI desktop with gradient rendering and status bar
- Terminal window with interactive command shell
- System information commands (memory, CPU, uptime)
- Bootable image creation and VMware Fusion setup

## Regenerating Documentation

The documentation PDFs are generated from source using ReportLab:

```bash
pip install reportlab
python3 tools/generate_pdf.py           # Kernel documentation
python3 tools/generate_novusos_pdf.py   # NovusOS documentation
```

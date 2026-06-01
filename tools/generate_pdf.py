#!/usr/bin/env python3
"""
Generate a comprehensive technical documentation PDF for custom-os.
"""

import os
import sys
from datetime import datetime

from reportlab.lib.pagesizes import letter
from reportlab.lib.units import inch, cm
from reportlab.lib.colors import HexColor, black, white, grey
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, Preformatted, KeepTogether, HRFlowable, Image,
    ListFlowable, ListItem
)
from reportlab.platypus.frames import Frame
from reportlab.platypus.doctemplate import PageTemplate, BaseDocTemplate
from reportlab.lib.fonts import addMapping
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont

# ─── Colors ───────────────────────────────────────────────────────────
DARK_BG      = HexColor("#1a1a2e")
ACCENT       = HexColor("#0f3460")
HIGHLIGHT    = HexColor("#e94560")
CODE_BG      = HexColor("#f4f4f8")
CODE_BORDER  = HexColor("#d0d0d8")
HEADING_CLR  = HexColor("#16213e")
SUBHEAD_CLR  = HexColor("#0f3460")
TEXT_CLR     = HexColor("#2c3e50")
LIGHT_GREY   = HexColor("#ecf0f1")
TABLE_HEADER = HexColor("#2c3e50")
TABLE_ALT    = HexColor("#f8f9fa")
RUST_ORANGE  = HexColor("#ce422b")

# ─── Project Root ─────────────────────────────────────────────────────
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KERNEL_SRC   = os.path.join(PROJECT_ROOT, "kernel", "src")
OUTPUT_PDF   = os.path.join(PROJECT_ROOT, "Custom-OS-Technical-Documentation.pdf")

# ─── Styles ───────────────────────────────────────────────────────────
styles = getSampleStyleSheet()

style_title = ParagraphStyle(
    "DocTitle", parent=styles["Title"],
    fontSize=32, leading=38, textColor=HEADING_CLR,
    spaceAfter=6, alignment=TA_CENTER,
    fontName="Helvetica-Bold"
)

style_subtitle = ParagraphStyle(
    "DocSubtitle", parent=styles["Normal"],
    fontSize=16, leading=20, textColor=SUBHEAD_CLR,
    spaceAfter=20, alignment=TA_CENTER,
    fontName="Helvetica"
)

style_h1 = ParagraphStyle(
    "H1", parent=styles["Heading1"],
    fontSize=22, leading=28, textColor=HEADING_CLR,
    spaceBefore=24, spaceAfter=12,
    fontName="Helvetica-Bold",
    borderWidth=0, borderPadding=0,
)

style_h2 = ParagraphStyle(
    "H2", parent=styles["Heading2"],
    fontSize=17, leading=22, textColor=SUBHEAD_CLR,
    spaceBefore=18, spaceAfter=8,
    fontName="Helvetica-Bold"
)

style_h3 = ParagraphStyle(
    "H3", parent=styles["Heading3"],
    fontSize=13, leading=17, textColor=ACCENT,
    spaceBefore=12, spaceAfter=6,
    fontName="Helvetica-Bold"
)

style_body = ParagraphStyle(
    "Body", parent=styles["Normal"],
    fontSize=10.5, leading=15, textColor=TEXT_CLR,
    spaceAfter=8, alignment=TA_JUSTIFY,
    fontName="Helvetica"
)

style_body_sm = ParagraphStyle(
    "BodySmall", parent=style_body,
    fontSize=9.5, leading=13,
)

style_code = ParagraphStyle(
    "Code", parent=styles["Code"],
    fontSize=7.5, leading=10,
    fontName="Courier",
    backColor=CODE_BG,
    borderColor=CODE_BORDER,
    borderWidth=0.5,
    borderPadding=6,
    spaceBefore=4, spaceAfter=8,
    leftIndent=12, rightIndent=12,
)

style_code_title = ParagraphStyle(
    "CodeTitle", parent=styles["Normal"],
    fontSize=9, leading=12,
    fontName="Helvetica-BoldOblique",
    textColor=ACCENT,
    spaceBefore=8, spaceAfter=2,
    leftIndent=12,
)

style_bullet = ParagraphStyle(
    "BulletItem", parent=style_body,
    leftIndent=24, bulletIndent=12,
    spaceBefore=2, spaceAfter=2,
)

style_toc = ParagraphStyle(
    "TOC", parent=styles["Normal"],
    fontSize=12, leading=18, textColor=SUBHEAD_CLR,
    leftIndent=20, spaceBefore=4, spaceAfter=4,
)

style_toc_sub = ParagraphStyle(
    "TOCSub", parent=style_toc,
    fontSize=10.5, leading=15, textColor=TEXT_CLR,
    leftIndent=44,
)

style_caption = ParagraphStyle(
    "Caption", parent=styles["Normal"],
    fontSize=9, leading=12, textColor=grey,
    alignment=TA_CENTER, spaceAfter=12,
    fontName="Helvetica-Oblique"
)

style_footer = ParagraphStyle(
    "Footer", parent=styles["Normal"],
    fontSize=8, textColor=grey, alignment=TA_RIGHT,
)

# ─── Helper Functions ─────────────────────────────────────────────────
def heading1(text):
    return [
        HRFlowable(width="100%", thickness=1.5, color=ACCENT, spaceBefore=6, spaceAfter=0),
        Paragraph(text, style_h1),
    ]

def heading2(text):
    return [Paragraph(text, style_h2)]

def heading3(text):
    return [Paragraph(text, style_h3)]

def body(text):
    return [Paragraph(text, style_body)]

def body_sm(text):
    return [Paragraph(text, style_body_sm)]

def bullet(text):
    return [Paragraph(f"&bull;  {text}", style_bullet)]

def code_block(code, title=None):
    elements = []
    if title:
        elements.append(Paragraph(title, style_code_title))
    safe = (code.replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;"))
    elements.append(Preformatted(safe, style_code))
    return elements

def read_file(relative_path):
    full = os.path.join(PROJECT_ROOT, relative_path)
    try:
        with open(full, "r") as f:
            return f.read()
    except Exception as e:
        return f"[Could not read: {e}]"

def make_table(headers, rows, col_widths=None):
    data = [headers] + rows
    t = Table(data, colWidths=col_widths, repeatRows=1)
    style_cmds = [
        ("BACKGROUND", (0, 0), (-1, 0), TABLE_HEADER),
        ("TEXTCOLOR", (0, 0), (-1, 0), white),
        ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
        ("FONTSIZE", (0, 0), (-1, 0), 10),
        ("FONTSIZE", (0, 1), (-1, -1), 9),
        ("FONTNAME", (0, 1), (-1, -1), "Helvetica"),
        ("ALIGN", (0, 0), (-1, -1), "LEFT"),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("GRID", (0, 0), (-1, -1), 0.5, grey),
        ("TOPPADDING", (0, 0), (-1, -1), 5),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
        ("LEFTPADDING", (0, 0), (-1, -1), 6),
        ("RIGHTPADDING", (0, 0), (-1, -1), 6),
    ]
    for i in range(1, len(data)):
        if i % 2 == 0:
            style_cmds.append(("BACKGROUND", (0, i), (-1, i), TABLE_ALT))
    t.setStyle(TableStyle(style_cmds))
    return [t, Spacer(1, 8)]


# ─── Page Number Footer ──────────────────────────────────────────────
def add_page_number(canvas, doc):
    canvas.saveState()
    canvas.setFont("Helvetica", 8)
    canvas.setFillColor(grey)
    canvas.drawRightString(
        letter[0] - 0.75 * inch, 0.5 * inch,
        f"custom-os Technical Documentation  |  Page {doc.page}"
    )
    canvas.drawString(
        0.75 * inch, 0.5 * inch,
        "Confidential - Architecture & Implementation Reference"
    )
    # Top line
    canvas.setStrokeColor(ACCENT)
    canvas.setLineWidth(0.5)
    canvas.line(0.75*inch, letter[1] - 0.6*inch, letter[0] - 0.75*inch, letter[1] - 0.6*inch)
    # Bottom line
    canvas.line(0.75*inch, 0.7*inch, letter[0] - 0.75*inch, 0.7*inch)
    canvas.restoreState()

def first_page(canvas, doc):
    pass  # No header/footer on cover page


# ═══════════════════════════════════════════════════════════════════════
#  BUILD THE DOCUMENT
# ═══════════════════════════════════════════════════════════════════════
def build():
    doc = SimpleDocTemplate(
        OUTPUT_PDF,
        pagesize=letter,
        leftMargin=0.75*inch,
        rightMargin=0.75*inch,
        topMargin=0.85*inch,
        bottomMargin=0.85*inch,
        title="custom-os: AArch64 Operating System - Technical Documentation",
        author="Abhishek Bhatia",
        subject="Operating System Design and Implementation",
    )

    story = []

    # ═══════════════════════════════════════════════════════════════════
    # COVER PAGE
    # ═══════════════════════════════════════════════════════════════════
    story.append(Spacer(1, 2*inch))
    story.append(Paragraph("custom-os", ParagraphStyle(
        "CoverTitle", parent=style_title, fontSize=44, leading=52,
        textColor=HEADING_CLR, alignment=TA_CENTER
    )))
    story.append(Spacer(1, 8))
    story.append(HRFlowable(width="40%", thickness=3, color=HIGHLIGHT,
                            spaceBefore=0, spaceAfter=12))
    story.append(Paragraph(
        "A Bare-Metal Operating System for AArch64",
        ParagraphStyle("CoverSub1", parent=style_subtitle, fontSize=18, leading=24)
    ))
    story.append(Spacer(1, 6))
    story.append(Paragraph(
        "Complete Technical Documentation",
        ParagraphStyle("CoverSub2", parent=style_subtitle, fontSize=14,
                       textColor=TEXT_CLR)
    ))
    story.append(Spacer(1, 1.5*inch))

    cover_info = [
        ["Language", "Rust (nightly, #![no_std])"],
        ["Architecture", "AArch64 (ARM64) / ARMv8-A"],
        ["Kernel Type", "Monolithic"],
        ["Target Platform", "QEMU virt machine (qemu-system-aarch64)"],
        ["CPU Model", "Cortex-A72"],
        ["RAM", "256 MB"],
        ["Author", "Abhishek Bhatia"],
        ["Date", datetime.now().strftime("%B %d, %Y")],
        ["Lines of Code", "~3,500+ (Rust + Assembly)"],
    ]
    t = Table(cover_info, colWidths=[2.2*inch, 4*inch])
    t.setStyle(TableStyle([
        ("FONTNAME", (0, 0), (0, -1), "Helvetica-Bold"),
        ("FONTNAME", (1, 0), (1, -1), "Helvetica"),
        ("FONTSIZE", (0, 0), (-1, -1), 11),
        ("TEXTCOLOR", (0, 0), (0, -1), ACCENT),
        ("TEXTCOLOR", (1, 0), (1, -1), TEXT_CLR),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
        ("LINEBELOW", (0, 0), (-1, -2), 0.3, LIGHT_GREY),
        ("ALIGN", (0, 0), (-1, -1), "LEFT"),
    ]))
    story.append(t)
    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # TABLE OF CONTENTS
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("Table of Contents"))
    toc_items = [
        ("1.", "Executive Summary"),
        ("2.", "System Architecture"),
        ("  2.1", "Hardware Target & QEMU virt Memory Map"),
        ("  2.2", "Kernel Virtual Address Layout"),
        ("  2.3", "Project Structure"),
        ("  2.4", "Build Infrastructure"),
        ("3.", "Phase 1: Boot Sequence & UART Console"),
        ("  3.1", "Assembly Boot Stub (boot.S)"),
        ("  3.2", "Linker Script"),
        ("  3.3", "PL011 UART Driver"),
        ("  3.4", "Console Macros"),
        ("4.", "Phase 2: Exception Handling, GICv3 & Timer"),
        ("  4.1", "Exception Vector Table"),
        ("  4.2", "GICv3 Interrupt Controller"),
        ("  4.3", "ARM Generic Timer"),
        ("5.", "Phase 3: Memory Management"),
        ("  5.1", "Physical Memory Manager (Bitmap Allocator)"),
        ("  5.2", "Virtual Memory Manager (4-Level Page Tables)"),
        ("  5.3", "MMU Initialization"),
        ("6.", "Phase 4: Kernel Heap Allocator"),
        ("7.", "Phase 5: Synchronization & Preemptive Scheduler"),
        ("  7.1", "SpinLock with Interrupt Masking"),
        ("  7.2", "Task Structure & Context"),
        ("  7.3", "Round-Robin Scheduler"),
        ("  7.4", "Assembly Context Switch"),
        ("8.", "Phase 6: Device Drivers"),
        ("  8.1", "VirtIO MMIO Transport"),
        ("  8.2", "Split Virtqueue Implementation"),
        ("  8.3", "VirtIO Block Driver"),
        ("  8.4", "VirtIO Network Driver"),
        ("9.", "Phase 7: Filesystem"),
        ("  9.1", "Virtual File System (VFS)"),
        ("  9.2", "FAT32 Read-Only Driver"),
        ("  9.3", "RAM Filesystem"),
        ("10.", "Phase 8: Network Stack"),
        ("  10.1", "Ethernet Frame Layer"),
        ("  10.2", "ARP"),
        ("  10.3", "IPv4"),
        ("  10.4", "ICMP (Ping)"),
        ("  10.5", "UDP"),
        ("  10.6", "TCP"),
        ("11.", "Phase 9: System Call Interface"),
        ("12.", "Phase 10: Userspace & Init Process"),
        ("  12.1", "ELF Loader"),
        ("  12.2", "User Page Table & EL0 Transition"),
        ("  12.3", "Init Process"),
        ("13.", "Bugs, Issues & Fixes"),
        ("14.", "Boot Output & Verification"),
        ("15.", "Appendix: Complete Source Listing"),
    ]
    for num, title in toc_items:
        if num.startswith("  "):
            story.append(Paragraph(f"<b>{num}</b>  {title}", style_toc_sub))
        else:
            story.append(Paragraph(f"<b>{num}</b>  {title}", style_toc))
    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 1. EXECUTIVE SUMMARY
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("1. Executive Summary"))
    story.extend(body(
        "<b>custom-os</b> is a fully functional operating system kernel written from scratch in Rust, "
        "targeting the AArch64 (ARM64) architecture. The kernel runs on QEMU's <i>virt</i> machine "
        "with a Cortex-A72 CPU model and 256 MB of RAM. It implements all fundamental OS subsystems: "
        "boot initialization, interrupt handling, virtual memory with MMU, heap allocation, preemptive "
        "multitasking, device drivers, a filesystem layer, a TCP/IP network stack, a system call "
        "interface, and user-mode process execution."
    ))
    story.extend(body(
        "The project demonstrates that a modern systems programming language like Rust can be used to "
        "build a complete operating system from bare metal, leveraging Rust's <font face='Courier'>#![no_std]</font> "
        "and <font face='Courier'>#![no_main]</font> attributes to operate without any runtime library. "
        "The kernel was developed in 10 incremental phases, each independently testable, progressing "
        "from serial output to a fully running userspace init process that communicates with the kernel "
        "via system calls."
    ))

    story.extend(heading2("Key Achievements"))
    achievements = [
        "Bare-metal AArch64 boot with assembly stub, BSS zeroing, and FP/SIMD enablement",
        "AArch64 exception vector table with 16 handler stubs using branch-to-stub pattern",
        "GICv3 interrupt controller initialization (Distributor + Redistributor + CPU Interface)",
        "ARM Generic Timer with 10ms tick driving preemptive scheduling",
        "4-level page table MMU with identity + higher-half mapping (48-bit VA, 4KB granule)",
        "Bitmap-based physical page allocator managing 65,536 pages (256 MB)",
        "Linked-list kernel heap allocator with coalescing free blocks",
        "Preemptive round-robin scheduler with assembly context_switch",
        "VirtIO MMIO device drivers for block storage and networking (legacy v1 support)",
        "VFS abstraction with FAT32 read-only filesystem and in-memory RamFS",
        "Full TCP/IP network stack: Ethernet, ARP, IPv4, ICMP, UDP, TCP",
        "SVC-based system call interface dispatching from EL0 to EL1 handlers",
        "ELF loader with user page table creation and EL1-to-EL0 transition via eret",
        "Userspace init process printing 'Hello from userspace!' via sys_write syscall",
    ]
    for a in achievements:
        story.extend(bullet(a))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 2. SYSTEM ARCHITECTURE
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("2. System Architecture"))
    story.extend(body(
        "The kernel follows a monolithic architecture where all subsystems (memory management, "
        "scheduling, drivers, filesystem, networking) run in a single address space at Exception "
        "Level 1 (EL1). Userspace processes run at EL0 with separate page tables (TTBR0) and "
        "communicate with the kernel through the SVC (Supervisor Call) instruction."
    ))

    story.extend(heading2("2.1 Hardware Target & QEMU virt Memory Map"))
    story.extend(body(
        "The target platform is QEMU's <font face='Courier'>virt</font> machine with GICv3. "
        "The following QEMU command launches the OS:"
    ))
    story.extend(code_block(
        "qemu-system-aarch64 \\\n"
        "  -machine virt,gic-version=3 \\\n"
        "  -cpu cortex-a72 \\\n"
        "  -m 256M \\\n"
        "  -nographic -serial mon:stdio \\\n"
        "  -drive file=disk.img,format=raw,if=none,id=disk0 \\\n"
        "  -device virtio-blk-device,drive=disk0 \\\n"
        "  -netdev user,id=net0 \\\n"
        "  -device virtio-net-device,netdev=net0 \\\n"
        "  -kernel target/kernel.bin",
        "QEMU Launch Command"
    ))

    story.extend(make_table(
        ["Device / Region", "Base Address", "Size / IRQ", "Description"],
        [
            ["GIC Distributor", "0x0800_0000", "--", "Interrupt routing configuration"],
            ["GIC Redistributor", "0x080A_0000", "--", "Per-CPU interrupt config (GICv3)"],
            ["PL011 UART", "0x0900_0000", "SPI 1 (INTID 33)", "Serial console I/O"],
            ["VirtIO MMIO x32", "0x0A00_0000", "SPI 16-47", "32 slots, stride 0x200"],
            ["PCIe ECAM", "0x3F00_0000", "--", "PCI configuration space"],
            ["RAM", "0x4000_0000", "256 MB", "Main system memory"],
        ],
        col_widths=[1.5*inch, 1.3*inch, 1.3*inch, 2.6*inch]
    ))

    story.extend(heading2("2.2 Kernel Virtual Address Layout"))
    story.extend(body(
        "The MMU is configured with separate page tables for the lower half (TTBR0, identity-mapped "
        "during boot, later used for userspace) and upper half (TTBR1, higher-half kernel). Both use "
        "48-bit virtual addresses with 4KB granule."
    ))
    story.extend(make_table(
        ["Region", "Virtual Address", "Mapping"],
        [
            ["MMIO (identity)", "0x0000_0000 - 0x3FFF_FFFF", "Device memory (nGnRnE)"],
            ["RAM (identity)", "0x4000_0000 - 0x4FFF_FFFF", "Normal memory (WB-WA)"],
            ["MMIO (higher-half)", "0xFFFF_0000_0000_0000+", "Mirror of MMIO region"],
            ["Kernel code+data", "0xFFFF_0000_4008_0000+", "Mirror of kernel in RAM"],
            ["Userspace code", "0x0040_0000+", "User ELF segments (TTBR0)"],
            ["Userspace stack", "0x7FFF_C000 - 0x7FFF_FFFF", "16 KB user stack (TTBR0)"],
        ],
        col_widths=[1.5*inch, 2.3*inch, 2.9*inch]
    ))

    story.extend(heading2("2.3 Project Structure"))
    story.extend(code_block(
        "custom-os/\n"
        "  Cargo.toml                     # Workspace root\n"
        "  Makefile                       # Build + run targets\n"
        "  rust-toolchain.toml            # Nightly + aarch64-unknown-none\n"
        "  .cargo/config.toml             # Cross-compilation config\n"
        "  disk.img                       # 64MB FAT32 disk image\n"
        "  kernel/\n"
        "    Cargo.toml\n"
        "    linker.ld                    # Kernel linker script\n"
        "    init.elf                     # Embedded userspace binary\n"
        "    src/\n"
        "      main.rs                    # Kernel entry point\n"
        "      console.rs                 # print!/println! macros\n"
        "      arch/aarch64/\n"
        "        mod.rs, boot.S, exceptions.rs, gic.rs, timer.rs\n"
        "      mm/\n"
        "        mod.rs, pmm.rs, vmm.rs, heap.rs\n"
        "      sync/\n"
        "        mod.rs, spinlock.rs\n"
        "      sched/\n"
        "        mod.rs, task.rs, scheduler.rs, process.rs\n"
        "      drivers/\n"
        "        mod.rs, uart.rs, pci.rs\n"
        "        virtio/ (mod.rs, queue.rs, block.rs, net.rs)\n"
        "      fs/\n"
        "        mod.rs, vfs.rs, fat32.rs, ramfs.rs\n"
        "      net/\n"
        "        mod.rs, ethernet.rs, arp.rs, ipv4.rs,\n"
        "        icmp.rs, udp.rs, tcp.rs\n"
        "      syscall/\n"
        "        mod.rs, handlers.rs\n"
        "  userspace/init/\n"
        "    Cargo.toml, linker.ld, src/main.rs\n"
        "  tools/\n"
        "    mkimage.sh                   # FAT32 disk image creator",
        "Project Directory Layout"
    ))

    story.extend(heading2("2.4 Build Infrastructure"))
    story.extend(body(
        "The project uses Rust's nightly toolchain targeting <font face='Courier'>aarch64-unknown-none</font> "
        "(bare-metal AArch64). The workspace is configured with <font face='Courier'>panic = \"abort\"</font> "
        "and LTO enabled for minimal binary size. The kernel binary is extracted from the ELF using "
        "<font face='Courier'>rust-objcopy</font>."
    ))
    story.extend(code_block(read_file("rust-toolchain.toml"), "rust-toolchain.toml"))
    story.extend(code_block(read_file(".cargo/config.toml"), ".cargo/config.toml"))
    story.extend(code_block(read_file("Makefile"), "Makefile"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 3. PHASE 1: BOOT + UART
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("3. Phase 1: Boot Sequence & UART Console"))
    story.extend(body(
        "The boot sequence begins when QEMU loads the raw kernel binary at physical address "
        "<font face='Courier'>0x4008_0000</font> and jumps to <font face='Courier'>_start</font>. "
        "The assembly boot stub performs critical early initialization before handing control to Rust."
    ))

    story.extend(heading2("3.1 Assembly Boot Stub (boot.S)"))
    story.extend(body(
        "The boot stub performs five essential tasks: (1) save the DTB pointer from x0, "
        "(2) check CPU ID and park secondary cores, (3) zero the BSS section, "
        "(4) enable FP/SIMD via CPACR_EL1, and (5) set the stack pointer and call kernel_main."
    ))
    story.extend(code_block(read_file("kernel/src/arch/aarch64/boot.S"), "kernel/src/arch/aarch64/boot.S"))

    story.extend(heading2("3.2 Linker Script"))
    story.extend(body(
        "The linker script places <font face='Courier'>.text.boot</font> at the QEMU load address "
        "0x4008_0000, followed by code, rodata, data, and BSS sections. It exports symbols for BSS "
        "zeroing and defines a 64KB boot stack."
    ))
    story.extend(code_block(read_file("kernel/linker.ld"), "kernel/linker.ld"))

    story.extend(heading2("3.3 PL011 UART Driver"))
    story.extend(body(
        "The PL011 UART at 0x0900_0000 provides the serial console. The driver implements "
        "<font face='Courier'>core::fmt::Write</font>, checking the TXFF (transmit FIFO full) flag "
        "before writing each character."
    ))
    story.extend(code_block(read_file("kernel/src/drivers/uart.rs"), "kernel/src/drivers/uart.rs"))

    story.extend(heading2("3.4 Console Macros"))
    story.extend(code_block(read_file("kernel/src/console.rs"), "kernel/src/console.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 4. PHASE 2: EXCEPTIONS + GIC + TIMER
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("4. Phase 2: Exception Handling, GICv3 & Timer"))

    story.extend(heading2("4.1 Exception Vector Table"))
    story.extend(body(
        "AArch64 requires a 2KB-aligned vector table with 16 entries, each 128 bytes. Since the full "
        "handler code exceeds 128 bytes, each entry contains a single <font face='Courier'>b</font> "
        "(branch) instruction to a handler stub placed outside the table. Each stub uses "
        "SAVE_CONTEXT/RESTORE_CONTEXT macros that save all 31 general-purpose registers plus ELR_EL1, "
        "SPSR_EL1, ESR_EL1, and FAR_EL1 into an ExceptionContext struct on the stack."
    ))
    story.extend(body(
        "The four exception classes at each of four source levels give 16 vectors total: "
        "Synchronous, IRQ, FIQ, and SError for Current EL with SP_EL0, Current EL with SP_ELx, "
        "Lower EL AArch64, and Lower EL AArch32."
    ))
    story.extend(code_block(read_file("kernel/src/arch/aarch64/exceptions.rs"),
                            "kernel/src/arch/aarch64/exceptions.rs"))

    story.extend(heading2("4.2 GICv3 Interrupt Controller"))
    story.extend(body(
        "The Generic Interrupt Controller version 3 (GICv3) manages all hardware interrupts. "
        "Initialization requires three components: the Distributor (GICD) for global SPI routing, "
        "the Redistributor (GICR) for per-CPU PPI/SGI configuration, and the CPU Interface via "
        "ICC system registers. Key design decision: EOI (End of Interrupt) is written <b>before</b> "
        "dispatching to the handler, because the timer handler may call schedule() which performs a "
        "context switch and never returns to the EOI write."
    ))
    story.extend(code_block(read_file("kernel/src/arch/aarch64/gic.rs"),
                            "kernel/src/arch/aarch64/gic.rs"))

    story.extend(heading2("4.3 ARM Generic Timer"))
    story.extend(body(
        "The virtual EL1 timer (INTID 27, PPI) generates 10ms ticks. The timer frequency is read "
        "from CNTFRQ_EL0, and CNTV_TVAL_EL0 is set to freq/100 for each tick. The tick handler "
        "re-arms the timer and calls the scheduler for preemptive multitasking."
    ))
    story.extend(code_block(read_file("kernel/src/arch/aarch64/timer.rs"),
                            "kernel/src/arch/aarch64/timer.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 5. PHASE 3: MEMORY MANAGEMENT
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("5. Phase 3: Memory Management"))

    story.extend(heading2("5.1 Physical Memory Manager (Bitmap Allocator)"))
    story.extend(body(
        "The PMM uses a bitmap where each bit represents one 4KB page. For 256MB of RAM starting "
        "at 0x4000_0000, there are 65,536 pages requiring a 1024-entry u64 bitmap (8KB). "
        "alloc_page() scans for the first free bit using trailing_zeros(), zeros the allocated page, "
        "and marks it used. Pages occupied by the kernel image are marked during initialization."
    ))
    story.extend(code_block(read_file("kernel/src/mm/pmm.rs"), "kernel/src/mm/pmm.rs"))

    story.extend(heading2("5.2 Virtual Memory Manager (4-Level Page Tables)"))
    story.extend(body(
        "AArch64 uses 4-level page tables (L0-L3) with 512 entries per table and 4KB granule. "
        "Virtual address bits are extracted as: L0 = VA[47:39], L1 = VA[38:30], L2 = VA[29:21], "
        "L3 = VA[20:12]. The boot page tables use 2MB block mappings at L2 for simplicity. "
        "The map_page() function creates full 4-level mappings with L3 page descriptors, used for "
        "userspace process page tables."
    ))
    story.extend(body(
        "Page table entry flags include: Valid (bit 0), Table/Block (bit 1), AttrIndx (bits 4:2) "
        "for MAIR index selection, AP (bits 7:6) for access permissions, SH (bits 9:8) for "
        "shareability, AF (bit 10) for access flag, and UXN/PXN (bits 54:53) for execute-never."
    ))
    story.extend(code_block(read_file("kernel/src/mm/vmm.rs"), "kernel/src/mm/vmm.rs"))

    story.extend(heading2("5.3 MMU Initialization"))
    story.extend(body(
        "MMU setup configures MAIR_EL1 (index 0 = normal WB-WA 0xFF, index 1 = device-nGnRnE 0x00), "
        "TCR_EL1 (T0SZ=T1SZ=16 for 48-bit VA, 4KB granule, inner shareable, 40-bit IPS), and "
        "loads TTBR0/TTBR1 with the boot page table address. Both TTBR0 and TTBR1 initially point "
        "to the same L0 table, providing both identity-mapped and higher-half access."
    ))
    story.extend(code_block(read_file("kernel/src/mm/mod.rs"), "kernel/src/mm/mod.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 6. PHASE 4: HEAP
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("6. Phase 4: Kernel Heap Allocator"))
    story.extend(body(
        "The heap allocator implements <font face='Courier'>GlobalAlloc</font> using a linked-list "
        "of free blocks. It is initialized with 1MB (256 pages) allocated from the PMM. The allocator "
        "uses first-fit allocation and coalesces adjacent free blocks on deallocation to reduce "
        "fragmentation. Thread safety is provided by an inline spin-lock mutex."
    ))
    story.extend(body(
        "With the heap in place, the kernel gains access to Rust's <font face='Courier'>alloc</font> "
        "crate, enabling <font face='Courier'>Box</font>, <font face='Courier'>Vec</font>, "
        "<font face='Courier'>String</font>, <font face='Courier'>BTreeMap</font>, and other "
        "heap-allocated data structures."
    ))
    story.extend(code_block(read_file("kernel/src/mm/heap.rs"), "kernel/src/mm/heap.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 7. PHASE 5: SYNC + SCHEDULER
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("7. Phase 5: Synchronization & Preemptive Scheduler"))

    story.extend(heading2("7.1 SpinLock with Interrupt Masking"))
    story.extend(body(
        "The SpinLock saves and disables interrupts (DAIF) on lock acquisition, restoring them on "
        "drop. This prevents deadlocks from timer IRQs attempting to acquire a lock already held. "
        "The lock uses AtomicBool with Acquire/Release ordering and a test-and-test-and-set pattern."
    ))
    story.extend(code_block(read_file("kernel/src/sync/spinlock.rs"), "kernel/src/sync/spinlock.rs"))

    story.extend(heading2("7.2 Task Structure & Context"))
    story.extend(body(
        "Each task has a <font face='Courier'>TaskContext</font> struct (repr(C) for assembly interop) "
        "containing callee-saved registers x19-x30, SP, and state tracking. Tasks are allocated with "
        "4-page (16KB) kernel stacks from the PMM."
    ))
    story.extend(code_block(read_file("kernel/src/sched/task.rs"), "kernel/src/sched/task.rs"))

    story.extend(heading2("7.3 Round-Robin Scheduler"))
    story.extend(body(
        "The scheduler maintains a VecDeque of tasks. On each timer tick, schedule() is called: "
        "it moves the current task to the back of the queue, picks the next Ready task, and performs "
        "a context switch. A task_entry_trampoline in assembly enables interrupts (DAIFClr) before "
        "calling the task's entry function, bridging the IRQ-disabled context switch to normal execution."
    ))
    story.extend(code_block(read_file("kernel/src/sched/scheduler.rs"),
                            "kernel/src/sched/scheduler.rs"))

    story.extend(heading2("7.4 Assembly Context Switch"))
    story.extend(body(
        "The context_switch function saves callee-saved registers (x19-x30) and SP to the previous "
        "task's TaskContext, then loads them from the next task's TaskContext and returns via the "
        "loaded LR (x30). This seamlessly resumes the next task from wherever it was last preempted."
    ))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 8. PHASE 6: DRIVERS
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("8. Phase 6: Device Drivers"))

    story.extend(heading2("8.1 VirtIO MMIO Transport"))
    story.extend(body(
        "VirtIO devices are discovered by probing 32 MMIO slots starting at 0x0A00_0000 with a stride "
        "of 0x200. Each slot is checked for the magic value 0x74726976 ('virt' in little-endian). "
        "Device initialization follows the VirtIO specification: reset, ACKNOWLEDGE, DRIVER, feature "
        "negotiation, FEATURES_OK, queue setup, and DRIVER_OK. The driver supports both legacy (v1) "
        "and modern (v2) transports."
    ))
    story.extend(code_block(read_file("kernel/src/drivers/virtio/mod.rs"),
                            "kernel/src/drivers/virtio/mod.rs"))

    story.extend(heading2("8.2 Split Virtqueue Implementation"))
    story.extend(body(
        "The split virtqueue consists of three components: a descriptor table (buffer addresses and "
        "sizes), an available ring (descriptors offered to the device), and a used ring (descriptors "
        "returned by the device). The legacy layout requires the used ring to be page-aligned after "
        "the descriptor table and available ring. Descriptors are managed as a free list."
    ))
    story.extend(code_block(read_file("kernel/src/drivers/virtio/queue.rs"),
                            "kernel/src/drivers/virtio/queue.rs"))

    story.extend(heading2("8.3 VirtIO Block Driver"))
    story.extend(body(
        "The block driver reads/writes 512-byte sectors using 3-descriptor chains: a request header "
        "(type + sector number), a data buffer, and a status byte. It polls the used ring for "
        "completion. This driver enables the FAT32 filesystem."
    ))
    story.extend(code_block(read_file("kernel/src/drivers/virtio/block.rs"),
                            "kernel/src/drivers/virtio/block.rs"))

    story.extend(heading2("8.4 VirtIO Network Driver"))
    story.extend(body(
        "The network driver manages RX and TX virtqueues. RX buffers are pre-populated and submitted "
        "to the device. Each frame is prefixed with a VirtioNetHeader (10 bytes). The MAC address is "
        "read from device configuration space at offset 0x100."
    ))
    story.extend(code_block(read_file("kernel/src/drivers/virtio/net.rs"),
                            "kernel/src/drivers/virtio/net.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 9. PHASE 7: FILESYSTEM
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("9. Phase 7: Filesystem"))

    story.extend(heading2("9.1 Virtual File System (VFS)"))
    story.extend(body(
        "The VFS provides a unified interface for all filesystems through the FileSystem and FileOps "
        "traits. Mount points are stored in a sorted vector (longest path first) enabling "
        "longest-prefix matching. The VFS is protected by a SpinLock for thread safety."
    ))
    story.extend(code_block(read_file("kernel/src/fs/vfs.rs"), "kernel/src/fs/vfs.rs"))

    story.extend(heading2("9.2 FAT32 Read-Only Driver"))
    story.extend(body(
        "The FAT32 driver parses the BIOS Parameter Block (BPB) from sector 0, follows cluster "
        "chains through the FAT, and parses both 8.3 short directory entries and Long File Name "
        "(LFN) entries. Files are read by following the cluster chain and concatenating sectors. "
        "The block device is wrapped in a SpinLock for safe concurrent access."
    ))
    story.extend(code_block(read_file("kernel/src/fs/fat32.rs"), "kernel/src/fs/fat32.rs"))

    story.extend(heading2("9.3 RAM Filesystem"))
    story.extend(body(
        "RamFS stores files and directories as an in-memory tree using BTreeMap. It supports "
        "create, read, write, and delete operations, and is mounted at /tmp."
    ))
    story.extend(code_block(read_file("kernel/src/fs/ramfs.rs"), "kernel/src/fs/ramfs.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 10. PHASE 8: NETWORK STACK
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("10. Phase 8: Network Stack"))
    story.extend(body(
        "The network stack implements a layered architecture from Ethernet frames up to TCP "
        "connections. The guest IP is 10.0.2.15 (QEMU user-mode networking default), with gateway "
        "at 10.0.2.2 and subnet mask 255.255.255.0."
    ))

    story.extend(heading2("10.1 Ethernet Frame Layer"))
    story.extend(body(
        "Handles frame parsing and construction. Dispatches by EtherType: 0x0800 for IPv4, "
        "0x0806 for ARP."
    ))
    story.extend(code_block(read_file("kernel/src/net/ethernet.rs"),
                            "kernel/src/net/ethernet.rs"))

    story.extend(heading2("10.2 ARP (Address Resolution Protocol)"))
    story.extend(body(
        "Maintains a BTreeMap-based ARP table. Responds to ARP requests for our IP and resolves "
        "MAC addresses for outgoing packets. Resolution includes a polling loop that processes "
        "incoming frames while waiting for the reply."
    ))
    story.extend(code_block(read_file("kernel/src/net/arp.rs"), "kernel/src/net/arp.rs"))

    story.extend(heading2("10.3 IPv4"))
    story.extend(body(
        "Parses and constructs IPv4 headers with internet checksum calculation. Handles TTL=64, "
        "Don't Fragment flag, and subnet-based next-hop routing (same subnet = direct, otherwise = gateway)."
    ))
    story.extend(code_block(read_file("kernel/src/net/ipv4.rs"), "kernel/src/net/ipv4.rs"))

    story.extend(heading2("10.4 ICMP"))
    story.extend(body(
        "Responds to Echo Request with Echo Reply, enabling <font face='Courier'>ping</font> from the host."
    ))
    story.extend(code_block(read_file("kernel/src/net/icmp.rs"), "kernel/src/net/icmp.rs"))

    story.extend(heading2("10.5 UDP"))
    story.extend(body(
        "Stateless UDP with a port dispatch table. Applications bind handlers to ports. "
        "A UDP echo server runs on port 7."
    ))
    story.extend(code_block(read_file("kernel/src/net/udp.rs"), "kernel/src/net/udp.rs"))

    story.extend(heading2("10.6 TCP"))
    story.extend(body(
        "Simplified TCP state machine implementing LISTEN, SYN_RECEIVED, ESTABLISHED, CLOSE_WAIT, "
        "LAST_ACK, FIN_WAIT1, FIN_WAIT2, and CLOSED states. Supports passive open (server), "
        "connection tracking with BTreeMap, proper TCP checksum with pseudo-header, and data "
        "echo for testing. A TCP echo server listens on port 8080."
    ))
    story.extend(code_block(read_file("kernel/src/net/tcp.rs"), "kernel/src/net/tcp.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 11. PHASE 9: SYSCALLS
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("11. Phase 9: System Call Interface"))
    story.extend(body(
        "System calls use the AArch64 SVC (Supervisor Call) instruction. The convention is: "
        "syscall number in x8, arguments in x0-x5, return value in x0. When SVC executes at EL0, "
        "the CPU takes a synchronous exception to the Lower EL AArch64 vector. The handler checks "
        "ESR_EL1.EC == 0x15 (SVC from AArch64) and dispatches to the syscall handler, which "
        "modifies x0 in the ExceptionContext for the return value."
    ))

    story.extend(make_table(
        ["Number", "Name", "Arguments", "Description"],
        [
            ["0", "exit", "x0=code", "Terminate process"],
            ["1", "write", "x0=fd, x1=buf, x2=len", "Write to file descriptor"],
            ["2", "read", "x0=fd, x1=buf, x2=len", "Read from file descriptor"],
            ["3", "open", "x0=path, x1=len", "Open file"],
            ["4", "close", "x0=fd", "Close file descriptor"],
            ["7", "yield", "(none)", "Yield CPU to scheduler"],
            ["8", "getpid", "(none)", "Get current process ID"],
            ["9", "sleep", "x0=ms", "Sleep for milliseconds"],
        ],
        col_widths=[0.7*inch, 0.8*inch, 2*inch, 3.2*inch]
    ))

    story.extend(code_block(read_file("kernel/src/syscall/mod.rs"), "kernel/src/syscall/mod.rs"))
    story.extend(code_block(read_file("kernel/src/syscall/handlers.rs"),
                            "kernel/src/syscall/handlers.rs"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 12. PHASE 10: USERSPACE
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("12. Phase 10: Userspace & Init Process"))

    story.extend(heading2("12.1 ELF Loader"))
    story.extend(body(
        "The init process binary is compiled as a separate Rust crate targeting aarch64-unknown-none, "
        "linked at virtual address 0x400000. The ELF binary is embedded in the kernel via "
        "<font face='Courier'>include_bytes!</font>. The ELF loader parses the ELF64 header, "
        "iterates PT_LOAD segments, allocates physical pages, copies segment data, and maps them "
        "into a freshly allocated user page table (TTBR0)."
    ))

    story.extend(heading2("12.2 User Page Table & EL0 Transition"))
    story.extend(body(
        "A critical challenge: the kernel executes from the lower-half address space (0x4008_0000), "
        "which is mapped via TTBR0. When we switch TTBR0 to the user page table, the kernel code "
        "must still be accessible for the instructions between the TTBR0 write and the eret. "
        "This is solved by merging the boot page table entries into the user page table at L1/L2 "
        "level, handling collisions where user and kernel share the same L0 index."
    ))
    story.extend(body(
        "The EL0 transition sets TTBR0 to the user page table, flushes TLBs, sets SP_EL0 to the "
        "user stack top (0x8000_0000), ELR_EL1 to the ELF entry point, SPSR_EL1 to 0 (EL0t with "
        "interrupts enabled), and executes eret."
    ))
    story.extend(code_block(read_file("kernel/src/sched/process.rs"),
                            "kernel/src/sched/process.rs"))

    story.extend(heading2("12.3 Init Process"))
    story.extend(body(
        "The init process is a minimal no_std Rust binary that uses inline assembly SVC wrappers "
        "to make system calls. It writes 'Hello from userspace!' to stdout, queries its PID, and "
        "enters a loop printing tick counts with periodic sys_yield calls."
    ))
    story.extend(code_block(read_file("userspace/init/src/main.rs"),
                            "userspace/init/src/main.rs"))
    story.extend(code_block(read_file("userspace/init/linker.ld"),
                            "userspace/init/linker.ld"))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 13. BUGS, ISSUES & FIXES
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("13. Bugs, Issues & Fixes"))
    story.extend(body(
        "Building an OS from scratch inevitably involves debugging subtle hardware and software "
        "interactions. This section documents every significant bug encountered during development, "
        "its root cause, and the fix applied. These represent hard-won lessons in bare-metal "
        "AArch64 programming."
    ))

    bugs = [
        {
            "title": "1. Exception Vector Table Overflow",
            "symptom": "QEMU crashed or produced garbage output when exceptions fired.",
            "cause": "Each vector table entry is limited to 128 bytes (0x80). The initial implementation "
                     "placed the full SAVE_CONTEXT macro, handler call, and RESTORE_CONTEXT inside "
                     "each entry, totaling ~170 bytes that overflowed into the next vector entry.",
            "fix": "Changed each vector entry to contain only a single <font face='Courier'>b</font> "
                   "(branch) instruction, branching to handler stubs placed outside the vector table "
                   "with no size constraint. This is the standard 'branch-to-stub' pattern.",
        },
        {
            "title": "2. GICv2 vs GICv3 Mismatch",
            "symptom": "ICC system register accesses (msr/mrs ICC_*) caused undefined instruction "
                       "exceptions.",
            "cause": "QEMU's virt machine defaults to GICv2, which does not support ICC system "
                     "registers. The GICv3 driver was written assuming GICv3.",
            "fix": "Added <font face='Courier'>-machine virt,gic-version=3</font> to the QEMU "
                   "command line to force GICv3.",
        },
        {
            "title": "3. Operator Precedence Bug in Page Table Setup",
            "symptom": "2MB block mappings pointed to wrong physical addresses; memory corruption.",
            "cause": "In Rust, the <font face='Courier'>&lt;&lt;</font> operator has lower precedence "
                     "than <font face='Courier'>+</font>. The expression "
                     "<font face='Courier'>0x4000_0000 + (i as u64) &lt;&lt; 21</font> was parsed as "
                     "<font face='Courier'>(0x4000_0000 + (i as u64)) &lt;&lt; 21</font>.",
            "fix": "Added explicit parentheses: "
                   "<font face='Courier'>0x4000_0000 + ((i as u64) &lt;&lt; 21)</font>.",
        },
        {
            "title": "4. ESR/FAR Save Offset Mismatch",
            "symptom": "Exception handler printed garbage values for ESR_EL1 and FAR_EL1.",
            "cause": "The assembly stored ESR/FAR at stack offset <font face='Courier'>[sp, #16*17]</font> "
                     "(272 bytes), but the Rust ExceptionContext struct expected them at offset 264 bytes "
                     "(after 31 GPRs + ELR + SPSR = 33 u64 values = 264 bytes).",
            "fix": "Changed the store offset to <font face='Courier'>[sp, #16*16+8]</font> (264), "
                   "matching the struct layout.",
        },
        {
            "title": "5. FP/SIMD Trap from Compiler-Generated NEON Instructions",
            "symptom": "Synchronous exception with EC=7 (FP/SIMD trapped) during array initialization.",
            "cause": "The Rust compiler used NEON (SIMD) instructions for optimized array fills (e.g., "
                     "zeroing buffers). CPACR_EL1.FPEN was not set, so all FP/SIMD instructions were "
                     "trapped.",
            "fix": "Added FP/SIMD enablement in boot.S before calling kernel_main: "
                   "<font face='Courier'>mov x1, #(3 &lt;&lt; 20); msr CPACR_EL1, x1; isb</font>. "
                   "This sets FPEN=0b11, allowing FP/SIMD at all exception levels.",
        },
        {
            "title": "6. Heap Address Mismatch",
            "symptom": "Panic during heap initialization: assertion on expected address failed.",
            "cause": "The heap init code hardcoded HEAP_START=0x42000000, but the PMM's first free page "
                     "was at a different address because more kernel pages were used than expected.",
            "fix": "Removed the address assertion and dynamically used whatever address the PMM returns "
                   "for the first heap page.",
        },
        {
            "title": "7. No Preemption (Tasks Not Switching) - Part 1",
            "symptom": "Only the first task ran; other tasks never executed despite timer ticks.",
            "cause": "New tasks were created with LR pointing to the task entry function. When the "
                     "scheduler did a context_switch from the timer IRQ handler, the new task began "
                     "executing with interrupts still masked (DAIF from the IRQ context). Since eret "
                     "was never executed for the new task, interrupts remained masked forever.",
            "fix": "Created a <font face='Courier'>task_entry_trampoline</font> in assembly that "
                   "executes <font face='Courier'>msr DAIFClr, #0b0010</font> (unmask IRQs) before "
                   "branching to the actual entry function. New tasks' LR is set to this trampoline.",
        },
        {
            "title": "8. No Preemption (Tasks Not Switching) - Part 2",
            "symptom": "After fixing the trampoline, tasks still didn't preempt.",
            "cause": "The GIC EOI (End of Interrupt) write was at the end of handle_irq(), after the "
                     "handler dispatch. But when the timer handler called schedule() and context_switch "
                     "switched to another task, handle_irq() never returned, so EOI was never written. "
                     "The GIC then suppressed all further interrupts of equal or lower priority.",
            "fix": "Moved the ICC_EOIR1_EL1 write to <b>before</b> dispatching to the handler, "
                   "ensuring EOI is always acknowledged even if the handler doesn't return.",
        },
        {
            "title": "9. VirtIO Legacy vs Modern Protocol Confusion",
            "symptom": "VirtIO device initialization failed; block reads returned no data.",
            "cause": "QEMU's virt machine presents VirtIO devices as version 1 (legacy), but the driver "
                     "initially used version 2 (modern) registers: DEVICE_FEATURES_SEL, QUEUE_DESC_LOW/HIGH, "
                     "QUEUE_READY. Legacy devices use different registers: GUEST_PAGE_SIZE, QUEUE_PFN, "
                     "QUEUE_ALIGN.",
            "fix": "Added version checking in init_device() and setup_queue(). For version 1: set "
                   "GUEST_PAGE_SIZE=4096, use QUEUE_PFN for queue address (as page frame number), "
                   "use QUEUE_ALIGN=4096. For version 2: use QUEUE_DESC_LOW/HIGH, QUEUE_AVAIL_LOW/HIGH, "
                   "QUEUE_USED_LOW/HIGH, and QUEUE_READY.",
        },
        {
            "title": "10. PCI ECAM Data Abort",
            "symptom": "Data abort exception when scanning PCI at 0x3F00_0000.",
            "cause": "QEMU's virt machine without explicit PCI devices doesn't properly handle ECAM "
                     "reads, causing a bus abort for invalid device/function combinations.",
            "fix": "Skipped PCI enumeration; VirtIO MMIO is used instead.",
        },
        {
            "title": "11. static mut Reference Warnings (Rust 2024 Edition)",
            "symptom": "Compiler warnings about deprecated <font face='Courier'>&amp;BOOT_TABLES.field</font> syntax.",
            "cause": "Rust is deprecating direct references to mutable statics due to undefined "
                     "behavior risks.",
            "fix": "Changed to <font face='Courier'>&amp;raw const BOOT_TABLES.field</font> for "
                   "raw pointer creation without creating a reference.",
        },
        {
            "title": "12. Virtqueue Raw Pointer Not Send",
            "symptom": "Compilation error: <font face='Courier'>*mut VirtqDesc cannot be sent between threads safely</font>.",
            "cause": "The Virtqueue struct contains raw pointers (*mut VirtqDesc, etc.) which don't "
                     "implement Send. The FAT32 filesystem wraps VirtioBlock in SpinLock, which requires Send.",
            "fix": "Added <font face='Courier'>unsafe impl Send for Virtqueue {}</font>. The safety "
                   "argument: virtqueue access is always protected by the SpinLock wrapping VirtioBlock.",
        },
        {
            "title": "13. TTBR0 Switch Crash During EL0 Transition",
            "symptom": "System hangs after <font face='Courier'>eret</font> to userspace; no exceptions printed.",
            "cause": "The kernel runs at 0x4008_0000, which is in TTBR0's address range (lower half). "
                     "When spawn_init() changed TTBR0 to the user page table, the next instruction fetch "
                     "used the new TTBR0, which didn't have the kernel mapped at 0x4008_0000. The CPU "
                     "faulted trying to fetch the eret instruction itself.",
            "fix": "Merged the boot page table entries into the user page table at L1/L2 granularity, "
                   "handling collisions where user (0x400000) and kernel (0x40080000) share the same "
                   "L0 index. The merge walks the boot page table tree and copies entries that don't "
                   "conflict with user mappings.",
        },
        {
            "title": "14. EC=0 (Unknown) Exception at User Entry Point",
            "symptom": "After fixing TTBR0, got EC=0x0 exception at ELR=0x400000.",
            "cause": "The first version of the merge code simply copied boot L0[0] over user L0[0], "
                     "which destroyed the user's page table entries for 0x400000. Physical address "
                     "in L3 entry was 0x0, and the instruction at physical 0x0 was UDF (undefined).",
            "fix": "Implemented proper 3-level merge: at each level (L0, L1, L2), only copy boot "
                   "entries where the user doesn't already have a mapping, preserving both kernel "
                   "and user mappings in the combined page table.",
        },
    ]

    for bug in bugs:
        story.extend(heading3(bug["title"]))
        story.extend(body(f"<b>Symptom:</b> {bug['symptom']}"))
        story.extend(body(f"<b>Root Cause:</b> {bug['cause']}"))
        story.extend(body(f"<b>Fix:</b> {bug['fix']}"))
        story.append(Spacer(1, 6))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 14. BOOT OUTPUT & VERIFICATION
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("14. Boot Output & Verification"))
    story.extend(body(
        "The following is the actual serial console output from a successful boot of custom-os "
        "on QEMU, demonstrating all 10 phases functioning correctly:"
    ))
    story.extend(code_block(
        "Hello from custom-os!\n"
        "DTB pointer: 0x48000000\n"
        "[exceptions] Vector table installed\n"
        "[gic] GICv3 initialized\n"
        "[timer] Initialized at 62500000 Hz, tick every 10ms\n"
        "[vmm] MMU enabled with identity + higher-half mapping\n"
        "[mm] Physical memory: 65334 pages free / 65536 total\n"
        "[heap] Initialized 1024 KB at 0x400ca000\n"
        "[sched] Scheduler initialized\n"
        "[virtio] Found network device (id=1) at 0xa003c00, version 1\n"
        "[virtio] Found block device (id=2) at 0xa003e00, version 1\n"
        "[virtio-net] Initialized, MAC 52:54:00:12:34:56\n"
        "[virtio-blk] Initialized, queue size = 128\n"
        "[fat32] Mounted: 512 bytes/sector, 1 sectors/cluster, root cluster 2\n"
        "[vfs] Mounting 'fat32' at /\n"
        "[vfs] Mounting 'ramfs' at /tmp\n"
        "[fs] /hello.txt (33 bytes): Hello from custom-os filesystem!\n"
        "\n"
        "[fs] / directory listing:\n"
        "  DIR         0  .fseventsd\n"
        "  FILE       33  hello.txt\n"
        "  DIR         0  docs\n"
        "[fs] Created /tmp/test.txt\n"
        "[net] Stack initialized, IP 10.0.2.15, MAC 52:54:00:12:34:56\n"
        "[tcp] Listening on port 8080\n"
        "[sched] Spawned task net_poll (id=1)\n"
        "[sched] Spawned task syscall_test (id=2)\n"
        "[sched] Spawned task init_loader (id=3)\n"
        "[sched] Spawned task task_a (id=4)\n"
        "[sched] Spawned task task_b (id=5)\n"
        "Kernel initialized. Entering idle loop.\n"
        "[syscall] Hello from SVC!\n"
        "[syscall] sys_write returned 26\n"
        "[syscall] sys_getpid returned 0\n"
        "[syscall] All syscall tests passed!\n"
        "[process] Loading init ELF (74960 bytes)\n"
        "[process] Entry point: 0x400000\n"
        "[process] LOAD: vaddr=0x400000 memsz=0x104 filesz=0x104 XR\n"
        "[process] LOAD: vaddr=0x401000 memsz=0x1048 filesz=0x1048 -R\n"
        "[process] User stack at 0x7fffc000-0x80000000\n"
        "[process] Jumping to EL0 at 0x400000\n"
        "Hello from userspace!\n"
        "[init] pid=0\n"
        "[init] tick 1\n"
        "[task_b] tick 1\n"
        "[init] tick 2\n"
        "[task_a] tick 1\n"
        "[task_a] tick 2\n"
        "[init] tick 3\n"
        "[task_b] tick 2\n"
        "[task_b] tick 3\n"
        "[init] tick 4\n"
        "...",
        "Complete Boot Output (serial console)"
    ))

    story.extend(heading2("Verification Checklist"))
    checks = [
        ["Phase 1: Boot + UART", "PASS", "'Hello from custom-os!' printed"],
        ["Phase 2: Exceptions + GIC + Timer", "PASS", "Timer ticking at 10ms, GICv3 initialized"],
        ["Phase 3: MMU + Memory", "PASS", "65,534 free pages, identity + higher-half mapping"],
        ["Phase 4: Heap", "PASS", "1024 KB heap, Vec/Box/String work"],
        ["Phase 5: Scheduler", "PASS", "Tasks interleave with preemptive switching"],
        ["Phase 6: VirtIO Drivers", "PASS", "Block: read FAT32 sector; Net: MAC read"],
        ["Phase 7: Filesystem", "PASS", "/hello.txt read (33 bytes), /tmp/test.txt created"],
        ["Phase 8: Network Stack", "PASS", "IP 10.0.2.15, TCP listening on 8080"],
        ["Phase 9: Syscalls", "PASS", "SVC write returned 26, getpid returned 0"],
        ["Phase 10: Userspace", "PASS", "'Hello from userspace!' from EL0 init process"],
    ]
    story.extend(make_table(
        ["Phase", "Status", "Evidence"],
        checks,
        col_widths=[2.2*inch, 0.6*inch, 3.9*inch]
    ))

    story.append(PageBreak())

    # ═══════════════════════════════════════════════════════════════════
    # 15. APPENDIX: KERNEL ENTRY POINT
    # ═══════════════════════════════════════════════════════════════════
    story.extend(heading1("15. Appendix: Kernel Entry Point"))
    story.extend(body(
        "The kernel_main function orchestrates all initialization phases sequentially. "
        "This is the central integration point where all subsystems are brought up in the correct order."
    ))
    story.extend(code_block(read_file("kernel/src/main.rs"), "kernel/src/main.rs"))

    story.extend(heading2("Phase Dependency Graph"))
    story.extend(code_block(
        "P1 (Boot+UART)\n"
        "  |___ P2 (Exceptions+GIC+Timer)\n"
        "        |___ P3 (MMU+Memory)\n"
        "              |___ P4 (Heap)\n"
        "                    |___ P5 (Scheduler)\n"
        "                          |___ P6 (Drivers)\n"
        "                          |     |___ P7 (Filesystem)\n"
        "                          |     |___ P8 (Network Stack)\n"
        "                          |___ P9 (Syscalls)\n"
        "                                |___ P10 (Userspace)",
        "Implementation Phase Dependencies"
    ))

    story.extend(heading2("Disk Image Creation Script"))
    story.extend(code_block(read_file("tools/mkimage.sh"), "tools/mkimage.sh"))

    story.extend(heading2("Architecture Module"))
    story.extend(code_block(read_file("kernel/src/arch/aarch64/mod.rs"),
                            "kernel/src/arch/aarch64/mod.rs"))

    # ═══════════════════════════════════════════════════════════════════
    # BUILD
    # ═══════════════════════════════════════════════════════════════════
    doc.build(story, onFirstPage=first_page, onLaterPages=add_page_number)
    print(f"\nPDF generated: {OUTPUT_PDF}")
    print(f"  Pages: (check file)")
    fsize = os.path.getsize(OUTPUT_PDF)
    print(f"  Size: {fsize / 1024:.0f} KB")


if __name__ == "__main__":
    build()

.section .text.boot
.global _start

_start:
    // x0 contains DTB pointer from QEMU, save it
    mov     x19, x0

    // Only let CPU 0 through, park all others
    mrs     x1, mpidr_el1
    and     x1, x1, #0xFF
    cbnz    x1, .Lpark

    // Zero BSS
    adrp    x1, __bss_start
    add     x1, x1, :lo12:__bss_start
    adrp    x2, __bss_end
    add     x2, x2, :lo12:__bss_end
.Lbss_loop:
    cmp     x1, x2
    b.ge    .Lbss_done
    stp     xzr, xzr, [x1], #16
    b       .Lbss_loop
.Lbss_done:

    // Enable FP/SIMD (CPACR_EL1.FPEN = 0b11)
    mov     x1, #(3 << 20)
    msr     CPACR_EL1, x1
    isb

    // Set up the stack
    adrp    x1, __stack_top
    add     x1, x1, :lo12:__stack_top
    mov     sp, x1

    // Jump to Rust entry point with DTB pointer as argument
    mov     x0, x19
    bl      kernel_main

    // Should never return, but just in case
.Lpark:
    wfe
    b       .Lpark

use core::arch::global_asm;

#[repr(C)]
pub struct ExceptionContext {
    pub gpr: [u64; 31],
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub esr_el1: u64,
    pub far_el1: u64,
}

const CONTEXT_SIZE: usize = core::mem::size_of::<ExceptionContext>();

global_asm!(
    "
    .macro SAVE_CONTEXT
        sub     sp, sp, #{size}
        stp     x0,  x1,  [sp, #16 * 0]
        stp     x2,  x3,  [sp, #16 * 1]
        stp     x4,  x5,  [sp, #16 * 2]
        stp     x6,  x7,  [sp, #16 * 3]
        stp     x8,  x9,  [sp, #16 * 4]
        stp     x10, x11, [sp, #16 * 5]
        stp     x12, x13, [sp, #16 * 6]
        stp     x14, x15, [sp, #16 * 7]
        stp     x16, x17, [sp, #16 * 8]
        stp     x18, x19, [sp, #16 * 9]
        stp     x20, x21, [sp, #16 * 10]
        stp     x22, x23, [sp, #16 * 11]
        stp     x24, x25, [sp, #16 * 12]
        stp     x26, x27, [sp, #16 * 13]
        stp     x28, x29, [sp, #16 * 14]
        str     x30,      [sp, #16 * 15]

        mrs     x0, elr_el1
        mrs     x1, spsr_el1
        mrs     x2, esr_el1
        mrs     x3, far_el1
        stp     x0, x1, [sp, #16 * 15 + 8]
        stp     x2, x3, [sp, #16 * 16 + 8]
    .endm

    .macro RESTORE_CONTEXT
        ldp     x0, x1, [sp, #16 * 15 + 8]
        msr     elr_el1, x0
        msr     spsr_el1, x1

        ldp     x0,  x1,  [sp, #16 * 0]
        ldp     x2,  x3,  [sp, #16 * 1]
        ldp     x4,  x5,  [sp, #16 * 2]
        ldp     x6,  x7,  [sp, #16 * 3]
        ldp     x8,  x9,  [sp, #16 * 4]
        ldp     x10, x11, [sp, #16 * 5]
        ldp     x12, x13, [sp, #16 * 6]
        ldp     x14, x15, [sp, #16 * 7]
        ldp     x16, x17, [sp, #16 * 8]
        ldp     x18, x19, [sp, #16 * 9]
        ldp     x20, x21, [sp, #16 * 10]
        ldp     x22, x23, [sp, #16 * 11]
        ldp     x24, x25, [sp, #16 * 12]
        ldp     x26, x27, [sp, #16 * 13]
        ldp     x28, x29, [sp, #16 * 14]
        ldr     x30,      [sp, #16 * 15]
        add     sp, sp, #{size}
    .endm

    .macro HANDLER_STUB handler
        SAVE_CONTEXT
        mov     x0, sp
        bl      \\handler
        RESTORE_CONTEXT
        eret
    .endm

    // Vector table: each entry is only 0x80 (128) bytes, so just branch out
    .balign 0x800
    .global __exception_vectors
    __exception_vectors:

    // Current EL with SP_EL0
    .balign 0x80
        b       __exc_sync_cur_sp0
    .balign 0x80
        b       __exc_irq_cur_sp0
    .balign 0x80
        b       __exc_fiq_cur_sp0
    .balign 0x80
        b       __exc_serror_cur_sp0

    // Current EL with SP_ELx
    .balign 0x80
        b       __exc_sync_cur_spx
    .balign 0x80
        b       __exc_irq_cur_spx
    .balign 0x80
        b       __exc_fiq_cur_spx
    .balign 0x80
        b       __exc_serror_cur_spx

    // Lower EL, AArch64
    .balign 0x80
        b       __exc_sync_lower64
    .balign 0x80
        b       __exc_irq_lower64
    .balign 0x80
        b       __exc_fiq_lower64
    .balign 0x80
        b       __exc_serror_lower64

    // Lower EL, AArch32
    .balign 0x80
        b       __exc_sync_lower32
    .balign 0x80
        b       __exc_irq_lower32
    .balign 0x80
        b       __exc_fiq_lower32
    .balign 0x80
        b       __exc_serror_lower32

    // Handler stubs (outside the vector table, no size limit)
    __exc_sync_cur_sp0:
        HANDLER_STUB exception_sync_current_el_sp0
    __exc_irq_cur_sp0:
        HANDLER_STUB exception_irq_current_el_sp0
    __exc_fiq_cur_sp0:
        HANDLER_STUB exception_fiq_current_el_sp0
    __exc_serror_cur_sp0:
        HANDLER_STUB exception_serror_current_el_sp0

    __exc_sync_cur_spx:
        HANDLER_STUB exception_sync_current_el_spx
    __exc_irq_cur_spx:
        HANDLER_STUB exception_irq_current_el_spx
    __exc_fiq_cur_spx:
        HANDLER_STUB exception_fiq_current_el_spx
    __exc_serror_cur_spx:
        HANDLER_STUB exception_serror_current_el_spx

    __exc_sync_lower64:
        HANDLER_STUB exception_sync_lower_el_aarch64
    __exc_irq_lower64:
        HANDLER_STUB exception_irq_lower_el_aarch64
    __exc_fiq_lower64:
        HANDLER_STUB exception_fiq_lower_el_aarch64
    __exc_serror_lower64:
        HANDLER_STUB exception_serror_lower_el_aarch64

    __exc_sync_lower32:
        HANDLER_STUB exception_sync_lower_el_aarch32
    __exc_irq_lower32:
        HANDLER_STUB exception_irq_lower_el_aarch32
    __exc_fiq_lower32:
        HANDLER_STUB exception_fiq_lower_el_aarch32
    __exc_serror_lower32:
        HANDLER_STUB exception_serror_lower_el_aarch32
    ",
    size = const CONTEXT_SIZE,
);

fn default_handler(ctx: &ExceptionContext, kind: &str) {
    crate::println!(
        "Unhandled exception: {}",
        kind
    );
    crate::println!(
        "  ELR_EL1:  {:#018x}  SPSR_EL1: {:#018x}",
        ctx.elr_el1,
        ctx.spsr_el1
    );
    crate::println!(
        "  ESR_EL1:  {:#018x}  FAR_EL1:  {:#018x}",
        ctx.esr_el1,
        ctx.far_el1
    );
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
extern "C" fn exception_sync_current_el_sp0(ctx: &ExceptionContext) {
    default_handler(ctx, "Synchronous (Current EL, SP_EL0)");
}

#[no_mangle]
extern "C" fn exception_irq_current_el_sp0(ctx: &ExceptionContext) {
    default_handler(ctx, "IRQ (Current EL, SP_EL0)");
}

#[no_mangle]
extern "C" fn exception_fiq_current_el_sp0(ctx: &ExceptionContext) {
    default_handler(ctx, "FIQ (Current EL, SP_EL0)");
}

#[no_mangle]
extern "C" fn exception_serror_current_el_sp0(ctx: &ExceptionContext) {
    default_handler(ctx, "SError (Current EL, SP_EL0)");
}

#[no_mangle]
extern "C" fn exception_sync_current_el_spx(ctx: &mut ExceptionContext) {
    let ec = (ctx.esr_el1 >> 26) & 0x3F;
    match ec {
        0x15 => {
            // SVC from EL1 (kernel-mode syscall test)
            crate::syscall::dispatch(ctx);
        }
        _ => {
            default_handler(ctx, "Synchronous (Current EL, SP_ELx)");
        }
    }
}

#[no_mangle]
extern "C" fn exception_irq_current_el_spx(_ctx: &ExceptionContext) {
    super::gic::handle_irq();
}

#[no_mangle]
extern "C" fn exception_fiq_current_el_spx(ctx: &ExceptionContext) {
    default_handler(ctx, "FIQ (Current EL, SP_ELx)");
}

#[no_mangle]
extern "C" fn exception_serror_current_el_spx(ctx: &ExceptionContext) {
    default_handler(ctx, "SError (Current EL, SP_ELx)");
}

#[no_mangle]
extern "C" fn exception_sync_lower_el_aarch64(ctx: &mut ExceptionContext) {
    let ec = (ctx.esr_el1 >> 26) & 0x3F;
    match ec {
        0x15 => {
            crate::syscall::dispatch(ctx);
        }
        _ => {
            crate::println!(
                "[exc] Lower EL sync: EC={:#x} ESR={:#x} ELR={:#x} FAR={:#x}",
                ec, ctx.esr_el1, ctx.elr_el1, ctx.far_el1
            );
            default_handler(ctx, "Synchronous (Lower EL, AArch64)");
        }
    }
}

#[no_mangle]
extern "C" fn exception_irq_lower_el_aarch64(_ctx: &mut ExceptionContext) {
    super::gic::handle_irq();
}

#[no_mangle]
extern "C" fn exception_fiq_lower_el_aarch64(ctx: &ExceptionContext) {
    default_handler(ctx, "FIQ (Lower EL, AArch64)");
}

#[no_mangle]
extern "C" fn exception_serror_lower_el_aarch64(ctx: &ExceptionContext) {
    default_handler(ctx, "SError (Lower EL, AArch64)");
}

#[no_mangle]
extern "C" fn exception_sync_lower_el_aarch32(ctx: &ExceptionContext) {
    default_handler(ctx, "Synchronous (Lower EL, AArch32)");
}

#[no_mangle]
extern "C" fn exception_irq_lower_el_aarch32(ctx: &ExceptionContext) {
    default_handler(ctx, "IRQ (Lower EL, AArch32)");
}

#[no_mangle]
extern "C" fn exception_fiq_lower_el_aarch32(ctx: &ExceptionContext) {
    default_handler(ctx, "FIQ (Lower EL, AArch32)");
}

#[no_mangle]
extern "C" fn exception_serror_lower_el_aarch32(ctx: &ExceptionContext) {
    default_handler(ctx, "SError (Lower EL, AArch32)");
}

pub fn init() {
    unsafe {
        core::arch::asm!(
            "adrp x0, __exception_vectors",
            "add  x0, x0, :lo12:__exception_vectors",
            "msr  vbar_el1, x0",
            "isb",
            out("x0") _,
        );
    }
    crate::println!("[exceptions] Vector table installed");
}

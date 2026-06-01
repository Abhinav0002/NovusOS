use core::ptr::{read_volatile, write_volatile};

const GICD_BASE: usize = 0x0800_0000;
const GICR_BASE: usize = 0x080A_0000;

// Distributor registers
const GICD_CTLR: usize = GICD_BASE;
const GICD_TYPER: usize = GICD_BASE + 0x0004;
const GICD_IGROUPR: usize = GICD_BASE + 0x0080;
const GICD_ISENABLER: usize = GICD_BASE + 0x0100;
const GICD_ICENABLER: usize = GICD_BASE + 0x0180;
const GICD_IPRIORITYR: usize = GICD_BASE + 0x0400;
const GICD_ICFGR: usize = GICD_BASE + 0x0C00;

// Redistributor registers (SGI frame at +0x10000)
const GICR_WAKER: usize = GICR_BASE + 0x0014;
const GICR_SGI_BASE: usize = GICR_BASE + 0x10000;
const GICR_IGROUPR0: usize = GICR_SGI_BASE + 0x0080;
const GICR_ISENABLER0: usize = GICR_SGI_BASE + 0x0100;
const GICR_IPRIORITYR: usize = GICR_SGI_BASE + 0x0400;

// Timer PPI
const TIMER_IRQ: u32 = 27;

unsafe fn reg_write(addr: usize, val: u32) {
    write_volatile(addr as *mut u32, val);
}

unsafe fn reg_read(addr: usize) -> u32 {
    read_volatile(addr as *const u32)
}

fn gicd_init() {
    unsafe {
        // Disable distributor
        reg_write(GICD_CTLR, 0);

        // Read number of interrupt lines
        let typer = reg_read(GICD_TYPER);
        let num_irqs = ((typer & 0x1F) + 1) * 32;

        // Configure all SPIs: Group 1, priority 0xA0, disabled
        let num_regs = (num_irqs / 32) as usize;
        for i in 1..num_regs {
            reg_write(GICD_IGROUPR + i * 4, 0xFFFF_FFFF);
            reg_write(GICD_ICENABLER + i * 4, 0xFFFF_FFFF);
        }

        // Set all SPI priorities to 0xA0
        for i in 8..(num_irqs as usize / 4) {
            reg_write(GICD_IPRIORITYR + i * 4, 0xA0A0_A0A0);
        }

        // Enable distributor with affinity routing (ARE_NS) and Group 1 NS
        reg_write(GICD_CTLR, (1 << 4) | (1 << 1));
    }
}

fn gicr_init() {
    unsafe {
        // Wake up redistributor
        let waker = reg_read(GICR_WAKER);
        reg_write(GICR_WAKER, waker & !(1 << 1)); // Clear ProcessorSleep

        // Wait for ChildrenAsleep to clear
        while (reg_read(GICR_WAKER) & (1 << 2)) != 0 {
            core::hint::spin_loop();
        }

        // Set SGIs/PPIs to Group 1
        reg_write(GICR_IGROUPR0, 0xFFFF_FFFF);

        // Set PPI priorities to 0xA0
        for i in 0..8 {
            reg_write(GICR_IPRIORITYR + i * 4, 0xA0A0_A0A0);
        }

        // Enable timer PPI (INTID 27)
        reg_write(GICR_ISENABLER0, 1 << TIMER_IRQ);
    }
}

fn cpu_interface_init() {
    unsafe {
        // Enable system register interface
        core::arch::asm!(
            "mrs x0, ICC_SRE_EL1",
            "orr x0, x0, #0x7",
            "msr ICC_SRE_EL1, x0",
            "isb",
            out("x0") _,
        );

        // Set priority mask to allow all priorities
        core::arch::asm!("msr ICC_PMR_EL1, {}", in(reg) 0xFFu64);

        // EOI mode: drop priority and deactivate
        core::arch::asm!("msr ICC_CTLR_EL1, {}", in(reg) 0u64);

        // Enable Group 1 interrupts
        core::arch::asm!("msr ICC_IGRPEN1_EL1, {}", in(reg) 1u64);

        // Unmask IRQs (clear I bit in DAIF)
        core::arch::asm!("msr DAIFClr, #0b0010");
    }
}

pub fn init() {
    gicd_init();
    gicr_init();
    cpu_interface_init();
    crate::println!("[gic] GICv3 initialized");
}

pub fn handle_irq() {
    let intid: u64;
    unsafe {
        core::arch::asm!("mrs {}, ICC_IAR1_EL1", out(reg) intid);
    }

    // Write EOI early so the GIC can accept new interrupts even if
    // the handler triggers a context switch and never returns here
    unsafe {
        core::arch::asm!("msr ICC_EOIR1_EL1, {}", in(reg) intid);
    }

    match intid as u32 {
        TIMER_IRQ => {
            super::timer::handle_tick();
        }
        1023 => {}
        id => {
            crate::println!("[gic] Unhandled IRQ: {}", id);
        }
    }
}

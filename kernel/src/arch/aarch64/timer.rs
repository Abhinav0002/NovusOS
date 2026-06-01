use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

const TICKS_PER_SECOND: u64 = 100; // 10ms per tick

fn counter_frequency() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {}, CNTFRQ_EL0", out(reg) freq);
    }
    freq
}

pub fn init() {
    let freq = counter_frequency();
    let tval = freq / TICKS_PER_SECOND;

    unsafe {
        // Set timer value
        core::arch::asm!("msr CNTV_TVAL_EL0, {}", in(reg) tval);
        // Enable timer: ENABLE=1, IMASK=0
        core::arch::asm!("msr CNTV_CTL_EL0, {}", in(reg) 1u64);
    }

    crate::println!("[timer] Initialized at {} Hz, tick every {}ms", freq, 1000 / TICKS_PER_SECOND);
}

pub fn handle_tick() {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;

    // Re-arm the timer
    let freq = counter_frequency();
    let tval = freq / TICKS_PER_SECOND;
    unsafe {
        core::arch::asm!("msr CNTV_TVAL_EL0, {}", in(reg) tval);
    }

    if tick % (TICKS_PER_SECOND * 1) == 0 {
        crate::println!("[timer] {} seconds elapsed", tick / TICKS_PER_SECOND);
    }

    // Trigger scheduler
    crate::sched::schedule();
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

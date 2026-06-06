use core::arch::asm;

fn read_cntfrq() -> u64 {
    let val: u64;
    unsafe { asm!("mrs {}, CNTFRQ_EL0", out(reg) val) };
    val
}

fn read_cntvct() -> u64 {
    let val: u64;
    unsafe { asm!("mrs {}, CNTVCT_EL0", out(reg) val) };
    val
}

pub struct BootTimer {
    freq: u64,
    start: u64,
}

impl BootTimer {
    pub fn new() -> Self {
        Self {
            freq: read_cntfrq(),
            start: read_cntvct(),
        }
    }

    pub fn elapsed_secs(&self) -> u64 {
        let now = read_cntvct();
        if self.freq == 0 {
            return 0;
        }
        (now - self.start) / self.freq
    }

    pub fn format_uptime(&self) -> (u64, u64, u64) {
        let total = self.elapsed_secs();
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        (h, m, s)
    }
}

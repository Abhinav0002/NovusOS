use core::arch::global_asm;

pub mod exceptions;
pub mod gic;
pub mod timer;

global_asm!(include_str!("boot.S"));

pub fn init() {
    exceptions::init();
    gic::init();
    timer::init();
}

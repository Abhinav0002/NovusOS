use core::fmt;

const UART0_BASE: usize = 0x0900_0000;

const UART_DR: usize = 0x00;
const UART_FR: usize = 0x18;
const UART_FR_TXFF: u32 = 1 << 5;

pub struct Pl011 {
    base: usize,
}

impl Pl011 {
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    fn wait_tx_ready(&self) {
        unsafe {
            while (core::ptr::read_volatile((self.base + UART_FR) as *const u32) & UART_FR_TXFF)
                != 0
            {
                core::hint::spin_loop();
            }
        }
    }

    pub fn putc(&self, c: u8) {
        self.wait_tx_ready();
        unsafe {
            core::ptr::write_volatile((self.base + UART_DR) as *mut u8, c);
        }
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.putc(b'\r');
            }
            self.putc(byte);
        }
        Ok(())
    }
}

pub fn uart() -> Pl011 {
    Pl011::new(UART0_BASE)
}

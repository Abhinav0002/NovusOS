use core::ptr::read_volatile;

const ECAM_BASE: usize = 0x3F00_0000;

pub fn enumerate() {
    crate::println!("[pci] Scanning ECAM at {:#x}", ECAM_BASE);
    let mut count = 0;

    for bus in 0..2u32 {
        for dev in 0..32u32 {
            let addr = ECAM_BASE + ((bus << 20) | (dev << 15)) as usize;
            let vendor_id = unsafe { read_volatile(addr as *const u16) };
            if vendor_id == 0xFFFF || vendor_id == 0 {
                continue;
            }
            let device_id = unsafe { read_volatile((addr + 2) as *const u16) };
            let class = unsafe { read_volatile((addr + 11) as *const u8) };
            let subclass = unsafe { read_volatile((addr + 10) as *const u8) };
            crate::println!(
                "[pci] {:02x}:{:02x}.0  {:04x}:{:04x}  class {:02x}.{:02x}",
                bus, dev, vendor_id, device_id, class, subclass
            );
            count += 1;
        }
    }

    crate::println!("[pci] Found {} devices", count);
}

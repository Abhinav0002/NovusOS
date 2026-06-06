pub mod queue;
pub mod block;
pub mod input;
pub mod net;

use core::ptr::{read_volatile, write_volatile};

const VIRTIO_MMIO_BASE: usize = 0x0A00_0000;
const VIRTIO_MMIO_STRIDE: usize = 0x200;
const VIRTIO_MMIO_COUNT: usize = 32;

// MMIO register offsets (common to v1 and v2)
const MAGIC: usize = 0x000;
const VERSION: usize = 0x004;
const DEVICE_ID: usize = 0x008;
const DEVICE_FEATURES: usize = 0x010;
const DEVICE_FEATURES_SEL: usize = 0x014;
const DRIVER_FEATURES: usize = 0x020;
const DRIVER_FEATURES_SEL: usize = 0x024;
const GUEST_PAGE_SIZE: usize = 0x028; // v1 only
const QUEUE_SEL: usize = 0x030;
const QUEUE_NUM_MAX: usize = 0x034;
const QUEUE_NUM: usize = 0x038;
const QUEUE_ALIGN: usize = 0x03C; // v1 only
const QUEUE_PFN: usize = 0x040;   // v1 only
const QUEUE_READY: usize = 0x044; // v2 only
const QUEUE_NOTIFY: usize = 0x050;
const INTERRUPT_STATUS: usize = 0x060;
const INTERRUPT_ACK: usize = 0x064;
const STATUS: usize = 0x070;
// v2 only registers
const QUEUE_DESC_LOW: usize = 0x080;
const QUEUE_DESC_HIGH: usize = 0x084;
const QUEUE_AVAIL_LOW: usize = 0x090;
const QUEUE_AVAIL_HIGH: usize = 0x094;
const QUEUE_USED_LOW: usize = 0x0A0;
const QUEUE_USED_HIGH: usize = 0x0A4;

const DEVICE_NET: u32 = 1;
const DEVICE_BLOCK: u32 = 2;
const DEVICE_INPUT: u32 = 18;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_DRIVER_OK: u32 = 4;

const VIRTIO_MAGIC: u32 = 0x7472_6976;

pub struct VirtioMmio {
    pub base: usize,
    pub device_id: u32,
    pub version: u32,
}

impl VirtioMmio {
    fn read(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }

    fn write(&self, offset: usize, val: u32) {
        unsafe { write_volatile((self.base + offset) as *mut u32, val) }
    }

    pub fn init_device(&self) -> bool {
        // Reset
        self.write(STATUS, 0);

        // Acknowledge
        self.write(STATUS, STATUS_ACKNOWLEDGE);
        self.write(STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

        if self.version == 1 {
            // Legacy: set guest page size, read features directly
            self.write(GUEST_PAGE_SIZE, 4096);
            let features = self.read(DEVICE_FEATURES);
            self.write(DRIVER_FEATURES, features);
            self.write(STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK);
        } else {
            // Modern: use FEATURES_SEL
            self.write(DEVICE_FEATURES_SEL, 0);
            let features = self.read(DEVICE_FEATURES);
            self.write(DRIVER_FEATURES_SEL, 0);
            self.write(DRIVER_FEATURES, features);
            self.write(STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK);
            let status = self.read(STATUS);
            if status & STATUS_FEATURES_OK == 0 {
                crate::println!("[virtio] Features negotiation failed");
                return false;
            }
        }

        true
    }

    pub fn setup_queue(&self, queue_idx: u32, vq: &queue::Virtqueue) {
        self.write(QUEUE_SEL, queue_idx);
        self.write(QUEUE_NUM, vq.size as u32);

        if self.version == 1 {
            // Legacy: set QUEUE_ALIGN and QUEUE_PFN
            self.write(QUEUE_ALIGN, 4096);
            let pfn = vq.desc_addr() as u32 / 4096;
            self.write(QUEUE_PFN, pfn);
        } else {
            let desc_addr = vq.desc_addr();
            self.write(QUEUE_DESC_LOW, desc_addr as u32);
            self.write(QUEUE_DESC_HIGH, (desc_addr >> 32) as u32);

            let avail_addr = vq.avail_addr();
            self.write(QUEUE_AVAIL_LOW, avail_addr as u32);
            self.write(QUEUE_AVAIL_HIGH, (avail_addr >> 32) as u32);

            let used_addr = vq.used_addr();
            self.write(QUEUE_USED_LOW, used_addr as u32);
            self.write(QUEUE_USED_HIGH, (used_addr >> 32) as u32);

            self.write(QUEUE_READY, 1);
        }
    }

    pub fn driver_ok(&self) {
        let status = self.read(STATUS);
        self.write(STATUS, status | STATUS_DRIVER_OK);
    }

    pub fn notify(&self, queue_idx: u32) {
        self.write(QUEUE_NOTIFY, queue_idx);
    }

    pub fn queue_num_max(&self, queue_idx: u32) -> u32 {
        self.write(QUEUE_SEL, queue_idx);
        self.read(QUEUE_NUM_MAX)
    }

    pub fn ack_interrupt(&self) {
        let status = self.read(INTERRUPT_STATUS);
        self.write(INTERRUPT_ACK, status);
    }
}

pub fn probe() -> alloc::vec::Vec<VirtioMmio> {
    let mut devices = alloc::vec::Vec::new();

    for i in 0..VIRTIO_MMIO_COUNT {
        let base = VIRTIO_MMIO_BASE + i * VIRTIO_MMIO_STRIDE;
        let magic = unsafe { read_volatile(base as *const u32) };
        if magic != VIRTIO_MAGIC {
            continue;
        }

        let device_id = unsafe { read_volatile((base + DEVICE_ID) as *const u32) };
        if device_id == 0 {
            continue;
        }

        let version = unsafe { read_volatile((base + VERSION) as *const u32) };
        let name = match device_id {
            DEVICE_NET => "network",
            DEVICE_BLOCK => "block",
            DEVICE_INPUT => "input",
            _ => "unknown",
        };

        crate::println!(
            "[virtio] Found {} device (id={}) at {:#x}, version {}",
            name, device_id, base, version
        );

        devices.push(VirtioMmio { base, device_id, version });
    }

    devices
}

use super::{VirtioMmio, queue::{Virtqueue, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE, QUEUE_SIZE}};
use crate::mm::pmm;
use alloc::vec::Vec;

const VIRTIO_NET_HDR_SIZE: usize = 10;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VirtioNetHeader {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
}

const RX_BUF_SIZE: usize = 1526; // ETH_FRAME_MAX + VirtioNetHeader

pub struct VirtioNet {
    mmio: VirtioMmio,
    rx_queue: Virtqueue,
    tx_queue: Virtqueue,
    rx_buffers: Vec<usize>, // Physical addresses of RX buffer pages
    tx_buf: usize,          // Physical address of TX buffer page
    pub mac: [u8; 6],
}

impl VirtioNet {
    pub fn new(mmio: VirtioMmio) -> Option<Self> {
        if !mmio.init_device() {
            return None;
        }

        // Read MAC address from device config (offset 0x100)
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = unsafe {
                core::ptr::read_volatile((mmio.base + 0x100 + i) as *const u8)
            };
        }

        let rx_max = mmio.queue_num_max(0).min(128) as usize;
        let tx_max = mmio.queue_num_max(1).min(128) as usize;

        let mut rx_queue = Virtqueue::new(rx_max);
        let mut tx_queue = Virtqueue::new(tx_max);

        mmio.setup_queue(0, &rx_queue);
        mmio.setup_queue(1, &tx_queue);
        mmio.driver_ok();

        // Allocate RX buffers and populate RX queue
        let num_rx_bufs = rx_max.min(32);
        let mut rx_buffers = Vec::with_capacity(num_rx_bufs);

        for _ in 0..num_rx_bufs {
            let page = pmm::alloc_page().expect("OOM for net RX buffer");
            let buf_addr = page.as_usize();
            rx_buffers.push(buf_addr);

            if let Some(desc_idx) = rx_queue.alloc_desc() {
                unsafe {
                    let d = &mut *rx_queue.desc.add(desc_idx as usize);
                    d.addr = buf_addr as u64;
                    d.len = RX_BUF_SIZE as u32;
                    d.flags = VRING_DESC_F_WRITE;
                    d.next = 0;
                }
                rx_queue.push_avail(desc_idx);
            }
        }

        mmio.notify(0); // Notify device that RX buffers are ready

        // Allocate TX buffer
        let tx_buf = pmm::alloc_page().expect("OOM for net TX buffer").as_usize();

        crate::println!(
            "[virtio-net] Initialized, MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );

        Some(Self {
            mmio,
            rx_queue,
            tx_queue,
            rx_buffers,
            tx_buf,
            mac,
        })
    }

    pub fn send(&mut self, frame: &[u8]) -> bool {
        if frame.len() + VIRTIO_NET_HDR_SIZE > 4096 {
            return false;
        }

        // Write VirtioNetHeader + frame to TX buffer
        unsafe {
            let header = self.tx_buf as *mut VirtioNetHeader;
            *header = VirtioNetHeader::default();

            core::ptr::copy_nonoverlapping(
                frame.as_ptr(),
                (self.tx_buf + VIRTIO_NET_HDR_SIZE) as *mut u8,
                frame.len(),
            );
        }

        let desc_idx = match self.tx_queue.alloc_desc() {
            Some(idx) => idx,
            None => return false,
        };

        unsafe {
            let d = &mut *self.tx_queue.desc.add(desc_idx as usize);
            d.addr = self.tx_buf as u64;
            d.len = (VIRTIO_NET_HDR_SIZE + frame.len()) as u32;
            d.flags = 0;
            d.next = 0;
        }

        self.tx_queue.push_avail(desc_idx);
        self.mmio.notify(1);

        // Poll for completion
        loop {
            if let Some((id, _)) = self.tx_queue.pop_used() {
                self.tx_queue.free_desc(id as u16);
                return true;
            }
            core::hint::spin_loop();
        }
    }

    pub fn recv(&mut self) -> Option<Vec<u8>> {
        let (id, len) = self.rx_queue.pop_used()?;

        let buf_addr = unsafe { (*self.rx_queue.desc.add(id as usize)).addr as usize };

        // Skip VirtioNetHeader, copy the Ethernet frame
        let frame_len = len as usize - VIRTIO_NET_HDR_SIZE;
        let mut frame = alloc::vec![0u8; frame_len];
        unsafe {
            core::ptr::copy_nonoverlapping(
                (buf_addr + VIRTIO_NET_HDR_SIZE) as *const u8,
                frame.as_mut_ptr(),
                frame_len,
            );
        }

        // Re-submit the buffer to RX queue
        unsafe {
            let d = &mut *self.rx_queue.desc.add(id as usize);
            d.addr = buf_addr as u64;
            d.len = RX_BUF_SIZE as u32;
            d.flags = VRING_DESC_F_WRITE;
            d.next = 0;
        }
        self.rx_queue.push_avail(id as u16);
        self.mmio.notify(0);

        Some(frame)
    }
}

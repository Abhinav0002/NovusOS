use super::{VirtioMmio, queue::{Virtqueue, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE}};
use crate::mm::pmm;

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

#[repr(C)]
struct VirtioBlkReqHeader {
    req_type: u32,
    reserved: u32,
    sector: u64,
}

pub struct VirtioBlock {
    mmio: VirtioMmio,
    queue: Virtqueue,
    req_buf: usize, // Physical address of request buffer page
}

impl VirtioBlock {
    pub fn new(mmio: VirtioMmio) -> Option<Self> {
        if !mmio.init_device() {
            return None;
        }

        let max_queue = mmio.queue_num_max(0);
        let queue_size = max_queue.min(128) as usize;
        let mut queue = Virtqueue::new(queue_size);
        mmio.setup_queue(0, &queue);
        mmio.driver_ok();

        // Allocate a buffer page for requests
        let req_buf = pmm::alloc_page().expect("OOM for block request buffer").as_usize();

        crate::println!("[virtio-blk] Initialized, queue size = {}", queue_size);

        Some(Self { mmio, queue, req_buf })
    }

    pub fn read_sector(&mut self, sector: u64, buf: &mut [u8; 512]) -> bool {
        self.do_request(VIRTIO_BLK_T_IN, sector, buf)
    }

    pub fn write_sector(&mut self, sector: u64, buf: &[u8; 512]) -> bool {
        let mut data = *buf;
        self.do_request(VIRTIO_BLK_T_OUT, sector, &mut data)
    }

    fn do_request(&mut self, req_type: u32, sector: u64, buf: &mut [u8; 512]) -> bool {
        // Layout in req_buf page:
        // offset 0: VirtioBlkReqHeader (16 bytes)
        // offset 16: data buffer (512 bytes)
        // offset 528: status byte
        let header_addr = self.req_buf;
        let data_addr = self.req_buf + 16;
        let status_addr = self.req_buf + 528;

        unsafe {
            let header = header_addr as *mut VirtioBlkReqHeader;
            (*header).req_type = req_type;
            (*header).reserved = 0;
            (*header).sector = sector;

            // For write, copy data to buffer
            if req_type == VIRTIO_BLK_T_OUT {
                core::ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    data_addr as *mut u8,
                    512,
                );
            }

            // Set status to 0xFF (sentinel)
            *(status_addr as *mut u8) = 0xFF;
        }

        // Build descriptor chain: header -> data -> status
        let d0 = self.queue.alloc_desc().expect("no free desc");
        let d1 = self.queue.alloc_desc().expect("no free desc");
        let d2 = self.queue.alloc_desc().expect("no free desc");

        unsafe {
            let desc0 = &mut *self.queue.desc.add(d0 as usize);
            desc0.addr = header_addr as u64;
            desc0.len = 16;
            desc0.flags = VRING_DESC_F_NEXT;
            desc0.next = d1;

            let desc1 = &mut *self.queue.desc.add(d1 as usize);
            desc1.addr = data_addr as u64;
            desc1.len = 512;
            desc1.flags = VRING_DESC_F_NEXT
                | if req_type == VIRTIO_BLK_T_IN { VRING_DESC_F_WRITE } else { 0 };
            desc1.next = d2;

            let desc2 = &mut *self.queue.desc.add(d2 as usize);
            desc2.addr = status_addr as u64;
            desc2.len = 1;
            desc2.flags = VRING_DESC_F_WRITE;
            desc2.next = 0;
        }

        self.queue.push_avail(d0);
        self.mmio.notify(0);

        // Poll for completion
        loop {
            if let Some((id, _len)) = self.queue.pop_used() {
                self.queue.free_desc(d2);
                self.queue.free_desc(d1);
                self.queue.free_desc(d0);

                let status = unsafe { *(status_addr as *const u8) };
                if req_type == VIRTIO_BLK_T_IN && status == 0 {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            data_addr as *const u8,
                            buf.as_mut_ptr(),
                            512,
                        );
                    }
                }
                return status == 0;
            }
            core::hint::spin_loop();
        }
    }
}

use crate::mm::{pmm, PAGE_SIZE};

pub const QUEUE_SIZE: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

pub const VRING_DESC_F_NEXT: u16 = 1;
pub const VRING_DESC_F_WRITE: u16 = 2;

#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; QUEUE_SIZE],
}

pub struct Virtqueue {
    pub desc: *mut VirtqDesc,
    pub avail: *mut VirtqAvail,
    pub used: *mut VirtqUsed,
    pub size: usize,
    pub free_head: u16,
    pub num_free: u16,
    pub last_used_idx: u16,
}

unsafe impl Send for Virtqueue {}

impl Virtqueue {
    /// Creates a legacy-compatible virtqueue with contiguous memory layout:
    /// [descriptors][available ring][padding to 4096][used ring]
    pub fn new(size: usize) -> Self {
        let desc_size = size * core::mem::size_of::<VirtqDesc>();
        let avail_size = 6 + 2 * size; // flags(2) + idx(2) + ring(2*size) + used_event(2)

        // Legacy layout: used ring must be page-aligned after desc+avail
        let used_offset = align_up(desc_size + avail_size, 4096);
        let used_size = 6 + 8 * size;  // flags(2) + idx(2) + ring(8*size) + avail_event(2)

        let total = used_offset + used_size;
        let pages_needed = (total + PAGE_SIZE - 1) / PAGE_SIZE;

        let first_page = pmm::alloc_page().expect("OOM allocating virtqueue");
        let base = first_page.as_usize();
        for _ in 1..pages_needed {
            pmm::alloc_page().expect("OOM allocating virtqueue");
        }

        let desc = base as *mut VirtqDesc;
        let avail = (base + desc_size) as *mut VirtqAvail;
        let used = (base + used_offset) as *mut VirtqUsed;

        // Initialize free list
        unsafe {
            for i in 0..size {
                let d = &mut *desc.add(i);
                d.addr = 0;
                d.len = 0;
                d.flags = 0;
                d.next = if i + 1 < size { (i + 1) as u16 } else { 0 };
            }

            let a = &mut *avail;
            a.flags = 0;
            a.idx = 0;

            let u = &mut *used;
            u.flags = 0;
            u.idx = 0;
        }

        Self {
            desc,
            avail,
            used,
            size,
            free_head: 0,
            num_free: size as u16,
            last_used_idx: 0,
        }
    }

    pub fn desc_addr(&self) -> u64 {
        self.desc as u64
    }

    pub fn avail_addr(&self) -> u64 {
        self.avail as u64
    }

    pub fn used_addr(&self) -> u64 {
        self.used as u64
    }

    pub fn alloc_desc(&mut self) -> Option<u16> {
        if self.num_free == 0 {
            return None;
        }
        let idx = self.free_head;
        unsafe {
            self.free_head = (*self.desc.add(idx as usize)).next;
        }
        self.num_free -= 1;
        Some(idx)
    }

    pub fn free_desc(&mut self, idx: u16) {
        unsafe {
            let d = &mut *self.desc.add(idx as usize);
            d.addr = 0;
            d.len = 0;
            d.flags = 0;
            d.next = self.free_head;
        }
        self.free_head = idx;
        self.num_free += 1;
    }

    pub fn push_avail(&mut self, desc_idx: u16) {
        unsafe {
            let a = &mut *self.avail;
            let idx = a.idx as usize % self.size;
            a.ring[idx] = desc_idx;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            a.idx = a.idx.wrapping_add(1);
        }
    }

    pub fn pop_used(&mut self) -> Option<(u32, u32)> {
        unsafe {
            let u = &*self.used;
            if self.last_used_idx == u.idx {
                return None;
            }
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            let idx = self.last_used_idx as usize % self.size;
            let elem = u.ring[idx];
            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            Some((elem.id, elem.len))
        }
    }
}

fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

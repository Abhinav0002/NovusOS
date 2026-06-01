pub mod pmm;
pub mod vmm;
pub mod heap;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

pub const KERNEL_VA_BASE: usize = 0xFFFF_0000_0000_0000;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

impl PhysAddr {
    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn page_aligned(self) -> bool {
        self.0 & (PAGE_SIZE - 1) == 0
    }

    pub fn to_virt(self) -> VirtAddr {
        VirtAddr(self.0 + KERNEL_VA_BASE)
    }
}

impl VirtAddr {
    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn to_phys(self) -> PhysAddr {
        PhysAddr(self.0 - KERNEL_VA_BASE)
    }
}

pub fn init() {
    let ram_base = 0x4000_0000usize;
    let ram_size = 256 * 1024 * 1024; // 256 MB

    unsafe {
        pmm::init(ram_base, ram_size);
    }

    crate::println!(
        "[mm] Physical memory: {} pages free / {} total",
        pmm::free_pages(),
        pmm::total_pages()
    );
}

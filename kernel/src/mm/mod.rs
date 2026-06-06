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

pub fn init_from_boot_info(bi: &boot_info::BootInfo) {
    use boot_info::MemoryRegionKind;

    let mut ram_base = u64::MAX;
    let mut ram_end = 0u64;

    for i in 0..bi.memory_region_count as usize {
        let region = &bi.memory_regions[i];
        if region.kind == MemoryRegionKind::Usable {
            if region.start < ram_base {
                ram_base = region.start;
            }
            let end = region.start + region.size;
            if end > ram_end {
                ram_end = end;
            }
        }
    }

    let ram_base = ram_base as usize;
    let ram_size = (ram_end - ram_base as u64) as usize;

    unsafe {
        pmm::init(ram_base, ram_size);
    }

    for i in 0..bi.memory_region_count as usize {
        let region = &bi.memory_regions[i];
        match region.kind {
            MemoryRegionKind::Usable => {}
            _ => unsafe {
                pmm::mark_region_used(region.start as usize, region.size as usize);
            },
        }
    }

    crate::println!(
        "[mm] Physical memory (BootInfo): {} pages free / {} total",
        pmm::free_pages(),
        pmm::total_pages()
    );
}

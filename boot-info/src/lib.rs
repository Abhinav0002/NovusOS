#![no_std]

pub const BOOT_INFO_MAGIC: u64 = 0x4E4F_5655_5342_4F4F;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Bgr = 0,
    Rgb = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FramebufferInfo {
    pub base_phys: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable = 0,
    Reserved = 1,
    AcpiReclaimable = 2,
    Bootloader = 3,
    Kernel = 4,
    Framebuffer = 5,
    BootInfo = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: u64,
    pub kind: MemoryRegionKind,
}

pub const MAX_MEMORY_REGIONS: usize = 128;

#[repr(C)]
pub struct BootInfo {
    pub magic: u64,
    pub framebuffer: FramebufferInfo,
    pub memory_region_count: u32,
    pub _padding: u32,
    pub memory_regions: [MemoryRegion; MAX_MEMORY_REGIONS],
}

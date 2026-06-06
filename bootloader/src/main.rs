#![no_main]
#![no_std]

extern crate alloc;

use boot_info::*;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::mem::memory_map::{MemoryMap, MemoryType};
use core::ptr;

static KERNEL_BIN: &[u8] = include_bytes!("../../target/kernel.bin");

const KERNEL_LOAD_ADDR: usize = 0x4008_0000;
const BOOT_INFO_ADDR: usize = 0x4007_0000;

fn find_gop_framebuffer() -> FramebufferInfo {
    let gop_handle = uefi::boot::get_handle_for_protocol::<GraphicsOutput>()
        .expect("GOP not found");
    let mut gop = uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)
        .expect("Failed to open GOP");

    let mut best_mode = 0;
    let mut best_res = 0u64;
    for i in 0..gop.modes().count() {
        let mode = gop.modes().nth(i).unwrap();
        let info = mode.info();
        let res = info.resolution().0 as u64 * info.resolution().1 as u64;
        if info.resolution() == (1024, 768) {
            best_mode = i;
            break;
        }
        if res > best_res {
            best_res = res;
            best_mode = i;
        }
    }

    let mode = gop.modes().nth(best_mode).unwrap();
    let _ = gop.set_mode(&mode);

    let mode_info = gop.current_mode_info();
    let (width, height) = mode_info.resolution();
    let stride = mode_info.stride();
    let pixel_format = match mode_info.pixel_format() {
        uefi::proto::console::gop::PixelFormat::Bgr => PixelFormat::Bgr,
        _ => PixelFormat::Rgb,
    };

    let fb_base = gop.frame_buffer().as_mut_ptr() as u64;
    let fb_size = (stride * height * 4) as u64;

    FramebufferInfo {
        base_phys: fb_base,
        size: fb_size,
        width: width as u32,
        height: height as u32,
        stride: stride as u32,
        pixel_format,
    }
}

fn build_memory_map(bi: &mut BootInfo, fb: &FramebufferInfo) {
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA)
        .expect("Failed to get memory map");

    let mut count = 0usize;
    for desc in mmap.entries() {
        if count >= MAX_MEMORY_REGIONS {
            break;
        }

        let kind = match desc.ty {
            MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA => MemoryRegionKind::Usable,
            MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA => MemoryRegionKind::Bootloader,
            MemoryType::ACPI_RECLAIM => MemoryRegionKind::AcpiReclaimable,
            _ => MemoryRegionKind::Reserved,
        };

        bi.memory_regions[count] = MemoryRegion {
            start: desc.phys_start,
            size: desc.page_count * 4096,
            kind,
        };
        count += 1;
    }

    // Mark kernel region
    if count < MAX_MEMORY_REGIONS {
        bi.memory_regions[count] = MemoryRegion {
            start: KERNEL_LOAD_ADDR as u64,
            size: KERNEL_BIN.len() as u64 + 0x10000, // kernel + 64KB stack
            kind: MemoryRegionKind::Kernel,
        };
        count += 1;
    }

    // Mark framebuffer region
    if count < MAX_MEMORY_REGIONS {
        bi.memory_regions[count] = MemoryRegion {
            start: fb.base_phys,
            size: fb.size,
            kind: MemoryRegionKind::Framebuffer,
        };
        count += 1;
    }

    // Mark BootInfo region
    if count < MAX_MEMORY_REGIONS {
        bi.memory_regions[count] = MemoryRegion {
            start: BOOT_INFO_ADDR as u64,
            size: 4096,
            kind: MemoryRegionKind::BootInfo,
        };
        count += 1;
    }

    bi.memory_region_count = count as u32;
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI helpers");

    // Step 1: Find GOP and capture framebuffer info
    let fb_info = find_gop_framebuffer();

    // Write 'B' to UART for debug
    unsafe {
        ptr::write_volatile(0x0900_0000 as *mut u8, b'B');
    }

    // Step 2: Copy kernel to load address
    unsafe {
        ptr::copy_nonoverlapping(
            KERNEL_BIN.as_ptr(),
            KERNEL_LOAD_ADDR as *mut u8,
            KERNEL_BIN.len(),
        );
    }

    // Step 3: Build BootInfo at known address
    let bi = unsafe { &mut *(BOOT_INFO_ADDR as *mut BootInfo) };
    bi.magic = BOOT_INFO_MAGIC;
    bi.framebuffer = fb_info;
    bi._padding = 0;
    build_memory_map(bi, &fb_info);

    // Write 'I' to UART for debug
    unsafe {
        ptr::write_volatile(0x0900_0000 as *mut u8, b'I');
    }

    // Step 4: Exit boot services
    let _ = unsafe { uefi::boot::exit_boot_services(MemoryType::LOADER_DATA) };

    // Write 'K' to UART for debug (post-ExitBootServices, UART still works)
    unsafe {
        ptr::write_volatile(0x0900_0000 as *mut u8, b'K');
        ptr::write_volatile(0x0900_0000 as *mut u8, b'\n');
    }

    // Step 5: Trampoline — disable MMU, flush caches, jump to kernel
    unsafe {
        core::arch::asm!(
            // Disable MMU, I-cache, D-cache
            "mrs    x2, sctlr_el1",
            "bic    x2, x2, #1",          // M: MMU off
            "bic    x2, x2, #(1 << 2)",   // C: D-cache off
            "bic    x2, x2, #(1 << 12)",  // I: I-cache off
            "msr    sctlr_el1, x2",
            "isb",

            // Invalidate TLB
            "tlbi   vmalle1",
            "dsb    sy",
            "isb",

            // Invalidate I-cache
            "ic     iallu",
            "dsb    sy",
            "isb",

            // Jump to kernel with BootInfo pointer in x0
            "mov    x0, {boot_info}",
            "br     {kernel_entry}",

            boot_info = in(reg) BOOT_INFO_ADDR,
            kernel_entry = in(reg) KERNEL_LOAD_ADDR,
            options(noreturn)
        );
    }
}

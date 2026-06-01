use crate::mm::{pmm, vmm, VirtAddr, PAGE_SIZE};
use crate::mm::vmm::PageTable;

const USER_STACK_TOP: usize = 0x8000_0000;
const USER_STACK_PAGES: usize = 4; // 16 KB user stack

static INIT_ELF: &[u8] = include_bytes!("../../init.elf");

#[repr(C)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
}

#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;

pub fn spawn_init() {
    crate::println!("[process] Loading init ELF ({} bytes)", INIT_ELF.len());

    // Validate ELF magic
    if INIT_ELF.len() < 64 || &INIT_ELF[0..4] != b"\x7FELF" {
        crate::println!("[process] Invalid ELF");
        return;
    }

    let header = unsafe { &*(INIT_ELF.as_ptr() as *const Elf64Header) };

    if header.e_machine != 0xB7 {
        crate::println!("[process] Not AArch64 ELF");
        return;
    }

    let entry = header.e_entry as usize;
    crate::println!("[process] Entry point: {:#x}", entry);

    // Allocate a fresh L0 page table for user address space
    let l0_page = pmm::alloc_page().expect("OOM for user L0 page table");
    let l0 = unsafe { &mut *(l0_page.as_usize() as *mut vmm::PageTable) };

    // Load PT_LOAD segments
    let phdr_base = unsafe { INIT_ELF.as_ptr().add(header.e_phoff as usize) };
    for i in 0..header.e_phnum {
        let phdr = unsafe { &*(phdr_base.add(i as usize * header.e_phentsize as usize) as *const Elf64Phdr) };

        if phdr.p_type != PT_LOAD {
            continue;
        }

        let vaddr_start = phdr.p_vaddr as usize;
        let memsz = phdr.p_memsz as usize;
        let filesz = phdr.p_filesz as usize;
        let offset = phdr.p_offset as usize;

        let executable = phdr.p_flags & PF_X != 0;
        let writable = phdr.p_flags & PF_W != 0;

        crate::println!(
            "[process] LOAD: vaddr={:#x} memsz={:#x} filesz={:#x} {}{}",
            vaddr_start, memsz, filesz,
            if executable { "X" } else { "-" },
            if writable { "W" } else { "R" },
        );

        // Map pages for this segment
        let page_start = vaddr_start & !(PAGE_SIZE - 1);
        let page_end = (vaddr_start + memsz + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        for va in (page_start..page_end).step_by(PAGE_SIZE) {
            let pa = pmm::alloc_page().expect("OOM mapping user segment");

            // Zero the page first
            unsafe {
                core::ptr::write_bytes(pa.as_usize() as *mut u8, 0, PAGE_SIZE);
            }

            // Copy file data if within filesz range
            let page_offset_in_seg = va.saturating_sub(vaddr_start);
            if page_offset_in_seg < filesz {
                let src_offset = offset + page_offset_in_seg;
                let copy_start = if va < vaddr_start { vaddr_start - va } else { 0 };
                let copy_len = (filesz - page_offset_in_seg).min(PAGE_SIZE - copy_start);
                if src_offset + copy_len <= INIT_ELF.len() {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            INIT_ELF.as_ptr().add(src_offset),
                            (pa.as_usize() + copy_start) as *mut u8,
                            copy_len,
                        );
                    }
                }
            }

            let flags = vmm::MappingFlags {
                writable,
                executable,
                user: true,
                device: false,
            };
            unsafe {
                vmm::map_page(l0, VirtAddr(va), pa, &flags);
            }
        }
    }

    // Map user stack pages
    let stack_bottom = USER_STACK_TOP - USER_STACK_PAGES * PAGE_SIZE;
    for i in 0..USER_STACK_PAGES {
        let va = stack_bottom + i * PAGE_SIZE;
        let pa = pmm::alloc_page().expect("OOM mapping user stack");
        unsafe {
            core::ptr::write_bytes(pa.as_usize() as *mut u8, 0, PAGE_SIZE);
            vmm::map_page(l0, VirtAddr(va), pa, &vmm::MappingFlags {
                writable: true,
                executable: false,
                user: true,
                device: false,
            });
        }
    }

    crate::println!("[process] User stack at {:#x}-{:#x}", stack_bottom, USER_STACK_TOP);

    let ttbr0 = l0_page.as_usize() as u64;

    // Merge kernel page table entries into user L0 so that kernel code
    // remains accessible during TTBR0 switch (kernel is at 0x40080000
    // which uses TTBR0's lower half). We walk boot TTBR0 and copy entries
    // at the L1/L2 level, merging with any user mappings.
    unsafe {
        let current_ttbr0: u64;
        core::arch::asm!("mrs {}, TTBR0_EL1", out(reg) current_ttbr0);
        let boot_l0 = &*(current_ttbr0 as *const PageTable);

        for l0_idx in 0..512 {
            let boot_l0e = boot_l0.entries[l0_idx];
            if boot_l0e & 1 == 0 { continue; }

            if l0.entries[l0_idx] & 1 == 0 {
                // No user mapping at this L0 index — just copy
                l0.entries[l0_idx] = boot_l0e;
            } else {
                // Both user and boot have entries — merge at L1 level
                let boot_l1 = &*((boot_l0e & 0xFFFF_FFFF_F000) as *const PageTable);
                let user_l1 = &mut *((l0.entries[l0_idx] & 0xFFFF_FFFF_F000) as *mut PageTable);

                for l1_idx in 0..512 {
                    let boot_l1e = boot_l1.entries[l1_idx];
                    if boot_l1e & 1 == 0 { continue; }

                    if user_l1.entries[l1_idx] & 1 == 0 {
                        user_l1.entries[l1_idx] = boot_l1e;
                    } else {
                        // Both have L1 entries — merge at L2
                        let boot_l1e_is_table = (boot_l1e & 0b10) != 0;
                        let user_l1e_is_table = (user_l1.entries[l1_idx] & 0b10) != 0;

                        if boot_l1e_is_table && user_l1e_is_table {
                            let boot_l2 = &*((boot_l1e & 0xFFFF_FFFF_F000) as *const PageTable);
                            let user_l2 = &mut *((user_l1.entries[l1_idx] & 0xFFFF_FFFF_F000) as *mut PageTable);
                            for l2_idx in 0..512 {
                                if boot_l2.entries[l2_idx] & 1 != 0 && user_l2.entries[l2_idx] & 1 == 0 {
                                    user_l2.entries[l2_idx] = boot_l2.entries[l2_idx];
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    crate::println!("[process] Jumping to EL0 at {:#x}", entry);

    unsafe {
        core::arch::asm!(
            "msr TTBR0_EL1, {ttbr0}",
            "isb",
            "tlbi vmalle1",
            "dsb sy",
            "isb",
            "msr SP_EL0, {sp_el0}",
            "msr ELR_EL1, {elr}",
            "msr SPSR_EL1, xzr",
            "eret",
            ttbr0 = in(reg) ttbr0,
            sp_el0 = in(reg) USER_STACK_TOP as u64,
            elr = in(reg) entry as u64,
            options(noreturn)
        );
    }
}

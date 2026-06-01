use super::{PhysAddr, VirtAddr, KERNEL_VA_BASE};
use super::pmm;

const ENTRIES_PER_TABLE: usize = 512;

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u64; ENTRIES_PER_TABLE],
}

// Page table entry flags
const PTE_VALID: u64 = 1 << 0;
const PTE_TABLE: u64 = 1 << 1; // L0-L2: table descriptor; L3: page descriptor
const PTE_AF: u64 = 1 << 10;   // Access flag
const PTE_SH_INNER: u64 = 3 << 8; // Inner shareable
const PTE_AP_RW_EL1: u64 = 0 << 6;
const PTE_AP_RO_EL1: u64 = 2 << 6;
const PTE_AP_RW_ALL: u64 = 1 << 6;
const PTE_UXN: u64 = 1 << 54;
const PTE_PXN: u64 = 1 << 53;

// MAIR attribute indices
const MAIR_NORMAL_IDX: u64 = 0;
const MAIR_DEVICE_IDX: u64 = 1;

fn attr_index(idx: u64) -> u64 {
    idx << 2
}

pub struct MappingFlags {
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub device: bool,
}

impl MappingFlags {
    pub fn kernel_code() -> Self {
        Self { writable: false, executable: true, user: false, device: false }
    }

    pub fn kernel_data() -> Self {
        Self { writable: true, executable: false, user: false, device: false }
    }

    pub fn kernel_device() -> Self {
        Self { writable: true, executable: false, user: false, device: true }
    }
}

fn flags_to_pte(flags: &MappingFlags) -> u64 {
    let mut pte = PTE_VALID | PTE_TABLE | PTE_AF;

    if flags.device {
        pte |= attr_index(MAIR_DEVICE_IDX);
    } else {
        pte |= attr_index(MAIR_NORMAL_IDX) | PTE_SH_INNER;
    }

    if flags.user {
        if flags.writable {
            pte |= PTE_AP_RW_ALL; // AP[2:1]=01: EL0+EL1 read-write
        } else {
            pte |= PTE_AP_RW_ALL | PTE_AP_RO_EL1; // AP[2:1]=11: EL0+EL1 read-only
        }
        if !flags.executable {
            pte |= PTE_UXN;
        }
        pte |= PTE_PXN; // never execute user pages at EL1
    } else {
        if flags.writable {
            pte |= PTE_AP_RW_EL1;
        } else {
            pte |= PTE_AP_RO_EL1;
        }
        if !flags.executable {
            pte |= PTE_UXN | PTE_PXN;
        }
    }

    pte
}

fn va_l0_index(va: usize) -> usize { (va >> 39) & 0x1FF }
fn va_l1_index(va: usize) -> usize { (va >> 30) & 0x1FF }
fn va_l2_index(va: usize) -> usize { (va >> 21) & 0x1FF }
fn va_l3_index(va: usize) -> usize { (va >> 12) & 0x1FF }

unsafe fn get_or_create_table(table: &mut PageTable, index: usize) -> &mut PageTable {
    let entry = table.entries[index];
    if entry & PTE_VALID != 0 {
        let next_phys = (entry & 0x0000_FFFF_FFFF_F000) as usize;
        &mut *(next_phys as *mut PageTable)
    } else {
        let new_page = pmm::alloc_page().expect("OOM allocating page table");
        let ptr = new_page.as_usize() as *mut PageTable;
        table.entries[index] = new_page.as_usize() as u64 | PTE_VALID | PTE_TABLE;
        &mut *ptr
    }
}

pub unsafe fn map_page(l0: &mut PageTable, va: VirtAddr, pa: PhysAddr, flags: &MappingFlags) {
    let l1 = get_or_create_table(l0, va_l0_index(va.as_usize()));
    let l2 = get_or_create_table(l1, va_l1_index(va.as_usize()));
    let l3 = get_or_create_table(l2, va_l2_index(va.as_usize()));

    let pte = (pa.as_usize() as u64 & 0x0000_FFFF_FFFF_F000) | flags_to_pte(flags);
    l3.entries[va_l3_index(va.as_usize())] = pte;
}

// Initial boot page tables (statically allocated, used before PMM is ready)
#[repr(C, align(4096))]
struct BootPageTables {
    l0: PageTable,
    l1_low: PageTable,  // Identity map for first 1GB
    l1_high: PageTable,  // Higher-half map
    l2_low: [PageTable; 2],
    l2_high: [PageTable; 2],
}

static mut BOOT_TABLES: BootPageTables = BootPageTables {
    l0: PageTable { entries: [0; ENTRIES_PER_TABLE] },
    l1_low: PageTable { entries: [0; ENTRIES_PER_TABLE] },
    l1_high: PageTable { entries: [0; ENTRIES_PER_TABLE] },
    l2_low: [
        PageTable { entries: [0; ENTRIES_PER_TABLE] },
        PageTable { entries: [0; ENTRIES_PER_TABLE] },
    ],
    l2_high: [
        PageTable { entries: [0; ENTRIES_PER_TABLE] },
        PageTable { entries: [0; ENTRIES_PER_TABLE] },
    ],
};

// Block descriptor flags (for 2MB blocks at L2)
const BLOCK_VALID: u64 = 1 << 0;
const BLOCK_AF: u64 = 1 << 10;
const BLOCK_SH_INNER: u64 = 3 << 8;

pub unsafe fn init_boot_page_tables() {
    // We use 2MB block mappings at L2 level for simplicity during boot
    // This avoids needing L3 tables

    // L0[0] -> l1_low (for identity map, VA 0x0000_0000_0000_0000 - 0x0000_007F_FFFF_FFFF)
    let l1_low_addr = &raw const BOOT_TABLES.l1_low as *const _ as u64;
    BOOT_TABLES.l0.entries[0] = l1_low_addr | PTE_VALID | PTE_TABLE;

    // L0[511] -> l1_high (for higher-half, VA 0xFFFF_FF80_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF)
    // Actually for KERNEL_VA_BASE = 0xFFFF_0000_0000_0000, L0 index = (0xFFFF_0000_0000_0000 >> 39) & 0x1FF
    let l0_high_idx = va_l0_index(KERNEL_VA_BASE);
    let l1_high_addr = &raw const BOOT_TABLES.l1_high as *const _ as u64;
    BOOT_TABLES.l0.entries[l0_high_idx] = l1_high_addr | PTE_VALID | PTE_TABLE;

    // Identity map: first 1GB as device memory (MMIO region 0x0000_0000 - 0x3FFF_FFFF)
    // L1_low[0] -> l2_low[0] (first 512 * 2MB = 1GB)
    let l2_low0_addr = &raw const BOOT_TABLES.l2_low[0] as *const _ as u64;
    BOOT_TABLES.l1_low.entries[0] = l2_low0_addr | PTE_VALID | PTE_TABLE;

    // Map first 1GB as device-nGnRnE 2MB blocks
    for i in 0..512 {
        let phys = (i as u64) << 21;
        BOOT_TABLES.l2_low[0].entries[i] = phys | BLOCK_VALID | BLOCK_AF
            | attr_index(MAIR_DEVICE_IDX);
    }

    // Identity map: RAM at 0x4000_0000 (L1_low[1])
    let l2_low1_addr = &raw const BOOT_TABLES.l2_low[1] as *const _ as u64;
    BOOT_TABLES.l1_low.entries[1] = l2_low1_addr | PTE_VALID | PTE_TABLE;

    // Map 1GB of RAM (0x4000_0000 - 0x7FFF_FFFF) as normal memory
    for i in 0..512 {
        let phys = 0x4000_0000 + ((i as u64) << 21);
        BOOT_TABLES.l2_low[1].entries[i] = phys | BLOCK_VALID | BLOCK_AF
            | attr_index(MAIR_NORMAL_IDX) | BLOCK_SH_INNER;
    }

    // Higher-half: map MMIO (same as identity, at 0xFFFF_0000_0000_0000)
    // L1_high[0] -> l2_high[0]
    let l2_high0_addr = &raw const BOOT_TABLES.l2_high[0] as *const _ as u64;
    BOOT_TABLES.l1_high.entries[0] = l2_high0_addr | PTE_VALID | PTE_TABLE;

    for i in 0..512 {
        let phys = (i as u64) << 21;
        BOOT_TABLES.l2_high[0].entries[i] = phys | BLOCK_VALID | BLOCK_AF
            | attr_index(MAIR_DEVICE_IDX);
    }

    // Higher-half: map RAM at 0xFFFF_0000_4000_0000
    // L1 index for 0xFFFF_0000_4000_0000 = (0x4000_0000 >> 30) & 0x1FF = 1
    let l2_high1_addr = &raw const BOOT_TABLES.l2_high[1] as *const _ as u64;
    BOOT_TABLES.l1_high.entries[1] = l2_high1_addr | PTE_VALID | PTE_TABLE;

    for i in 0..512 {
        let phys = 0x4000_0000 + ((i as u64) << 21);
        BOOT_TABLES.l2_high[1].entries[i] = phys | BLOCK_VALID | BLOCK_AF
            | attr_index(MAIR_NORMAL_IDX) | BLOCK_SH_INNER;
    }
}

pub unsafe fn enable_mmu() {
    let ttbr0 = &raw const BOOT_TABLES.l0 as *const _ as u64;
    let ttbr1 = &raw const BOOT_TABLES.l0 as *const _ as u64;

    // MAIR: index 0 = normal (write-back), index 1 = device-nGnRnE
    let mair: u64 = 0xFF | (0x00 << 8);

    // TCR_EL1:
    // T0SZ = 16 (48-bit VA for TTBR0)
    // T1SZ = 16 (48-bit VA for TTBR1)
    // TG0 = 0b00 (4KB granule for TTBR0)
    // TG1 = 0b10 (4KB granule for TTBR1)
    // SH0 = SH1 = 0b11 (inner shareable)
    // ORGN0 = IRGN0 = ORGN1 = IRGN1 = 0b01 (write-back, write-allocate)
    // IPS = 0b010 (40-bit PA)
    let tcr: u64 = (16)                 // T0SZ
        | (16 << 16)                    // T1SZ
        | (0b00 << 14)                  // TG0
        | (0b10 << 30)                  // TG1
        | (0b11 << 12)                  // SH0
        | (0b11 << 28)                  // SH1
        | (0b01 << 10)                  // ORGN0
        | (0b01 << 8)                   // IRGN0
        | (0b01 << 26)                  // ORGN1
        | (0b01 << 24)                  // IRGN1
        | (0b010 << 32);               // IPS (40-bit)

    core::arch::asm!(
        "msr MAIR_EL1, {mair}",
        "msr TCR_EL1, {tcr}",
        "msr TTBR0_EL1, {ttbr0}",
        "msr TTBR1_EL1, {ttbr1}",
        "isb",
        // Enable MMU + caches
        "mrs {tmp}, SCTLR_EL1",
        "orr {tmp}, {tmp}, #1",        // M bit (MMU enable)
        "orr {tmp}, {tmp}, #(1 << 2)", // C bit (data cache)
        "orr {tmp}, {tmp}, #(1 << 12)", // I bit (instruction cache)
        "msr SCTLR_EL1, {tmp}",
        "isb",
        mair = in(reg) mair,
        tcr = in(reg) tcr,
        ttbr0 = in(reg) ttbr0,
        ttbr1 = in(reg) ttbr1,
        tmp = out(reg) _,
    );

    crate::println!("[vmm] MMU enabled with identity + higher-half mapping");
}

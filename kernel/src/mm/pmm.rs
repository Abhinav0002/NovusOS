use super::{PhysAddr, PAGE_SIZE, PAGE_SHIFT};
use core::sync::atomic::{AtomicUsize, Ordering};

const MAX_PAGES: usize = 65536; // 256MB / 4KB
const BITMAP_SIZE: usize = MAX_PAGES / 64;

static mut BITMAP: [u64; BITMAP_SIZE] = [0; BITMAP_SIZE];
static mut BASE_ADDR: usize = 0;
static mut NUM_PAGES: usize = 0;
static FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    static __kernel_end: u8;
}

pub unsafe fn init(ram_base: usize, ram_size: usize) {
    BASE_ADDR = ram_base;
    NUM_PAGES = ram_size / PAGE_SIZE;

    // Mark all pages as free (0 = free, 1 = used)
    for i in 0..BITMAP_SIZE {
        BITMAP[i] = 0;
    }

    // Mark pages before and including the kernel as used
    let kernel_end = &__kernel_end as *const u8 as usize;
    // Also account for the boot stack (64KB after __kernel_end in linker script)
    let used_end = kernel_end + 0x10000;
    let used_pages = (used_end - ram_base + PAGE_SIZE - 1) / PAGE_SIZE;

    for i in 0..used_pages {
        mark_used(i);
    }

    FREE_COUNT.store(NUM_PAGES - used_pages, Ordering::Relaxed);
}

unsafe fn mark_used(page_idx: usize) {
    let word = page_idx / 64;
    let bit = page_idx % 64;
    BITMAP[word] |= 1 << bit;
}

unsafe fn mark_free(page_idx: usize) {
    let word = page_idx / 64;
    let bit = page_idx % 64;
    BITMAP[word] &= !(1 << bit);
}

fn is_free(page_idx: usize) -> bool {
    unsafe {
        let word = page_idx / 64;
        let bit = page_idx % 64;
        (BITMAP[word] & (1 << bit)) == 0
    }
}

pub fn alloc_page() -> Option<PhysAddr> {
    unsafe {
        for word_idx in 0..BITMAP_SIZE {
            if BITMAP[word_idx] != u64::MAX {
                let bit = (!BITMAP[word_idx]).trailing_zeros() as usize;
                let page_idx = word_idx * 64 + bit;
                if page_idx >= NUM_PAGES {
                    return None;
                }
                mark_used(page_idx);
                FREE_COUNT.fetch_sub(1, Ordering::Relaxed);
                let addr = BASE_ADDR + page_idx * PAGE_SIZE;

                // Zero the page
                core::ptr::write_bytes(addr as *mut u8, 0, PAGE_SIZE);

                return Some(PhysAddr(addr));
            }
        }
    }
    None
}

pub fn free_page(addr: PhysAddr) {
    unsafe {
        let page_idx = (addr.0 - BASE_ADDR) >> PAGE_SHIFT;
        if page_idx < NUM_PAGES && !is_free(page_idx) {
            mark_free(page_idx);
            FREE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn free_pages() -> usize {
    FREE_COUNT.load(Ordering::Relaxed)
}

pub fn total_pages() -> usize {
    unsafe { NUM_PAGES }
}

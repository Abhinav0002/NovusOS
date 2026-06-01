use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use super::{pmm, PAGE_SIZE};

const HEAP_SIZE: usize = 1024 * 1024;  // 1 MB initial heap

struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

const MIN_BLOCK_SIZE: usize = core::mem::size_of::<FreeBlock>();

pub struct LinkedListAllocator {
    head: *mut FreeBlock,
    initialized: bool,
}

unsafe impl Send for LinkedListAllocator {}
unsafe impl Sync for LinkedListAllocator {}

impl LinkedListAllocator {
    const fn new() -> Self {
        Self {
            head: null_mut(),
            initialized: false,
        }
    }

    unsafe fn init(&mut self) {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let first_page = pmm::alloc_page().expect("OOM during heap init");
        let heap_start = first_page.as_usize();

        for _ in 1..num_pages {
            pmm::alloc_page().expect("OOM during heap init");
        }

        let block = heap_start as *mut FreeBlock;
        (*block).size = HEAP_SIZE;
        (*block).next = null_mut();
        self.head = block;
        self.initialized = true;

        crate::println!(
            "[heap] Initialized {} KB at {:#x}",
            HEAP_SIZE / 1024,
            heap_start
        );
    }

    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    unsafe fn alloc_inner(&mut self, layout: Layout) -> *mut u8 {
        let size = Self::align_up(layout.size().max(MIN_BLOCK_SIZE), layout.align());

        let mut prev: *mut FreeBlock = null_mut();
        let mut current = self.head;

        while !current.is_null() {
            let block_addr = current as usize;
            let aligned_addr = Self::align_up(block_addr, layout.align());
            let padding = aligned_addr - block_addr;
            let total_needed = padding + size;

            if (*current).size >= total_needed {
                let remaining = (*current).size - total_needed;

                if remaining >= MIN_BLOCK_SIZE {
                    // Split: create a new free block after the allocated region
                    let new_block = (aligned_addr + size) as *mut FreeBlock;
                    (*new_block).size = remaining;
                    (*new_block).next = (*current).next;

                    if prev.is_null() {
                        self.head = new_block;
                    } else {
                        (*prev).next = new_block;
                    }
                } else {
                    // Use entire block
                    if prev.is_null() {
                        self.head = (*current).next;
                    } else {
                        (*prev).next = (*current).next;
                    }
                }

                return aligned_addr as *mut u8;
            }

            prev = current;
            current = (*current).next;
        }

        null_mut()
    }

    unsafe fn dealloc_inner(&mut self, ptr: *mut u8, layout: Layout) {
        let size = Self::align_up(layout.size().max(MIN_BLOCK_SIZE), layout.align());
        let free_addr = ptr as usize;

        // Insert in address order and merge adjacent blocks
        let new_block = free_addr as *mut FreeBlock;
        (*new_block).size = size;
        (*new_block).next = null_mut();

        if self.head.is_null() || (new_block as usize) < (self.head as usize) {
            (*new_block).next = self.head;
            self.head = new_block;
        } else {
            let mut current = self.head;
            while !(*current).next.is_null()
                && ((*current).next as usize) < free_addr
            {
                current = (*current).next;
            }
            (*new_block).next = (*current).next;
            (*current).next = new_block;
        }

        // Merge with next block
        let block = new_block;
        if !(*block).next.is_null() {
            let block_end = (block as usize) + (*block).size;
            if block_end == (*block).next as usize {
                (*block).size += (*(*block).next).size;
                (*block).next = (*(*block).next).next;
            }
        }

        // Merge previous with current (re-scan from head to find prev)
        let mut current = self.head;
        while !(*current).next.is_null() {
            let current_end = (current as usize) + (*current).size;
            if current_end == (*current).next as usize {
                (*current).size += (*(*current).next).size;
                (*current).next = (*(*current).next).next;
            } else {
                current = (*current).next;
            }
        }
    }
}

struct LockedAllocator {
    inner: spin::Mutex<LinkedListAllocator>,
}

impl LockedAllocator {
    const fn new() -> Self {
        Self {
            inner: spin::Mutex::new(LinkedListAllocator::new()),
        }
    }
}

unsafe impl GlobalAlloc for LockedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.inner.lock();
        allocator.alloc_inner(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.inner.lock();
        allocator.dealloc_inner(ptr, layout);
    }
}

#[global_allocator]
static ALLOCATOR: LockedAllocator = LockedAllocator::new();

pub fn init() {
    unsafe {
        ALLOCATOR.inner.lock().init();
    }
}

// Simple inline spin mutex to avoid external dependency
mod spin {
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicBool, Ordering};

    pub struct Mutex<T> {
        locked: AtomicBool,
        data: UnsafeCell<T>,
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    impl<T> Mutex<T> {
        pub const fn new(data: T) -> Self {
            Self {
                locked: AtomicBool::new(false),
                data: UnsafeCell::new(data),
            }
        }

        pub fn lock(&self) -> MutexGuard<T> {
            while self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                while self.locked.load(Ordering::Relaxed) {
                    core::hint::spin_loop();
                }
            }
            MutexGuard { mutex: self }
        }
    }

    pub struct MutexGuard<'a, T> {
        mutex: &'a Mutex<T>,
    }

    impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &*self.mutex.data.get() }
        }
    }

    impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut *self.mutex.data.get() }
        }
    }

    impl<'a, T> Drop for MutexGuard<'a, T> {
        fn drop(&mut self) {
            self.mutex.locked.store(false, Ordering::Release);
        }
    }
}

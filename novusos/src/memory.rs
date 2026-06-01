use uefi::mem::memory_map::{MemoryMap, MemoryType};

pub struct MemInfo {
    pub total_pages: u64,
    pub free_pages: u64,
}

impl MemInfo {
    pub fn total_mb(&self) -> u64 {
        self.total_pages * 4096 / (1024 * 1024)
    }

    pub fn free_mb(&self) -> u64 {
        self.free_pages * 4096 / (1024 * 1024)
    }
}

pub fn query_memory() -> MemInfo {
    let mut total_pages = 0u64;
    let mut free_pages = 0u64;

    if let Ok(map) = uefi::boot::memory_map(MemoryType::LOADER_DATA) {
        for desc in map.entries() {
            total_pages += desc.page_count;
            match desc.ty {
                MemoryType::CONVENTIONAL
                | MemoryType::BOOT_SERVICES_CODE
                | MemoryType::BOOT_SERVICES_DATA
                | MemoryType::LOADER_CODE
                | MemoryType::LOADER_DATA => {
                    free_pages += desc.page_count;
                }
                _ => {}
            }
        }
    }

    MemInfo { total_pages, free_pages }
}

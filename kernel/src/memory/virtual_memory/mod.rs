use alloc::collections::{BTreeMap, BTreeSet};
use crate::{HHDM_OFFSET, KERNEL_START_VMA_ADDRESS, serial_println};
use crate::memory::{PAGE_SIZE, VirtualAddress};

pub mod paging;
pub mod heap_allocator;

// ---- Tentative virtual memory map ----
// 0x0000000000000000 - 0x00007FFFFFFFFFFF     User-space memory
// 0x0000800000000000 - 0xFFFF7FFFFFFFFFFF     Non-canonical addresses (Unusable)
// 0xFFFF800000000000 - 0xFFFFC87FFFFFFFFF     Direct mapping of physical memory
// 0xFFFFC88000000000 - 0xFFFFC8FFFFFFFFFF     Unused guard hole
// 0xFFFFC90000000000 - 0xFFFFFFFEFFFFFFFF     kernel allocation space
// 0xFFFFFFFF00000000 - 0xFFFFFFFF7FFFFFFF     Unused guard hole
// 0xFFFFFFFF80000000 - 0xFFFFFFFFFFFFFFFF     Kernel mapping

pub struct VirtualMemoryManager {
    free_addresses: BTreeMap<VirtualAddress, usize>,
    free_regions: BTreeMap<usize, VirtualAddress>,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        Self {
            free_addresses: BTreeMap::from([(0xFFFFC90000000000, KERNEL_START_VMA_ADDRESS - 0xFFFFC90000000000)]),
            free_regions:  BTreeMap::from([(KERNEL_START_VMA_ADDRESS - 0xFFFFC90000000000, 0xFFFFC90000000000)])
        }
    }

    /// Allocates a single page in the kernel allocation space region
    pub fn allocate_page(&mut self) -> Option<VirtualAddress> {
        self.allocate_pages(1)
    }

    pub fn allocate_pages(&mut self, count: usize) -> Option<VirtualAddress> {
        let required_size = count * PAGE_SIZE;

        // Find the first region that is big enough to accommodate the allocation request
        let mut keep = 1;
        let region = self.free_regions.extract_if(|size, address| {
            let result = keep > 0;
            if *size >= required_size {
                keep -= 1;
            }

            result
        }).next();

        if let Some(region) = region {
            // Split the region if it is larger than the requested size
            if region.0 > required_size {
                let new_size = region.0 - required_size;
                let new_start_address = region.1 + required_size;

                self.free_regions.insert(new_size, new_start_address);
            }

            // Sync the other tree
            let removed_address = self.free_addresses.remove_entry(&region.1);

            if let Some(removed_address) = removed_address {
                if removed_address.1 != region.0 {
                    panic!("vmm: fatal mismatch between vmemory trees when allocating {} pages", count);
                }

                // Split the region if it is larger than the requested size
                if removed_address.1 > required_size {
                    let new_size = region.0 - required_size;
                    let new_start_address = region.1 + required_size;

                    self.free_addresses.insert(new_start_address, new_size);
                }

                return Some(removed_address.0)
            }

            panic!("vmm: fatal mismatch between vmemory trees when allocating {} pages", count);
        }

        None
    }

    pub fn deallocate_page(&self, ) {
        unimplemented!()
    }

    pub fn deallocate_pages(&self, ) {
        unimplemented!()
    }
}
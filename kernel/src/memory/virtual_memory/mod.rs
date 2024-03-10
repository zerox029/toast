use alloc::collections::{BTreeMap};
use crate::{KERNEL_START_VMA_ADDRESS};
use crate::memory::{PAGE_SIZE, VirtualAddress};

pub mod paging;
pub mod heap_allocator;

/// # Tentative virtual memory map
///
/// | Start Address      |   | End Address        |   | Usage                              |
/// |--------------------|---|--------------------|---|------------------------------------|
/// | 0x0000000000000000 |   | 0x00007FFFFFFFFFFF |   | User-space memory                  |
/// | 0x0000800000000000 |   | 0xFFFF7FFFFFFFFFFF |   | Non-canonical addresses (Unusable) |
/// | 0xFFFF800000000000 |   | 0xFFFFC87FFFFFFFFF |   | Direct mapping of physical memory  |
/// | 0xFFFFC88000000000 |   | 0xFFFFC8FFFFFFFFFF |   | Unused guard hole                  |
/// | 0xFFFFC90000000000 |   | 0xFFFFFFFEFFFFFFFF |   | kernel allocation space            |
/// | 0xFFFFFFFF00000000 |   | 0xFFFFFFFF7FFFFFFF |   | Unused guard hole                  |
/// | 0xFFFFFFFF80000000 |   | 0xFFFFFFFFFFFFFFFF |   | Kernel mapping                     |

/// Used to index the free_regions BTree since there can be multiple nodes with the same size
/// and all nodes in a BTree need to be unique. An index is added to guarantee uniqueness.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
struct SizeKey {
    size: usize,
    index: VirtualAddress,
}

pub struct VirtualMemoryManager {
    free_addresses: BTreeMap<VirtualAddress, usize>,
    free_regions: BTreeMap<SizeKey, VirtualAddress>,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        let free_addresses = BTreeMap::from([(0xFFFFC90000000000, KERNEL_START_VMA_ADDRESS - 0xFFFFC90000000000)]);

        let free_region_key = SizeKey {
            size: KERNEL_START_VMA_ADDRESS - 0xFFFFC90000000000,
            index: 0xFFFFC90000000000,
        };
        let free_regions = BTreeMap::from([(free_region_key, 0xFFFFC90000000000)]);

        Self {
            free_addresses,
            free_regions,
        }
    }

    /// Allocates a single page in the kernel allocation space region
    pub fn allocate_page(&mut self) -> Option<VirtualAddress> {
        self.allocate_pages(1)
    }

    pub fn allocate_pages(&mut self, count: usize) -> Option<VirtualAddress> {
        let required_size = count * PAGE_SIZE;

        // Find the first region that is big enough to accommodate the allocation request
        let mut region = None;
        let split_key = match self.free_regions.range(SizeKey{size: required_size, index: 0}..).next() {
            Some((k, _)) => *k,
            None => panic!("vmm: could not allocate memory")
        };
        if let Some((removed_key, _)) = self.free_regions.split_off(&split_key).into_iter().next() {
            region = self.free_regions.remove_entry(&removed_key);
        }

        if let Some(region) = region {
            // Shrink the region if it can fit the requested size
            if region.0.size > required_size {
                let new_size = region.0.size - required_size;
                let new_start_address = region.1 + required_size;

                self.free_regions.insert(SizeKey{size: new_size, index: region.0.index}, new_start_address);
            }

            // Sync the other tree
            let removed_address = self.free_addresses.remove_entry(&region.1);

            if let Some(removed_address) = removed_address {
                if removed_address.1 != region.0.size {
                    panic!("vmm: fatal mismatch between vmemory trees when allocating {} pages", count);
                }

                // Shrink the region if it can fit the requested size
                if removed_address.1 > required_size {
                    let new_size = region.0.size - required_size;
                    let new_start_address = region.1 + required_size;

                    self.free_addresses.insert(new_start_address, new_size);
                }

                return Some(removed_address.0)
            }

            panic!("vmm: fatal mismatch between vmemory trees when allocating {} pages", count);
        }

        None
    }

    pub fn deallocate_page(&mut self, address: VirtualAddress) {
        self.deallocate_pages(address, PAGE_SIZE);
    }

    pub fn deallocate_pages(&mut self, start_address: VirtualAddress, size: usize) {
        // If neighbouring region is unallocated, merge it with the one currently being freed
        if let Some(left_region) = self.free_addresses.range_mut(..start_address).next_back() {
            // Check if region is a direct neighbour
            if *left_region.0 + *left_region.1 == start_address {
                // Increase the region's size
                *left_region.1 += size;

                // Sync the other tree
                let removed_region = self.free_regions
                    .remove_entry(&SizeKey{ size: *left_region.1 - size, index: *left_region.0})
                    .expect("vmm: fatal mismatch between vmemory trees when freeing page");
                self.free_regions.insert(SizeKey{ size: removed_region.0.size + size, index: removed_region.0.index }, removed_region.1);
            }
        }
        else if let Some(removed_region) = self.free_addresses.remove_entry(&(start_address + size)) {
            self.free_addresses.insert(start_address, removed_region.1 + size);

            // Sync the other tree
            self.free_regions.remove_entry(&SizeKey{size: removed_region.1, index: removed_region. 0});
            self.free_regions.insert(SizeKey{size: removed_region.1 + size, index: start_address}, start_address);
        }

        // If both neighbouring regions are allocated, just reinsert the region
        else {
            self.free_addresses.insert(start_address, size);
            self.free_regions.insert(SizeKey{size, index: start_address}, start_address);
        }
    }
}
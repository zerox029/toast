use alloc::collections::{BTreeMap};
use crate::memory::{PAGE_SIZE, VirtualAddress};

pub mod paging;
pub mod heap_allocator;

pub const KERNEL_ALLOCATION_SPACE_START: VirtualAddress = 0xFFFFC90000000000;
pub const KERNEL_ALLOCATION_SPACE_END: VirtualAddress = 0xFFFFFFFEFFFFFFFF;
pub const KERNEL_ALLOCATION_SPACE_SIZE: VirtualAddress = KERNEL_ALLOCATION_SPACE_END - KERNEL_ALLOCATION_SPACE_START;

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
    allocated_amount: usize,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        let free_addresses = BTreeMap::from([(KERNEL_ALLOCATION_SPACE_START, KERNEL_ALLOCATION_SPACE_SIZE)]);

        let free_region_key = SizeKey {
            size: KERNEL_ALLOCATION_SPACE_SIZE,
            index: KERNEL_ALLOCATION_SPACE_START,
        };
        let free_regions = BTreeMap::from([(free_region_key, KERNEL_ALLOCATION_SPACE_START)]);

        Self {
            free_addresses,
            free_regions,
            allocated_amount: 0,
        }
    }

    pub fn get_allocated_amount(&self) -> usize {
        self.allocated_amount
    }

    pub fn display_memory(&self) {
        println!("free regions: {:X?}", self.free_regions);
        println!("free addresses: {:X?}", self.free_addresses);
    }

    /// Allocates a single page in the kernel allocation space region
    pub fn allocate_page(&mut self) -> Result<VirtualAddress, &'static str> {
        self.allocate_pages(1)
    }

    pub fn allocate_pages(&mut self, count: usize) -> Result<VirtualAddress, &'static str> {
        let required_size = count * PAGE_SIZE;

        // Find the first region that is big enough to accommodate the allocation request
        let region_key = match self.free_regions.range(SizeKey{size: required_size, index: 0}..).next() {
            Some((k, _)) => *k,
            None => return Err("vmm: not enough memory")
        };
        let region = self.free_regions.remove_entry(&region_key);

        if let Some(region) = region {
            // Shrink the region if it can fit the requested size
            if region.0.size > required_size {
                let new_size = region.0.size - required_size;
                let new_start_address = region.1 + required_size;

                self.free_regions.insert(SizeKey{size: new_size, index: new_start_address}, new_start_address);
            }

            // Sync the other tree
            let removed_address = self.free_addresses.remove_entry(&region.1);

            if let Some(removed_address) = removed_address {
                if removed_address.1 != region.0.size {
                    panic!("vmm: fatal mismatch between vmemory trees when allocating memory");
                }

                // Shrink the region if it can fit the requested size
                if removed_address.1 > required_size {
                    let new_size = region.0.size - required_size;
                    let new_start_address = region.1 + required_size;

                    self.free_addresses.insert(new_start_address, new_size);
                }

                self.allocated_amount += required_size;
                return Ok(removed_address.0)
            }

            panic!("vmm: fatal mismatch between vmemory trees when allocating memory");
        }

        Err("vmm: could not allocate requested memory")
    }

    pub fn deallocate_page(&mut self, address: VirtualAddress) -> Result<(), &'static str> {
        self.deallocate_pages(address, PAGE_SIZE)
    }

    pub fn deallocate_pages(&mut self, start_address: VirtualAddress, count: usize) -> Result<(), &'static str> {
        let required_size = count * PAGE_SIZE;

        // If neighbouring left region is unallocated, merge it with the one currently being freed
        if let Some(left_region) = self.free_addresses.range_mut(..start_address).next_back() {
            // Check if region is a direct neighbour
            if *left_region.0 + *left_region.1 == start_address {
                // Increase the region's size
                *left_region.1 += required_size;

                // Sync the other tree
                let removed_region = self.free_regions
                    .remove_entry(&SizeKey{ size: *left_region.1 - required_size, index: *left_region.0});

                return match removed_region {
                    None => Err("vmm: fatal mismatch between vmemory trees when freeing page"),
                    Some(removed_region) => {
                        self.free_regions.insert(SizeKey { size: removed_region.0.size + required_size, index: removed_region.0.index }, removed_region.1);
                        self.allocated_amount -= required_size;
                        Ok(())
                    }
                }
            }
        }

        return if let Some(removed_region) = self.free_addresses.remove_entry(&(start_address + count)) {
            self.free_addresses.insert(start_address, removed_region.1 + count);

            // Sync the other tree
            self.free_regions.remove_entry(&SizeKey { size: removed_region.1, index: removed_region.0 });
            self.free_regions.insert(SizeKey { size: removed_region.1 + count, index: start_address }, start_address);

            self.allocated_amount -= required_size;

            Ok(())
        }
        else { // If both neighbouring regions are allocated, just reinsert the region
            self.free_addresses.insert(start_address, count);
            self.free_regions.insert(SizeKey { size: count, index: start_address }, start_address);

            self.allocated_amount -= required_size;

            Ok(())
        }

        // TODO: Return Err if requested memory is already free
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use crate::memory::{PAGE_SIZE, VirtualAddress};
    use crate::memory::virtual_memory::{KERNEL_ALLOCATION_SPACE_SIZE, KERNEL_ALLOCATION_SPACE_START, SizeKey, VirtualMemoryManager};

    #[test_case]
    fn allocate_page_happy_path() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        let starting_region_size = vmm.free_addresses.values().next().expect("VMM was not initialized properly");
        let expected_region_size = starting_region_size - PAGE_SIZE;

        // WHEN
        let alloc = vmm.allocate_page();

        // THEN
        assert!(alloc.is_ok()); // Allocated memory correctly
        assert_eq!(*vmm.free_addresses.values().next().expect("no free regions left"), expected_region_size); // Free addresses tree was updated
        assert_vmm_trees_are_equivalent(&vmm.free_addresses, &vmm.free_regions);
    }

    #[test_case]
    fn allocate_multiple_pages_happy_path() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        let starting_region_size = vmm.free_addresses.values().next().expect("VMM was not initialized properly");
        let expected_region_size = starting_region_size - 5*PAGE_SIZE;

        // WHEN
        let alloc = vmm.allocate_pages(5);

        // THEN
        assert!(alloc.is_ok()); // Allocated memory correctly
        assert_eq!(*vmm.free_addresses.values().next().expect("no free regions left"), expected_region_size); // Free addresses tree was updated
        assert_vmm_trees_are_equivalent(&vmm.free_addresses, &vmm.free_regions);
    }

    #[test_case]
    fn allocate_page_out_of_memory() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        vmm.free_addresses = BTreeMap::new();
        vmm.free_regions = BTreeMap::new();

        // WHEN
        let alloc = vmm.allocate_page();

        // THEN
        assert!(alloc.is_err());
        assert_eq!(alloc.unwrap_err(), "vmm: not enough memory")
    }

    #[test_case]
    fn deallocation_no_merge() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        let expected_addresses_tree = BTreeMap::from([
            (KERNEL_ALLOCATION_SPACE_START, PAGE_SIZE),
            (KERNEL_ALLOCATION_SPACE_START + 2 * PAGE_SIZE, KERNEL_ALLOCATION_SPACE_SIZE - 2 * PAGE_SIZE)
        ]);
        let expected_regions_tree = BTreeMap::from([
            (SizeKey{size: PAGE_SIZE, index: KERNEL_ALLOCATION_SPACE_START}, KERNEL_ALLOCATION_SPACE_START),
            (SizeKey{size: KERNEL_ALLOCATION_SPACE_SIZE - 2 * PAGE_SIZE, index: KERNEL_ALLOCATION_SPACE_START + 2 * PAGE_SIZE}, KERNEL_ALLOCATION_SPACE_START + 2 * PAGE_SIZE)
        ]);

        // WHEN
        let alloc1 = vmm.allocate_page();
        let alloc2 = vmm.allocate_page();
        let dealloc = vmm.deallocate_page(alloc1.unwrap());

        // THEN
        assert!(dealloc.is_ok());
        assert_vmm_trees_are_equivalent(&vmm.free_addresses, &vmm.free_regions);
        assert_address_trees_are_equal(&expected_addresses_tree, &vmm.free_addresses);
        assert_region_trees_are_equal(&expected_regions_tree, &vmm.free_regions);
    }

    #[test_case]
    fn deallocation_merge_right() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        let expected_region_tree = vmm.free_regions.clone();
        let expected_addresses_tree = vmm.free_addresses.clone();

        // WHEN
        let alloc = vmm.allocate_page();
        let dealloc = vmm.deallocate_page(alloc.unwrap());

        // THEN
        assert!(dealloc.is_ok()); // Freed memory correctly
        assert_vmm_trees_are_equivalent(&vmm.free_addresses, &vmm.free_regions);
        assert_region_trees_are_equal(&expected_region_tree, &vmm.free_regions);
        assert_address_trees_are_equal(&expected_addresses_tree, &vmm.free_addresses);
    }

    #[test_case]
    fn deallocation_merge_left() {
        // GIVEN
        let mut vmm = VirtualMemoryManager::new();
        let expected_addresses_tree = BTreeMap::from([
            (KERNEL_ALLOCATION_SPACE_START, 2 * PAGE_SIZE),
            (KERNEL_ALLOCATION_SPACE_START + 3 * PAGE_SIZE, KERNEL_ALLOCATION_SPACE_SIZE - 3 * PAGE_SIZE)
        ]);
        let expected_regions_tree = BTreeMap::from([
            (SizeKey{size: 2 * PAGE_SIZE, index: KERNEL_ALLOCATION_SPACE_START}, KERNEL_ALLOCATION_SPACE_START),
            (SizeKey{size: KERNEL_ALLOCATION_SPACE_SIZE - 3 * PAGE_SIZE, index: KERNEL_ALLOCATION_SPACE_START + 3 * PAGE_SIZE}, KERNEL_ALLOCATION_SPACE_START + 3 * PAGE_SIZE)
        ]);

        // WHEN
        let alloc1 = vmm.allocate_page();
        let alloc2 = vmm.allocate_page();
        let alloc3 = vmm.allocate_page();

        let dealloc1 = vmm.deallocate_page(alloc1.unwrap()); // No merge
        let dealloc2 = vmm.deallocate_page(alloc2.unwrap()); // Merge left with alloc1

        // THEN
        assert!(dealloc1.is_ok());
        assert!(dealloc2.is_ok());
        assert_vmm_trees_are_equivalent(&vmm.free_addresses, &vmm.free_regions);
        assert_address_trees_are_equal(&expected_addresses_tree, &vmm.free_addresses);
        assert_region_trees_are_equal(&expected_regions_tree, &vmm.free_regions);
    }

    fn assert_vmm_trees_are_equivalent(free_addresses: &BTreeMap<VirtualAddress, usize>, free_regions: &BTreeMap<SizeKey, VirtualAddress>) {
        assert_eq!(free_regions.len(), free_addresses.len());
        for address in free_addresses {
            let corresponding_region = free_regions.get(&SizeKey{size: *address.1, index: *address.0});

            assert!(corresponding_region.is_some());
            assert_eq!(corresponding_region.unwrap(), address.0);
        }
    }

    fn assert_region_trees_are_equal(expected: &BTreeMap<SizeKey, VirtualAddress>, actual: &BTreeMap<SizeKey, VirtualAddress>) {
        assert_eq!(expected.len(), actual.len());
        for first_region in expected {
            let corresponding_region = actual.get(first_region.0);

            assert!(corresponding_region.is_some());
            assert_eq!(corresponding_region.unwrap(), first_region.1);
        }
    }

    fn assert_address_trees_are_equal(expected: &BTreeMap<VirtualAddress, usize>, actual: &BTreeMap<VirtualAddress, usize>) {
        assert_eq!(expected.len(), actual.len());
        for first_address in expected {
            let corresponding_region = actual.get(first_address.0);

            assert!(corresponding_region.is_some());
            assert_eq!(corresponding_region.unwrap(), first_address.1);
        }
    }
}
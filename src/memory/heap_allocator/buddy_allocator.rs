use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use crate::memory::paging::Page;

pub struct BuddyAllocator {
    available_pages: Vec<Page>
}
impl BuddyAllocator {

}
unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() > 4096 {
            panic!("mm: allocator only support allocations under 4KiB");
        }

        todo!()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, layout: Layout) {
        if layout.size() > 4096 {
            panic!("mm: allocator only support deallocations under 4KiB");
        }

        todo!()
    }
}

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

struct HeapPage {
    buddy_maps: [BuddyMap; 10],
}

impl HeapPage {
    fn new() -> Self {
        Self {
            buddy_maps: [
                BuddyMap::new(4096),
                BuddyMap::new(2048),
                BuddyMap::new(1024),
                BuddyMap::new(512),
                BuddyMap::new(256),
                BuddyMap::new(128),
                BuddyMap::new(64),
                BuddyMap::new(32),
                BuddyMap::new(16),
                BuddyMap::new(8),
            ],
        }
    }
}

struct BuddyMap {
    /// What size chunks should this map keep track of
    allocation_size: usize,
    /// 512 bits bitmap
    map: [u128; 4],
}

impl BuddyMap {
    fn new(allocation_size: usize) -> Self {
        BuddyMap {
            allocation_size,
            map: [0; 4],
        }
    }
}
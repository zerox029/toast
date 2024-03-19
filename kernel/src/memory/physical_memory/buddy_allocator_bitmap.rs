use alloc::collections::LinkedList;
use core::ptr;
use limine::memory_map;
use rlibc::memset;
use crate::memory::{PAGE_SIZE, PhysicalAddress};

/// Maximum allocation size, this allocator cannot allocate blocks larger than 2^MAX_ORDER pages
const MAX_ORDER: usize = 10;

pub struct MemoryRegion {
    /// The physical address of the start of the region
    start_address: PhysicalAddress,
    /// The size in bytes of the region
    size: usize,

    /// The total size in bytes of all the buddy bitmaps representing the region.
    bitmap_size: usize,
    /// The total number of frames contained in the region
    frame_count: usize,
    /// An array of pointers to the start of each buddy bitmap
    memory_maps: [Option<*mut u8>; MAX_ORDER + 1],

    /// A pointer to the next memory region
    next_region: Option<&'static MemoryRegion>,
}
impl MemoryRegion {
    pub fn new(start_address: PhysicalAddress, size: usize, memory_maps_start: *mut u8) -> MemoryRegion {
        let frame_count = size.div_ceil(PAGE_SIZE);

        let bitmap_size = (0..=MAX_ORDER).map(|order| Self::order_bitmap_size(frame_count, order)).sum();

        let memory_maps = {
            let mut memory_maps: [Option<*mut u8>; MAX_ORDER + 1] = [None; 11];

            for i in 0..=MAX_ORDER {
                let bitmap_size = Self::order_bitmap_size(frame_count, i);
                if bitmap_size == 0 {
                    break;
                }

                // Find address
                let bitmap_address = unsafe { Self::order_bitmap_address(memory_maps_start, frame_count, i) };

                let address = bitmap_address;
                memory_maps[i] = Some(address);

                // Set all bits to 0
                #[cfg(not(test))]
                unsafe { ptr::write_bytes(address, 0, Self::order_bitmap_size(frame_count, i)); }
            }

            memory_maps
        };

        Self {
            start_address,
            size,
            bitmap_size,
            frame_count,
            memory_maps,
            next_region: None,
        }
    }

    fn order_bitmap_size(frame_count: usize, order: usize) -> usize {
        frame_count.div_ceil(8).div_floor(2usize.pow(order as u32))
    }

    unsafe fn order_bitmap_address(base_address: *mut u8, frame_count: usize, order: usize) -> *mut u8 {
        let offset = (0..order).map(|order| Self::order_bitmap_size(frame_count, order)).sum();

        base_address.add(offset)
    }
}

pub struct BuddyAllocator {
    current_region: *mut MemoryRegion,
    buffer_start: *mut u8,
}

#[cfg(test)]
mod tests {
    use crate::memory::physical_memory::buddy_allocator_bitmap::MemoryRegion;

    #[test_case]
    fn order_bitmap_size_order_zero() {
        // Region fits exactly
        assert_eq!(MemoryRegion::order_bitmap_size(8, 0), 1);
        assert_eq!(MemoryRegion::order_bitmap_size(472, 0), 59);

        // Region falls between byte boundaries
        assert_eq!(MemoryRegion::order_bitmap_size(10, 0), 2);
        assert_eq!(MemoryRegion::order_bitmap_size(212, 0), 27);

        // Region smaller than 1 byte
        assert_eq!(MemoryRegion::order_bitmap_size(4, 0), 1);
        assert_eq!(MemoryRegion::order_bitmap_size(7, 0), 1);
    }

    #[test_case]
    fn order_bitmap_size_order_nonzero() {
        // Region fits exactly
        assert_eq!(MemoryRegion::order_bitmap_size(32, 1), 2);
        assert_eq!(MemoryRegion::order_bitmap_size(2048, 3), 32);

        // Region fits no higher order buddies
        assert_eq!(MemoryRegion::order_bitmap_size(8, 1), 0);

        // Region doesn't fit in higher order buddies
        assert_eq!(MemoryRegion::order_bitmap_size(24, 1), 1);
    }

    #[test_case]
    fn new_memory_region_full() {
        let region = MemoryRegion::new(0, 0x2000000, 0 as *mut u8);

        /// 0x2000000 bytes => 0x2000 frames
        /// Bitmap size: 0x400, 0x200, 0x100, 0x80, 0x40, 0x20, 0x10, 0x8, 0x4, 0x2, 0x1
        /// Total bitmaps size: 0x3FF

        let expected_memory_maps = [
            Some(0 as *mut u8),
            Some(0x400 as *mut u8),
            Some(0x600 as *mut u8),
            Some(0x700 as *mut u8),
            Some(0x780 as *mut u8),
            Some(0x7C0 as *mut u8),
            Some(0x7E0 as *mut u8),
            Some(0x7F0 as *mut u8),
            Some(0x7F8 as *mut u8),
            Some(0x7FC as *mut u8),
            Some(0x7FE as *mut u8),
        ];

        assert_eq!(region.start_address, 0);
        assert_eq!(region.size, 0x2000000);
        assert_eq!(region.bitmap_size, 0x7FF);
        assert_eq!(region.frame_count, 0x2000);
        assert_eq!(region.memory_maps, expected_memory_maps);
        assert!(region.next_region.is_none());
    }

    #[test_case]
    fn new_memory_region_not_full() {
        let region = MemoryRegion::new(0, 0x400000, 0 as *mut u8);

        /// 0x400000 bytes => 0x400 frames
        /// Bitmap size: 0x80, 0x40, 0x20, 0x10, 0x8, 0x4, 0x2, 0x1
        /// Total bitmaps size: 0x3FF

        let expected_memory_maps = [
            Some(0 as *mut u8),
            Some(0x80 as *mut u8),
            Some(0xC0 as *mut u8),
            Some(0xE0 as *mut u8),
            Some(0xF0 as *mut u8),
            Some(0xF8 as *mut u8),
            Some(0xFC as *mut u8),
            Some(0xFE as *mut u8),
            None,
            None,
            None,
        ];

        assert_eq!(region.start_address, 0);
        assert_eq!(region.size, 0x400000);
        assert_eq!(region.bitmap_size, 0xFF);
        assert_eq!(region.frame_count, 0x400);
        assert_eq!(region.memory_maps, expected_memory_maps);

        assert!(region.next_region.is_none());
    }
}
use core::ptr;
use crate::memory::{PAGE_SIZE, PhysicalAddress};

/// Maximum allocation size, this allocator cannot allocate blocks larger than 2^MAX_ORDER pages
const MAX_ORDER: usize = 10;

pub struct MemoryRegion {
    /// The physical address of the start of the region
    region_start_address: PhysicalAddress,
    /// The size in bytes of the region
    region_size: usize,
    /// The total number of frames contained in the region
    region_frame_count: usize,

    /// The total size in bytes of all the buddy bitmaps representing the region.
    bitmap_size: usize,
    /// The physical address of the start of the bitmap, this is largely equivalent to `memory_maps.as_ptr()`
    bitmap_start: *mut u8,
    /// An array of pointers to the start of each buddy bitmap
    bitmaps: [Option<*mut u8>; MAX_ORDER + 1],

    /// A pointer to the next memory region
    next_region: Option<&'static MemoryRegion>,
}
impl MemoryRegion {
    pub fn new(start_address: PhysicalAddress, size: usize, memory_maps_start: *mut u8) -> MemoryRegion {
        let frame_count = size.div_ceil(PAGE_SIZE);

        let bitmap_size = (0..=MAX_ORDER).map(|order| Self::order_bitmap_size(frame_count, order)).sum();

        let bitmaps = {
            let mut bitmaps: [Option<*mut u8>; MAX_ORDER + 1] = [None; 11];

            for i in 0..=MAX_ORDER {
                let bitmap_size = Self::order_bitmap_size(frame_count, i);
                if bitmap_size == 0 {
                    break;
                }

                // Find address
                let bitmap_address = unsafe { Self::order_bitmap_address(memory_maps_start, frame_count, i) };

                let address = bitmap_address;
                bitmaps[i] = Some(address);

                // Set all bits to 0
                unsafe { ptr::write_bytes(address, 0, Self::order_bitmap_size(frame_count, i)); }
            }

            bitmaps
        };

        Self {
            region_start_address: start_address,
            region_size: size,
            bitmap_start: memory_maps_start,
            bitmap_size,
            region_frame_count: frame_count,
            bitmaps,
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
    use limine::memory_map::EntryType;
    use crate::memory::physical_memory::buddy_allocator_bitmap::MemoryRegion;
    use crate::memory::PhysicalAddress;
    use crate::{HHDM_OFFSET, MEMORY_MAP_REQUEST};

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
        // GIVEN
        /// 0x2000000 bytes => 0x2000 frames
        /// Bitmap size: 0x400, 0x200, 0x100, 0x80, 0x40, 0x20, 0x10, 0x8, 0x4, 0x2, 0x1
        /// Total bitmaps size: 0x3FF
        let region_start: PhysicalAddress = *HHDM_OFFSET;
        let region_size = 0x2000000;

        let expected_bitmap_size = 0x7FFusize;
        let expected_frame_count = 0x2000;

        let bitmap_start_address = first_free_region(expected_bitmap_size as u64);

        let expected_memory_maps = unsafe { [
            Some(bitmap_start_address),
            Some(bitmap_start_address.add(0x400)),
            Some(bitmap_start_address.add(0x600)),
            Some(bitmap_start_address.add(0x700)),
            Some(bitmap_start_address.add(0x780)),
            Some(bitmap_start_address.add(0x7C0)),
            Some(bitmap_start_address.add(0x7E0)),
            Some(bitmap_start_address.add(0x7F0)),
            Some(bitmap_start_address.add(0x7F8)),
            Some(bitmap_start_address.add(0x7FC)),
            Some(bitmap_start_address.add(0x7FE)),
        ] };

        // WHEN
        let region = MemoryRegion::new(region_start, region_size, bitmap_start_address);

        // THEN
        assert_eq!(region.region_start_address, region_start);
        assert_eq!(region.region_size, region_size);
        assert_eq!(region.bitmap_size, expected_bitmap_size);
        assert_eq!(region.region_frame_count, expected_frame_count);
        assert_eq!(region.bitmaps, expected_memory_maps);
        assert!(region.next_region.is_none());

        assert!((0..expected_bitmap_size).all(|i| unsafe { *region.bitmap_start.add(i) } == 0));
    }

    #[test_case]
    fn new_memory_region_not_full() {
        // GIVEN
        /// 0x400000 bytes => 0x400 frames
        /// Bitmap size: 0x80, 0x40, 0x20, 0x10, 0x8, 0x4, 0x2, 0x1
        /// Total bitmaps size: 0x3FF
        let region_start: PhysicalAddress = *HHDM_OFFSET;
        let region_size = 0x400000;

        let expected_bitmap_size = 0xFFusize;
        let expected_frame_count = 0x400;

        let bitmap_start_address = first_free_region(expected_bitmap_size as u64);

        let expected_memory_maps = unsafe { [
            Some(bitmap_start_address),
            Some(bitmap_start_address.add(0x80)),
            Some(bitmap_start_address.add(0xC0)),
            Some(bitmap_start_address.add(0xE0)),
            Some(bitmap_start_address.add(0xF0)),
            Some(bitmap_start_address.add(0xF8)),
            Some(bitmap_start_address.add(0xFC)),
            Some(bitmap_start_address.add(0xFE)),
            None,
            None,
            None,
        ] };

        // WHEN
        let region = MemoryRegion::new(region_start, region_size, bitmap_start_address);

        // THEN
        assert_eq!(region.region_start_address, region_start);
        assert_eq!(region.region_size, region_size);
        assert_eq!(region.bitmap_size, expected_bitmap_size);
        assert_eq!(region.region_frame_count, expected_frame_count);
        assert_eq!(region.bitmaps, expected_memory_maps);
        assert!(region.next_region.is_none());

        assert!((0..expected_bitmap_size).all(|i| unsafe { *region.bitmap_start.add(i) } == 0));
    }

    fn first_free_region(expected_bitmap_size: u64) -> *mut u8 {
        unsafe { (MEMORY_MAP_REQUEST
                .get_response()
                .expect("could not retrieve the memory map")
                .entries()
                .iter()
                .find(|region| region.entry_type == EntryType::USABLE && region.length >= expected_bitmap_size)
                .expect("could not find a free region")
                .base as *mut u8)
                .add(*HHDM_OFFSET) }
    }
}
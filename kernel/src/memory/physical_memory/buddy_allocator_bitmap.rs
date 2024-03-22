use core::mem::size_of;
use core::ops::Index;
use core::ptr;
use core::slice::from_raw_parts;
use limine::memory_map;
use limine::memory_map::EntryType;
use linked_list_allocator::align_up;
use crate::{HHDM_OFFSET, set_bit, test_bit};
use crate::memory::{PAGE_SIZE, PhysicalAddress, VirtualAddress};
use crate::memory::physical_memory::{Frame, FrameAllocator};

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
                let bitmap_address = Self::order_bitmap_address(memory_maps_start, frame_count, i);

                let address = bitmap_address;
                bitmaps[i] = Some(address);

                // Set all bits to 0
                #[cfg(not(test))]
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

    fn order_bitmap_address(base_address: *mut u8, frame_count: usize, order: usize) -> *mut u8 {
        let offset = (0..order).map(|order| Self::order_bitmap_size(frame_count, order)).sum();

        unsafe { base_address.add(offset) }
    }

    /// Finds the first free block found in the bitmap, flips it and returns its address
    fn allocate_first_free_block(&self, order: usize) -> Option<VirtualAddress> {
        return match self.bitmaps[order] {
            None => None,
            Some(bitmap) => {
                let bitmap_size = Self::order_bitmap_size(self.region_frame_count, order);
                unsafe {
                    let mut bit_offset = 0; // The offset of the bit within the region bitmap
                    for (byte_index, byte) in from_raw_parts(bitmap, bitmap_size).iter().enumerate() {
                        let first_free_block_index = (!(*byte).reverse_bits()).trailing_zeros();

                        // Check if a free block was found in the current byte
                        if first_free_block_index < 8 {
                            set_bit!(byte, first_free_block_index);
                            bit_offset = byte_index * 8 + first_free_block_index as usize;
                            break;
                        }
                    }

                    // TODO: Return None if no free block was found

                    let block_address = self.region_start_address + bit_offset * 2usize.pow(order as u32) * PAGE_SIZE;

                    Some(block_address)
                }
            }
        }
    }

    fn allocate_block_at(&self, order: usize, address: PhysicalAddress) -> Option<VirtualAddress> {
        if address < self.region_start_address || address >= self.region_start_address + self.region_size {
            return None;
        }

        return match self.bitmaps[order] {
            None => None,
            Some(bitmap) => {
                let block_size = 2usize.pow(order as u32);

                let bit_index = address / block_size;
                let byte_index = bit_index / 8;
                let bit_offset = bit_index % 8;

                let containing_byte = unsafe { bitmap.add(byte_index) };
                if !test_bit!(containing_byte, bit_offset) {
                    set_bit!(containing_byte, bit_offset);
                    return Some(address + *HHDM_OFFSET);
                }

                None
            }
        }
    }
}

struct MemoryRegionIter {
    current: &'static MemoryRegion,
}

impl Iterator for MemoryRegionIter {
    type Item = &'static MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        return if let Some(next_region) = self.current.next_region {
            let region = self.current;
            self.current = next_region;


            Some(region)
        } else {
            None
        }
    }
}

pub struct BuddyAllocator {
    first_region: &'static MemoryRegion,
    allocated_amount: usize,
}

impl BuddyAllocator {
    pub unsafe fn init(memory_regions: &[&memory_map::Entry]) -> Result<Self, &'static str> {
        let mut buffer_size = 0usize;
        for entry in memory_regions.iter().filter(|entry| entry.entry_type == EntryType::USABLE) {
            let frame_count = entry.length.div_ceil(PAGE_SIZE as u64);
            buffer_size += size_of::<MemoryRegion>() * 2 + frame_count.div_ceil(8) as usize;
        }

        // Find an available region large enough to accommodate everything
        let containing_entry = memory_regions
            .iter()
            .find(|entry| entry.entry_type == EntryType::USABLE && entry.length >= buffer_size as u64)
            .ok_or("pmm: could not find a suitable memory region to hold the pmm")?;
        let buffer_start = align_up((containing_entry.base as usize + *HHDM_OFFSET), PAGE_SIZE);
        //let bitmap_size = align_down(containing_entry.base as usize, PAGE_SIZE);

        let mut current_buffer_start = buffer_start;
        let mut previous_region: Option<&mut MemoryRegion> = None;
        for entry in memory_regions.iter().filter(|entry| entry.entry_type == EntryType::USABLE) {
            info!("pmm: [0x{:X} -> 0x{:X}] - length 0x{:X} bytes", entry.base, entry.base + entry.length, entry.length);

            let bitmap_start_address = current_buffer_start + size_of::<MemoryRegion>();
            let region = MemoryRegion::new(entry.base as PhysicalAddress, entry.length as usize, bitmap_start_address as *mut u8);
            ptr::write(current_buffer_start as *mut MemoryRegion, region);

            let current_region = current_buffer_start as *mut MemoryRegion;

            if let Some(previous_region) = previous_region {
                previous_region.next_region = Some(unsafe { &*current_region });
            }
            previous_region = Some(unsafe { &mut * current_region });

            current_buffer_start += size_of::<MemoryRegion>() + unsafe { &mut * current_region }.bitmap_size;
        }

        Ok(Self {
            first_region: unsafe { &*(buffer_start as *const MemoryRegion) },
            allocated_amount: 0
        })
    }

    pub fn to_iter(&self) -> MemoryRegionIter {
        MemoryRegionIter {
            current: self.first_region
        }
    }

    /// Allocates the first free block of the requested order found in the bitmaps linked list
    /// and returns its address wrapped in an option.
    fn allocate_first_free_block(&self, order: usize) -> Option<VirtualAddress>  {
        self.to_iter().find_map(|region| region.allocate_first_free_block(order))
    }

    /// Allocates a free block of the given size at the requested address. Fails if no free block was found
    fn allocate_block_at(&self, order: usize, address: PhysicalAddress) -> Result<VirtualAddress, &'static str> {
        self.to_iter().find_map(|region| region.allocate_block_at(order, address)).ok_or("pmm: could not allocate memory")
    }
}

impl FrameAllocator for BuddyAllocator {
    fn allocate_frame(&mut self) -> Result<Frame, &'static str> {
        let frame_address = self.allocate_frames(0)?;
        let frame = Frame::containing_address(frame_address);

        Ok(frame)
    }

    fn deallocate_frame(&mut self, frame: Frame) -> Result<(), &'static str> {
        // self.deallocate_frame(frame.start_address(), 0)
        Err("bruh")
    }
}


#[cfg(test)]
mod tests {
    use core::ptr;
    use limine::memory_map::EntryType;
    use crate::memory::physical_memory::buddy_allocator_bitmap::{BuddyAllocator, MemoryRegion};
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
        serial_println!("{:?}", bitmap_start_address);

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
    }

    #[test_case]
    fn find_first_free_block_empty_bitmap() {
        // GIVEN
        let region_start: PhysicalAddress = 0;
        let region_size = 0x2000000;

        let expected_bitmap_size = 0x7FFusize;
        let bitmap_start_address = first_free_region(expected_bitmap_size as u64);

        let region = MemoryRegion::new(region_start, region_size, bitmap_start_address);

        // THEN
        assert!(region.find_first_free_block(0).is_some());
        assert_eq!(region.find_first_free_block(0).unwrap(), 0);
    }

    #[test_case]
    fn find_first_free_block_middle_bit() {
        // GIVEN
        let region_start: PhysicalAddress = 0;
        let region_size = 0x1000;

        let expected_bitmap_size = 1;
        let bitmap_start_address = first_free_region(expected_bitmap_size as u64);

        let region = MemoryRegion::new(region_start, region_size, bitmap_start_address);
        unsafe { ptr::write(region.bitmap_start, 0b11101111) };

        // WHEN
        let first_free_block = region.find_first_free_block(0);

        //THEN
        assert!(first_free_block.is_some());
        assert_eq!(first_free_block.unwrap(), 3);
    }


    #[test_case]
    fn buddy_allocator_init() {
        let buddy_allocator = unsafe {
            BuddyAllocator::init(MEMORY_MAP_REQUEST
                .get_response()
                .expect("could not retrieve the memory map")
                .entries())
                .unwrap()
        };

        let mut current_region = Some(buddy_allocator.first_region);

        buddy_allocator.to_iter().for_each(|region| {
            current_region = region.next_region;
        })
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
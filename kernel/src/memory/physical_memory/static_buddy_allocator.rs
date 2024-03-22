use core::intrinsics::size_of;
use core::ptr;
use limine::memory_map;
use limine::memory_map::EntryType;
use linked_list_allocator::align_up;
use crate::HHDM_OFFSET;
use crate::memory::{PAGE_SIZE, PhysicalAddress};
use crate::utils::bitmap_btree::BitmapBinaryTree;

const MAX_ORDER: usize = 10;

struct MemoryRegion {
    /// The physical address of the start of the region
    region_start_address: PhysicalAddress,
    /// The size in bytes of the region
    region_size: usize,
    /// The total number of frames contained in the region
    region_frame_count: usize,

    /// A binary tree bitmap representing all memory blocks available in this region
    memory_blocks: BitmapBinaryTree,
    /// A reference to the next memory region
    next_region: Option<&'static MemoryRegion>,
}
impl MemoryRegion {
    pub fn new(start_address: PhysicalAddress, size: usize, memory_maps_start: *mut u8) -> Self {
        let frame_count = size.div_ceil(PAGE_SIZE);
        let memory_blocks = BitmapBinaryTree::new(memory_maps_start, frame_count);

        Self {
            region_start_address: start_address,
            region_size: size,
            region_frame_count: frame_count,
            memory_blocks,
            next_region: None
        }
    }
}

pub struct BuddyAllocator {
    first_region: &'static MemoryRegion,
    allocated_amount: usize,
}
impl BuddyAllocator {
    pub unsafe fn init(memory_regions: &[&memory_map::Entry]) -> Result<Self, &'static str> {
        // Calculate how much memory will be necessary to accommodate the allocator
        let mut buffer_size = memory_regions
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .fold(0, |acc, entry|
                acc + size_of::<MemoryRegion>() + entry.length.div_ceil(PAGE_SIZE as u64).div_ceil(8) as usize);

        // Find an available region large enough to fit everything
        let containing_entry = memory_regions
            .iter()
            .find(|entry| entry.entry_type == EntryType::USABLE && entry.length >= buffer_size as u64)
            .ok_or("pmm: could not find a suitable memory region to hold the pmm")?;
        let buffer_start = align_up((containing_entry.base as usize + *HHDM_OFFSET), PAGE_SIZE);

        let mut current_buffer_start = buffer_start;
        let mut previous_region: Option<&mut MemoryRegion> = None;
        for entry in memory_regions.iter().filter(|entry| entry.entry_type == EntryType::USABLE) {
            let bitmap_start_address = current_buffer_start + size_of::<MemoryRegion>();
            let region = MemoryRegion::new(entry.base as PhysicalAddress, entry.length as usize, bitmap_start_address as *mut u8);
            ptr::write(current_buffer_start as *mut MemoryRegion, region);

            let current_region = current_buffer_start as *mut MemoryRegion;

            if let Some(previous_region) = previous_region {
                previous_region.next_region = Some(&*current_region);
            }
            previous_region = Some(&mut *current_region);

            current_buffer_start += size_of::<MemoryRegion>() + (&mut *current_region).memory_blocks.get_full_size()
        }

        Ok(Self {
            first_region: &*(buffer_start as *const MemoryRegion),
            allocated_amount: 0,
        })
    }
}
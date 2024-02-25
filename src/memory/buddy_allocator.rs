use alloc::collections::LinkedList;
use alloc::vec::Vec;
use crate::arch::multiboot2::structures::{MemoryMapEntry, MemoryMapIter};
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};
use crate::serial_println;

const MAX_ORDER: usize = 10;

pub struct BuddyAllocator {
    free_areas: [LinkedList<MemoryBlock>; MAX_ORDER + 1],

    kernel_start: usize,
    kernel_end: usize,
    multiboot_start: usize,
    multiboot_end: usize,

    current_area: Option<&'static MemoryMapEntry>,
    areas: MemoryMapIter,
}

#[derive(Copy, Clone)]
struct MemoryBlock {
    is_allocated: bool,
    starting_address: usize,
    size_class: usize
}

impl MemoryBlock {
    fn contains_address(&self, address: usize) -> bool {
        address >= self.starting_address && address < self.starting_address + PAGE_SIZE * 2usize.pow(self.size_class as u32)
    }
}

impl BuddyAllocator {
    pub fn new(kernel_start: usize, kernel_end: usize,
               multiboot_start: usize, multiboot_end: usize,
               memory_map: MemoryMapIter) -> Self {
        let mut max_order_list = LinkedList::new();
        max_order_list.push_back(MemoryBlock {
            is_allocated: false,
            starting_address: memory_map.clone().next().unwrap().base_addr as usize,
            size_class: MAX_ORDER
        });

        Self {
            free_areas: [
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                LinkedList::new(),
                max_order_list,
            ],
            kernel_start,
            kernel_end,
            multiboot_start,
            multiboot_end,
            current_area: memory_map.clone().next(),
            areas: memory_map
        }
    }

    pub fn set_allocated_frames(&mut self, frames: Vec<usize>) {
        for frame_number in frames {
            self.allocate_frame_at_address(frame_number * PAGE_SIZE);
        }
    }

    /// Allocates 2^order contiguous frames
    pub fn allocate_frames(&mut self, order: usize) -> Option<usize> {
        if order > MAX_ORDER {
            panic!("Cannot allocate more than {} contiguous frames", MAX_ORDER);
        }

        let first_free_block = self.free_areas[order].iter_mut().find(|block| block.is_allocated == false);
        return if first_free_block.is_some() {
            let block = first_free_block.unwrap();
            serial_println!("found unused {} order", block.size_class);
            block.is_allocated = true;
            Some(block.starting_address)
        } else {
            if order == MAX_ORDER {
                return None;
            }

            self.split_any_block(order + 1)
        }
    }

    /// Allocates a single frame at a given address. This is moslty used when transitioning from
    /// the linear allocator to this one.
    pub fn allocate_frame_at_address(&mut self, address: usize) -> Option<usize> {
        if self.free_areas[0].iter().find(|block| block.is_allocated && block.starting_address == address).is_some() {
            panic!("frame already allocated");
        }

        // 1. Find the biggest free block containing the address
        let mut current_block: Option<&mut MemoryBlock> = None;
        let mut current_order = 0;
        while current_block.is_none() && current_order <= MAX_ORDER {
            current_block = self.free_areas[current_order].iter_mut().find(|block| block.contains_address(address));
            current_order += 1;
        }

        let mut current_block = current_block.expect("could not allocate memory");
        current_block.is_allocated = true;

        let mut current_block_clone = current_block.clone();

        while current_block_clone.size_class > 0 {
            let buddy_size_class = current_block_clone.size_class - 1;

            let mut left_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address,
                size_class: buddy_size_class,
            };

            let mut right_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address + PAGE_SIZE * 2usize.pow(buddy_size_class as u32),
                size_class: buddy_size_class,
            };

            current_block_clone.is_allocated = true;

            if left_buddy.contains_address(address) {
                left_buddy.is_allocated = true;
                current_block_clone = left_buddy;

                self.free_areas[buddy_size_class].push_back(left_buddy);
                self.free_areas[buddy_size_class].push_back(right_buddy);
            }
            else {
                right_buddy.is_allocated = true;
                current_block_clone = right_buddy;

                self.free_areas[buddy_size_class].push_back(left_buddy);
                self.free_areas[buddy_size_class].push_back(right_buddy);
            }
        }

        Some(current_block_clone.starting_address)
    }

    /// Split a 2^n sized block into two 2^n-1 sized blocks, and sets the first one as allocated and returns it.
    /// The created blocks are added to the free_areas array at index n-1 and the original block is marked as allocated.
    fn split_any_block(&mut self, order: usize) -> Option<usize> {
        if order == 0 {
            panic!("cannot split block further");
        }

        // Find the first, smallest unallocated block that fits
        let mut current_block: Option<&mut MemoryBlock> = None;
        let mut current_order = order;
        while current_block.is_none() && order <= MAX_ORDER {
            current_block = self.free_areas[current_order].iter_mut().find(|block| !block.is_allocated);
            current_order += 1;
        }

        let mut current_block = current_block.expect("could not allocate memory");
        current_block.is_allocated = true;

        let mut current_block_clone = current_block.clone();

        // Repeatedly split until we get to the desired size
        while current_block_clone.size_class > order {
            let buddy_size_class = current_block_clone.size_class - 1;

            let left_buddy = MemoryBlock {
                is_allocated: true,
                starting_address: current_block_clone.starting_address,
                size_class: buddy_size_class,
            };

            let right_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address + PAGE_SIZE * 2usize.pow(buddy_size_class as u32),
                size_class: buddy_size_class,
            };

            // Mark the original block as allocated
            current_block_clone.is_allocated = true;

            // Add the two buddies to the linked list
            self.free_areas[buddy_size_class].push_back(left_buddy);
            self.free_areas[buddy_size_class].push_back(right_buddy);

            // Return only the (allocated) left buddy
            current_block_clone = left_buddy
        }

        Some(current_block_clone.starting_address)
    }

    fn merge_blocks() {
        todo!();
    }
}

impl FrameAllocator for BuddyAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let frame_address = self.allocate_frames(0).expect("could not allocate frame");
        let frame = Frame::containing_address(frame_address);

        Some(frame)
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        todo!()
    }
}
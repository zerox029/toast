use alloc::collections::LinkedList;
use alloc::vec::Vec;
use core::cmp::min;
use crate::arch::multiboot2::structures::{MemoryMapEntry, MemoryMapIter};
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};
use crate::{println, print, serial_println};
use crate::memory::buddy_allocator::BlockType::{LeftBuddy, RightBuddy, TopLevel};

const MAX_ORDER: usize = 10;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BlockType {
    TopLevel,
    LeftBuddy,
    RightBuddy
}

type MemoryBlocks = [LinkedList<MemoryBlock>; MAX_ORDER + 1];
pub struct BuddyAllocator {
    memory_blocks: MemoryBlocks,
}

#[derive(Debug, Copy, Clone)]
struct MemoryBlock {
    is_allocated: bool,
    starting_address: usize,
    size_class: usize,
    block_type: BlockType
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
        let mut memory_blocks: MemoryBlocks = [
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
            LinkedList::new(),
        ];

        // Fill the memory block lists
        for area in memory_map {
            Self::map_area(area, &mut memory_blocks, kernel_start, kernel_end, multiboot_start, multiboot_end);
        }

        Self {
            memory_blocks,
        }
    }

    /// Used when transitioning from the linear allocator, this makes sure all frames allocated
    /// by the previous allocator are marked as such in this one
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

        let first_free_block = self.memory_blocks[order].iter_mut().find(|block| block.is_allocated == false);
        return if first_free_block.is_some() {
            let block = first_free_block.unwrap();
            block.is_allocated = true;

            Some(block.starting_address)
        } else {
            self.split_block(order + 1)
        }
    }

    /// Deallocates 2^order contiguous frames
    pub fn deallocate_frames(&mut self, start_address: usize, order: usize) {
        let memory_block = self.memory_blocks[order].iter_mut()
            .find(|block| block.starting_address == start_address);

        if memory_block.is_none() {
            panic!("could not find the frame to deallocate");
        }

        if let Some(memory_block) = memory_block {
            if !memory_block.is_allocated {
                panic!("frame was already unallocated");
            }

            memory_block.is_allocated = false;

            // Merge only if block is a buddy
            if memory_block.block_type == TopLevel {
                return;
            }

            let buddy_address = if memory_block.block_type == LeftBuddy {
                memory_block.starting_address + PAGE_SIZE * 2usize.pow(memory_block.size_class as u32)
            } else {
                memory_block.starting_address - PAGE_SIZE * 2usize.pow(memory_block.size_class as u32)
            };

            let buddy = self.memory_blocks[order].iter_mut()
                .find(|block| block.starting_address == buddy_address);

            if buddy.is_none() {
                panic!("could not find the frame to deallocate");
            }

            // Merge the two blocks
            if let Some(buddy) = buddy {
                if !buddy.is_allocated {
                    let parent_block_address = min(start_address, buddy_address);

                    let _extracted_buddy = self.memory_blocks[order]
                        .extract_if(|block| block.starting_address == start_address);
                    let _extracted_buddy = self.memory_blocks[order]
                        .extract_if(|block| block.starting_address == buddy_address);

                    self.memory_blocks[order + 1]
                        .iter_mut()
                        .find(|block| block.starting_address == parent_block_address)
                        .expect("could not find a parent block")
                        .is_allocated = false;
                }
            }
        }
    }

    /// Allocates a single frame at a given address. This is mostly used when transitioning from
    /// the linear allocator to this one.
    pub fn allocate_frame_at_address(&mut self, address: usize) -> Option<usize> {
        if self.memory_blocks[0].iter().find(|block| block.is_allocated && block.starting_address == address).is_some() {
            panic!("frame already allocated");
        }

        // 1. Find the biggest free block containing the address
        let mut current_block: Option<&mut MemoryBlock> = None;
        let mut current_order = 0;
        while current_block.is_none() && current_order <= MAX_ORDER {
            current_block = self.memory_blocks[current_order].iter_mut().find(|block| block.contains_address(address));
            current_order += 1;
        }

        let current_block = current_block.expect("could not allocate memory");
        current_block.is_allocated = true;

        let mut current_block_clone = current_block.clone();

        while current_block_clone.size_class > 0 {
            let buddy_size_class = current_block_clone.size_class - 1;

            let mut left_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address,
                size_class: buddy_size_class,
                block_type: LeftBuddy
            };

            let mut right_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address + PAGE_SIZE * 2usize.pow(buddy_size_class as u32),
                size_class: buddy_size_class,
                block_type: RightBuddy,
            };

            if left_buddy.contains_address(address) {
                left_buddy.is_allocated = true;
                current_block_clone = left_buddy;

                self.memory_blocks[buddy_size_class].push_back(left_buddy);
                self.memory_blocks[buddy_size_class].push_back(right_buddy);
            }
            else {
                right_buddy.is_allocated = true;
                current_block_clone = right_buddy;

                self.memory_blocks[buddy_size_class].push_back(left_buddy);
                self.memory_blocks[buddy_size_class].push_back(right_buddy);
            }
        }

        Some(current_block_clone.starting_address)
    }

    fn map_area(area: &MemoryMapEntry, memory_blocks: &mut MemoryBlocks,
                kernel_start: usize, kernel_end: usize, multiboot_start: usize, multiboot_end: usize,) {
        let mut start_address = area.base_addr;
        let mut end_address = start_address as usize + PAGE_SIZE * 2usize.pow(MAX_ORDER as u32);

        while start_address < area.base_addr + area.size {
            let mut current_order = MAX_ORDER as u32;

            // If block starts in restricted area, move to the end of that area
            if start_address as usize >= kernel_start && start_address as usize <= kernel_end {
                // Offset so that allocation is page aligned
                let offset = PAGE_SIZE - kernel_end % PAGE_SIZE;

                start_address = (kernel_end + offset) as u64;
                end_address = start_address as usize + PAGE_SIZE * 2usize.pow(MAX_ORDER as u32);

                continue;
            }
            else if start_address as usize >= multiboot_start && start_address as usize <= multiboot_end {
                // Offset so that allocation is page aligned
                let offset = PAGE_SIZE - multiboot_end % PAGE_SIZE;

                start_address = (multiboot_end + offset) as u64;
                end_address = start_address as usize + PAGE_SIZE * 2usize.pow(MAX_ORDER as u32);

                continue;
            }


            // Find the largest block that fits
            while end_address > (area.base_addr + area.size) as usize
                || Self::block_is_in_forbidden_area(start_address as usize, end_address, kernel_start, kernel_end, multiboot_start, multiboot_end) {
                // If no block order fits, no more blocks can be added for this area
                if current_order == 0 {
                    return;
                }

                current_order -= 1;
                end_address = start_address as usize + PAGE_SIZE * 2usize.pow(current_order);
            }

            // Add the block to its corresponding list
            memory_blocks[current_order as usize].push_back(MemoryBlock {
                is_allocated: false,
                starting_address: start_address as usize,
                size_class: current_order as usize,
                block_type: TopLevel
            });

            // Move on to the next block
            start_address += (PAGE_SIZE * 2usize.pow(current_order)) as u64;
            end_address = start_address as usize + PAGE_SIZE * 2usize.pow(MAX_ORDER as u32);
        }
    }

    /// Split a 2^order sized block into two 2^order-1 sized blocks, and sets the first one as allocated and returns it.
    /// The created blocks are added to the free_areas array at index order-1 and the original block is marked as allocated.
    fn split_block(&mut self, order: usize) -> Option<usize> {
        if order == 0 {
            panic!("cannot split block further");
        }

        // Find the first, smallest unallocated block that fits
        let mut first_free_block: Option<&mut MemoryBlock> = None;
        let mut current_order = order;
        while first_free_block.is_none() && current_order <= MAX_ORDER {
            first_free_block = self.memory_blocks[current_order].iter_mut().find(|block| !block.is_allocated);
            current_order += 1;
        }

        let current_block = first_free_block.expect("could not allocate memory");
        current_block.is_allocated = true;

        let mut current_block_clone = current_block.clone();

        // Repeatedly split until we get to the desired size
        while current_block_clone.size_class >= order {
            let buddy_size_class = current_block_clone.size_class - 1;

            let left_buddy = MemoryBlock {
                is_allocated: true,
                starting_address: current_block_clone.starting_address,
                size_class: buddy_size_class,
                block_type: LeftBuddy
            };

            let right_buddy = MemoryBlock {
                is_allocated: false,
                starting_address: current_block_clone.starting_address + PAGE_SIZE * 2usize.pow(buddy_size_class as u32),
                size_class: buddy_size_class,
                block_type: RightBuddy
            };

            // Add the two buddies to the linked list
            self.memory_blocks[buddy_size_class].push_back(left_buddy);
            self.memory_blocks[buddy_size_class].push_back(right_buddy);

            // Return only the (allocated) left buddy
            current_block_clone = left_buddy
        }

        Some(current_block_clone.starting_address)
    }

    fn block_is_in_forbidden_area(start: usize, end: usize, kernel_start: usize, kernel_end: usize, multiboot_start: usize, multiboot_end: usize) -> bool {
        Self::block_start_is_in_forbidden_area(start, kernel_start, kernel_end, multiboot_start, multiboot_end)
        || Self::block_end_is_in_forbidden_area(end, kernel_start, kernel_end, multiboot_start, multiboot_end)
    }

    fn block_start_is_in_forbidden_area(start: usize, kernel_start: usize, kernel_end: usize, multiboot_start: usize, multiboot_end: usize) -> bool {
        (start >= kernel_start && start <= kernel_end) || (start >= multiboot_start && start <= multiboot_end)
    }

    fn block_end_is_in_forbidden_area(end: usize, kernel_start: usize, kernel_end: usize, multiboot_start: usize, multiboot_end: usize) -> bool {
        (end >= kernel_start && end <= kernel_end) || (end >= multiboot_start && end <= multiboot_end)
    }
}

impl FrameAllocator for BuddyAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let frame_address = self.allocate_frames(0).expect("could not allocate frame");
        let frame = Frame::containing_address(frame_address);

        Some(frame)
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        self.deallocate_frames(frame.start_address(), 0);
    }
}
use core::ops::DerefMut;
use crate::arch::multiboot2::BootInformation;
use crate::memory::linear_frame_allocator::PageFrameAllocator;
use crate::memory::paging::{ActivePageTable, PhysicalAddress};
use crate::{print, info, serial_println};
use crate::memory::buddy_allocator::BuddyAllocator;

use self::paging::remap_kernel;
use self::heap_allocator::{init_heap};

pub mod linear_frame_allocator;
pub mod paging;
pub mod heap_allocator;
pub mod buddy_allocator;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize
}

impl Frame {
    /// Returns the frame containing the address passed as argument
    pub fn containing_address(address: usize) -> Frame {
        Frame{ number: address / PAGE_SIZE }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    pub fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start,
            end
        }
    }

    fn clone(&self) -> Frame {
        Frame { number: self.number }
    }
}

pub struct FrameIter {
    start: Frame,
    end: Frame
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        }
        else {
            None
        }
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

pub struct MemoryManagementUnit {
    pub frame_allocator: PageFrameAllocator,
    pub active_page_table: ActivePageTable,
}

impl MemoryManagementUnit {
    pub fn new(boot_information: &BootInformation) -> Self {
        info!("mm: init...");

        let memory_map = boot_information.memory_map().expect("Memory map tag required");
        let elf_symbols = boot_information.elf_symbols().expect("Elf symbols tag required");

        let kernel_start = elf_symbols.section_headers().map(|s| s.start_address()).min().unwrap();
        let kernel_end = elf_symbols.section_headers().map(|s| s.end_address()).max().unwrap();

        let multiboot_start = boot_information.start_address();
        let multiboot_end = multiboot_start + (boot_information.total_size as usize);

        let mut frame_allocator = PageFrameAllocator::new(kernel_start, kernel_end,
                                                          multiboot_start, multiboot_end,
                                                          memory_map.entries());

        let mut active_page_table = remap_kernel(&mut frame_allocator, boot_information);
        init_heap(active_page_table.deref_mut(), &mut frame_allocator);



        let mut buddy_allocator = BuddyAllocator::new(kernel_start, kernel_end,
                                                  multiboot_start, multiboot_end,
                                                  memory_map.entries());

        let allocation = buddy_allocator.allocate_frame();
        serial_println!("Allocated at {}", allocation.unwrap().start_address());
        let allocation2 = buddy_allocator.allocate_frame();
        serial_println!("Allocated at {}", allocation2.unwrap().start_address());

        Self {
            frame_allocator,
            active_page_table,
        }
    }
}
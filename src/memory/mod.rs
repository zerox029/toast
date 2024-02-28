use core::ops::DerefMut;
use crate::arch::multiboot2::BootInformation;
use crate::memory::linear_frame_allocator::LinearFrameAllocator;
use crate::memory::paging::{ActivePageTable, Page, PhysicalAddress};
use crate::{print, info, serial_println};
use crate::memory::buddy_allocator::BuddyAllocator;
use crate::memory::paging::entry::EntryFlags;

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

pub struct MemoryManager {
    pub frame_allocator: BuddyAllocator,
    pub active_page_table: ActivePageTable,
}

impl MemoryManager {
    pub fn new(boot_information: &BootInformation) -> Self {
        info!("mm: init...");

        let memory_map = boot_information.memory_map().expect("Memory map tag required");
        let elf_symbols = boot_information.elf_symbols().expect("Elf symbols tag required");

        let kernel_start = elf_symbols.section_headers().map(|s| s.start_address()).min().unwrap();
        let kernel_end = elf_symbols.section_headers().map(|s| s.end_address()).max().unwrap();

        let multiboot_start = boot_information.start_address();
        let multiboot_end = multiboot_start + (boot_information.total_size as usize);

        let mut linear_allocator = LinearFrameAllocator::new(kernel_start, kernel_end,
                                                             multiboot_start, multiboot_end,
                                                             memory_map.entries());

        let mut active_page_table = remap_kernel(&mut linear_allocator, boot_information);
        init_heap(active_page_table.deref_mut(), &mut linear_allocator);

        // Switch to the buddy allocator
        let mut buddy_allocator = BuddyAllocator::new(kernel_start, kernel_end,
                                                  multiboot_start, multiboot_end,
                                                  memory_map.entries());


        buddy_allocator.set_allocated_frames(linear_allocator.allocated_frames());

        Self {
            frame_allocator: buddy_allocator,
            active_page_table,
        }
    }

    pub fn vmm_alloc() {
        unimplemented!();
    }

    pub fn vmm_zero_alloc() {
        unimplemented!();
    }

    pub fn vmm_free() {
        unimplemented!();
    }

    /// Allocates enough physically contiguous identity mapped pages to cover the requested size
    pub fn pmm_alloc(&mut self, size: usize, flags: EntryFlags) -> Option<usize> {
        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        let alloc_start = self.frame_allocator.allocate_frames(order);

        if let Some(alloc_start) = alloc_start {
            let alloc_size = 2usize.pow(order as u32);

            // Identity map the pages
            for page_number in 0..alloc_size {
                let page_address = alloc_start + PAGE_SIZE * page_number;
                let frame = Frame::containing_address(page_address);

                self.active_page_table.deref_mut().identity_map(frame, flags, &mut self.frame_allocator);
            }
        }

        alloc_start
    }

    pub fn pmm_zero_alloc() {
        unimplemented!();
    }

    pub fn pmm_free(&mut self, size: usize, address: usize) {
        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        self.frame_allocator.deallocate_frames(address, order);

        let freed_size = 2usize.pow(order as u32);

        // Unmap the pages
        for page_number in 0..freed_size {
            let page_address = address + PAGE_SIZE * page_number;
            let page = Page::containing_address(page_address);

            self.active_page_table.deref_mut().unmap_no_dealloc(&page);
        }
    }
}
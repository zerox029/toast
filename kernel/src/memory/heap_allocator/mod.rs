mod bump_allocator;
mod slab_allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::memory::{FrameAllocator};
use crate::memory::heap_allocator::slab_allocator::SlabAllocator;
use crate::memory::paging::mapper::Mapper;
use crate::memory::paging::{ActivePageTable, Page, VirtualAddress};
use crate::memory::paging::entry::EntryFlags;
use crate::{vga_print, ok, serial_println, HHDM_OFFSET};

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 1000 * 1024; // 1 MiB

#[global_allocator]
static ALLOCATOR: Locked<SlabAllocator> = Locked::new(SlabAllocator::new());

pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn init_heap<A>(page_table: &mut ActivePageTable, frame_allocator: &mut A) where A: FrameAllocator {
    serial_println!("mm: initializing the heap...");

    let page_range = {
        let heap_start: VirtualAddress = HEAP_START + *HHDM_OFFSET;
        let heap_end: VirtualAddress = heap_start + HEAP_SIZE - 1usize;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    serial_println!("Heap from {:X} to {:X}", HEAP_START, HEAP_START + HEAP_SIZE);

    for page in page_range {
        let frame = frame_allocator.allocate_frame().expect("Frame allocation failed");

        let flags = EntryFlags::PRESENT | EntryFlags::WRITABLE;

        page_table.map_to(page, frame, flags, frame_allocator)
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START + *HHDM_OFFSET, HEAP_SIZE);
    }

    serial_println!("mm: heap starts at 0x{:X}", HEAP_START + *HHDM_OFFSET);
}

// TODO: Setup custom test framework
pub fn test_heap() {
    // Simple allocation
    {
        let heap_value_1 = Box::new(41);
        let heap_value_2 = Box::new(13);

        assert_eq!(*heap_value_1, 41);
        assert_eq!(*heap_value_2, 13);
    }

    // Large vec
    {
        let n =1000;
        let mut vec = Vec::new();
        for i in 0..n {
            vec.push(i);
        }
        assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
    }

    // Many boxes
    {
        for i in 0..HEAP_SIZE {
            let x = Box::new(i);
            assert_eq!(*x, i);
        }
    }
}
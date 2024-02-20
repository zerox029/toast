mod bump_allocator;
mod fixed_size_block_allocator;
mod buddy_allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::memory::{FrameAllocator};
use crate::memory::heap_allocator::fixed_size_block_allocator::FixedSizeBlockAllocator;
use crate::memory::paging::mapper::Mapper;
use crate::memory::paging::{Page, VirtualAddress};
use crate::memory::paging::entry::EntryFlags;
use crate::{println, print};
use crate::memory::heap_allocator::bump_allocator::BumpAllocator;

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

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

pub fn init_heap<A>(mapper: &mut Mapper, frame_allocator: &mut A) where A: FrameAllocator {
    let page_range = {
        let heap_start: VirtualAddress = HEAP_START;
        let heap_end: VirtualAddress = heap_start + HEAP_SIZE - 1usize;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator.allocate_frame().expect("Frame allocation failed");
        let flags = EntryFlags::PRESENT | EntryFlags::WRITABLE;

        mapper.map_to(page, frame, flags, frame_allocator)
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_START);
    }

    println!("mm: heap starts at 0x{:X}", HEAP_START);
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
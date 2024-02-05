use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use linked_list_allocator::LockedHeap;
use crate::arch::multiboot2::BootInformation;
use crate::memory::FrameAllocator;
use crate::memory::paging::mapper::Mapper;
use crate::memory::paging::{Page, VirtualAddress};
use crate::memory::paging::entry::EntryFlags;

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub struct Dummy;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should never be called");
    }
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

        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_START);
    }
}

// TODO: Setup custom test framework
pub fn test_heap() {
    // Simple allocation
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);

    // Large vec
    let n =1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);

    // Many boxes
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}
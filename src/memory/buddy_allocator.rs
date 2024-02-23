use alloc::collections::LinkedList;
use crate::memory::{Frame, FrameAllocator};
use crate::memory::paging::Page;

const MAX_ORDER: usize = 10;

struct FreeArea {
    list: LinkedList<Page>,
    map: u64,
}

pub struct BuddyAllocator {
    free_areas: [FreeArea; MAX_ORDER],
}

impl FrameAllocator for BuddyAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        todo!()
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        todo!()
    }
}
use crate::memory::paging::PhysicalAddress;

pub use self::paging::test_paging;
pub use self::paging::remap_kernel;

pub mod page_frame_allocator;
mod paging;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize
}

impl Frame {
    /// Returns the frame containing the address passed as argument
    fn containing_address(address: usize) -> Frame {
        Frame{ number: address / PAGE_SIZE }
    }

    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start,
            end
        }
    }

    fn clone(&self) -> Frame {
        Frame { number: self.number }
    }
}

struct FrameIter {
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
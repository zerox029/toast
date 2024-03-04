use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use limine::memory_map::{Entry, EntryType};
use limine::response::MemoryMapResponse;
use crate::arch::multiboot2::structures::{MemoryMapEntry, MemoryMapIter};
use crate::memory::{Frame, FrameAllocator};

/// The amount of simultaneous frames that can be allocated with this allocator. A hard limit is needed because
/// this allocator is used before the heap is initialized
const ALLOCATION_LIMIT: usize = 300;

#[derive(Copy, Clone)]
pub struct FrameStatus {
    frame_id: Option<usize>,
    used: bool,
}

impl FrameStatus {
    fn default() -> Self {
        Self {
            frame_id: None,
            used: false,
        }
    }
}

/// Allocates frames linearly. This allocator is incredibly inefficient and should only be used before the heap is available
/// in order to track allocated and free frames.
pub struct LinearFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static Entry>,
    memory_map: &'static MemoryMapResponse,

    allocated_frames: [FrameStatus; ALLOCATION_LIMIT],
    allocated_frames_count: usize,
}

impl FrameAllocator for LinearFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        // Look for a previously allocated frame that has been freed
        for frame_number in 0..self.allocated_frames_count {
            if self.allocated_frames[frame_number].used == false {
                return Some(Frame { number: self.allocated_frames[frame_number].frame_id.unwrap() });
            }
        }

        if let Some(area) = self.current_area {
            let frame = Frame{ number: self.next_free_frame.number };

            let current_area_last_frame = {
                let address = area.base + area.length - 1;
                Frame::containing_address(address as usize)
            };

            // Move to the next area if all frames in the current area are used
            if frame > current_area_last_frame {
                self.choose_next_area();

                self.allocate_frame()
            }
            // Allocate the frame if it is unused
            else {
                self.next_free_frame.number += 1;

                self.allocated_frames[self.allocated_frames_count] = FrameStatus { frame_id: Some(frame.number), used: true };
                self.allocated_frames_count += 1;

                return Some(frame)
            }
        }
        else {
            None
        }
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        for frame_number in 0..self.allocated_frames_count {
            if self.allocated_frames[frame_number].frame_id.unwrap() == frame.number {
                self.allocated_frames[frame_number].used = false;
            }
        }
    }
}

impl LinearFrameAllocator {
    pub fn new(memory_map: &'static MemoryMapResponse) -> LinearFrameAllocator {
        let mut allocator = LinearFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            memory_map,

            allocated_frames: [FrameStatus::default(); ALLOCATION_LIMIT],
            allocated_frames_count: 0,
        };

        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self.memory_map.entries().iter().filter(|area| {
            area.entry_type == EntryType::USABLE && {
                let end_address = area.base + area.length - 1;
                Frame::containing_address(end_address as usize) >= self.next_free_frame
            }
        }).min_by_key(|area| area.base).map(|&area| area);

        // Set the new next free frame
        if let Some(area) = self.current_area {
            let area_start_frame = Frame::containing_address(area.base as usize);
            if self.next_free_frame < area_start_frame {
                self.next_free_frame = area_start_frame;
            }
        }
    }

    pub fn allocated_frames(&self) -> Vec<usize> {
        self.allocated_frames
            .iter()
            .filter(|frame| frame.used && frame.frame_id.is_some())
            .map(|frame| frame.frame_id.unwrap())
            .collect()
    }
}
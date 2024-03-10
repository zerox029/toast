use alloc::vec::Vec;
use limine::memory_map::{Entry, EntryType};
use limine::response::MemoryMapResponse;
use crate::memory::{Frame, PhysicalAddress};
use crate::memory::physical_memory::FrameAllocator;

/// The amount of simultaneous frames that can be allocated with this allocator. A hard limit is needed because
/// this allocator is used before the heap is initialized
const ALLOCATION_LIMIT: usize = 300;

#[derive(Copy, Clone)]
pub struct FrameStatus {
    frame_address: Option<PhysicalAddress>,
    used: bool,
}

impl FrameStatus {
    fn default() -> Self {
        Self {
            frame_address: None,
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
    fn allocate_frame(&mut self) -> Result<Frame, &'static str> {
        // Look for a previously allocated frame that has been freed
        for frame_number in 0..self.allocated_frames_count {
            if !self.allocated_frames[frame_number].used {
                return Ok(Frame { number: self.allocated_frames[frame_number].frame_address.unwrap() });
            }
        }

        if let Some(area) = self.current_area {
            let frame = Frame{ number: self.next_free_frame.number };

            let current_area_last_frame = {
                let address = (area.base + area.length - 1) as PhysicalAddress;
                Frame::containing_address(address)
            };

            // Move to the next area if all frames in the current area are used
            if frame > current_area_last_frame {
                self.choose_next_area();

                self.allocate_frame()
            }
            // Allocate the frame if it is unused
            else {
                self.next_free_frame.number += 1;

                self.allocated_frames[self.allocated_frames_count] = FrameStatus { frame_address: Some(frame.start_address()), used: true };
                self.allocated_frames_count += 1;

                Ok(frame)
            }
        }
        else {
            Err("linear frame allocator could not allocate requested frame")
        }
    }

    fn deallocate_frame(&mut self, frame: Frame) -> Result<(), &'static str> {
        for frame_number in 0..self.allocated_frames_count {
            if self.allocated_frames[frame_number].frame_address.unwrap() == frame.number {
                if self.allocated_frames[frame_number].used == true {
                    self.allocated_frames[frame_number].used = false;
                }
                else {
                    return Err("cannot deallocate a non-allocated frame");
                }
            }
            else {
                return Err("error deallocating frame");
            }
        }

        Ok(())
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
                let end_address = (area.base + area.length - 1) as PhysicalAddress;
                Frame::containing_address(end_address) >= self.next_free_frame
            }
        }).min_by_key(|area| area.base).copied();

        // Set the new next free frame
        if let Some(area) = self.current_area {
            let area_start_frame = Frame::containing_address(area.base as PhysicalAddress);
            if self.next_free_frame < area_start_frame {
                self.next_free_frame = area_start_frame;
            }
        }
    }

    pub fn allocated_frames(&self) -> Vec<PhysicalAddress> {
        self.allocated_frames
            .iter()
            .filter(|frame| frame.used && frame.frame_address.is_some())
            .map(|frame| frame.frame_address.unwrap())
            .collect()
    }
}
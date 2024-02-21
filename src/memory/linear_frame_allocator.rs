use crate::arch::multiboot2::structures::{MemoryMapEntry, MemoryMapIter};
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};

/// The amount of simultaneous frames that can be allocated with this allocator. A hard limit is needed because
/// this allocator is used before the heap is initialized
const ALLOCATION_LIMIT: usize = 100;

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
pub struct PageFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryMapEntry>,
    areas: MemoryMapIter,

    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,

    allocated_frames: [FrameStatus; ALLOCATION_LIMIT],
    allocated_frames_count: usize,
}

impl FrameAllocator for PageFrameAllocator {
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
                let address = area.base_addr + area.size - 1;
                Frame::containing_address(address as usize)
            };

            // Move to the next area if all frames in the current area are used
            if frame > current_area_last_frame {
                self.choose_next_area();
            }
            // Move outside the kernel region if the frame is part of it
            else if frame >= self.kernel_start && frame <= self.kernel_end {
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1
                };
            }
            // Move outside the multiboot region is the frame is part of it
            else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1
                };
            }
            // Return the frame if it is unused
            else {
                self.next_free_frame.number += 1;

                self.allocated_frames[self.allocated_frames_count] = FrameStatus { frame_id: Some(frame.number), used: true};
                self.allocated_frames_count += 1;

                return Some(frame)
            }

            self.allocate_frame()
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

impl PageFrameAllocator {
    pub fn new(kernel_start: usize, kernel_end: usize,
               multiboot_start: usize, multiboot_end: usize,
               memory_map: MemoryMapIter) -> PageFrameAllocator {
        let mut allocator = PageFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_map,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),

            allocated_frames: [FrameStatus::default(); ALLOCATION_LIMIT],
            allocated_frames_count: 0,
        };

        let mut page_count = 0;
        for area in allocator.areas {
            page_count += area.size as usize / PAGE_SIZE;
        }

        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self.areas.clone().filter(|area| {
            // Filter only the areas that still have free frames
            let address = area.base_addr + area.size - 1;
            Frame::containing_address(address as usize) >= self.next_free_frame
        }).min_by_key(|area| area.base_addr);

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}
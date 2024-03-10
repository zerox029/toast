use crate::memory::{Frame, VirtualAddress};
use crate::memory::physical_memory::FrameAllocator;
use crate::memory::virtual_memory::paging::entry::EntryFlags;
use crate::memory::virtual_memory::paging::table::{Level1, Table};
use super::{ActivePageTable, Page};

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
        where A: FrameAllocator
    {
        TemporaryPage {
            page,
            allocator: TinyAllocator::new(allocator),
        }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable)
               -> VirtualAddress
    {
        assert!(active_table.translate_page(self.page).is_none(),
                "temporary page is already mapped");
        active_table.map_to(self.page, frame, EntryFlags::WRITABLE, &mut self.allocator);
        self.page.start_address()
    }

    /// Unmaps the temporary page in the active table.
    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page, &mut self.allocator)
    }

    /// Maps the temporary page to the given page table frame in the active
    /// table. Returns a reference to the now mapped table.
    pub fn map_table_frame(&mut self,
                           frame: Frame,
                           active_table: &mut ActivePageTable)
                           -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }
}

struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    fn new<A>(allocator: &mut A) -> TinyAllocator
        where A: FrameAllocator
    {
        let mut f = || allocator.allocate_frame();
        let frames = [
            Some(f().expect("could not allocate frame")),
            Some(f().expect("could not allocate frame")),
            Some(f().expect("could not allocate frame"))
        ];
        TinyAllocator(frames)
    }
}

impl FrameAllocator for TinyAllocator {
    fn allocate_frame(&mut self) -> Result<Frame, &'static str> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return Ok(frame_option.take().unwrap());
            }
        }

        Err("could not allocate frame")
    }

    fn deallocate_frame(&mut self, frame: Frame) -> Result<(), &'static str> {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return Ok(());
            }
        }

        Err("Tiny allocator can hold only 3 frames.")
    }
}
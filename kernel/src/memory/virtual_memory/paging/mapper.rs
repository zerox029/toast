use core::arch::asm;
use core::ptr::Unique;
use crate::memory::{Frame, PAGE_SIZE, PhysicalAddress, VirtualAddress};
use crate::memory::virtual_memory::paging::table::{Level4, Table};
use crate::memory::virtual_memory::paging::{ENTRY_COUNT, Page};
use crate::memory::virtual_memory::paging::entry::EntryFlags;
use crate::HHDM_OFFSET;
use crate::arch::x86_64::registers::cr3;
use crate::memory::physical_memory::FrameAllocator;

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    pub unsafe fn new() -> Mapper {
        let p4_table: *mut Table<Level4> = (cr3() + *HHDM_OFFSET) as *mut _;

        Mapper {
            p4: Unique::new_unchecked(p4_table),
        }
    }

    pub unsafe fn new_at(address: VirtualAddress) -> Mapper {
        let p4_table: *mut Table<Level4> = address as *mut _;

        Mapper {
            p4: Unique::new_unchecked(p4_table),
        }
    }

    pub fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    /// Translates a virtual_memory address to the corresponding physical_memory address.
    /// Returns `None` if the address is not mapped.
    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address)).map(|frame| frame.number * PAGE_SIZE + offset)
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert_eq!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT), 0);
                        return Some(Frame {
                            number: start_frame.number + page.p2_index() *
                                ENTRY_COUNT + page.p1_index(),
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert_eq!(start_frame.number % ENTRY_COUNT, 0);
                            return Some(Frame {
                                number: start_frame.number + page.p1_index()
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    /// Maps the page to the frame with the provided flags.
    /// The `PRESENT` flag is added by default. Needs a
    /// `FrameAllocator` as it might need to create new page tables.
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
        let p4 = self.p4_mut();
        let p3 = p4.next_table_create(page.p4_index(), allocator);
        let p2 = p3.next_table_create(page.p3_index(), allocator);
        let p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
    }

    /// Maps the page to some free frame with the provided flags.
    /// The free frame is allocated from the given `FrameAllocator`.
    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
        let frame = allocator.allocate_frame().expect("out of memory");
        self.map_to(page, frame, flags, allocator);
    }

    /// Identity map the given frame with the provided flags such that its virtual_memory address corresponds
    /// to its physical_memory address. The `FrameAllocator` is used to create new page tables if needed.
    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator);
    }

    /// Same method as above but does not crash if the page was already mapped
    pub fn identity_map_if_unmapped<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A) where A: FrameAllocator {
        let page = Page::containing_address(frame.start_address());
        if self.check_is_unmapped(page, allocator) {
            self.map_to(page, frame, flags, allocator);
        }
    }

    /// Unmaps the given page and adds all freed frames to the given
    /// `FrameAllocator`.
    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A) where A: FrameAllocator {
        let frame = self.unmap_no_dealloc(&page).unwrap();

        // TODO free p(1,2,3) table if empty
        allocator.deallocate_frame(frame).expect("could not deallocate");
    }

    pub fn unmap_no_dealloc(&mut self, page: &Page) -> Option<Frame> {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");

        let frame = p1[page.p1_index()].pointed_frame();
        p1[page.p1_index()].set_unused();

        unsafe {
            asm!("invlpg [{}]", in(reg) page.start_address());
        }

        frame
    }

    fn check_is_unmapped<A>(&mut self, page: Page, allocator: &mut A) -> bool where A: FrameAllocator {
        let p4 = self.p4_mut();
        let p3 = p4.next_table_create(page.p4_index(), allocator);
        let p2 = p3.next_table_create(page.p3_index(), allocator);
        let p1 = p2.next_table_create(page.p2_index(), allocator);

        p1[page.p1_index()].is_unused()
    }
}
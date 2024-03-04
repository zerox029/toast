use core::arch::asm;
use core::ops::{Deref, DerefMut};
use limine::memory_map::EntryType;
use limine::response::MemoryMapResponse;
use crate::arch::multiboot2::BootInformation;
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};
use crate::memory::paging::entry::EntryFlags;
use crate::memory::paging::temporary_page::TemporaryPage;
use crate::memory::paging::mapper::Mapper;
use crate::{vga_print, info, ok, serial_println, HHDM_OFFSET, KERNEL_START_VMA_ADDRESS};
use crate::arch::x86_64::registers::cr3;
use crate::vga_buffer::VGA_BUFFER_ADDRESS;

pub mod entry;
pub mod table;
pub mod temporary_page;
pub mod mapper;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    /// Returns the page containing a virtual address
    pub fn containing_address(address: VirtualAddress) -> Page {
        // Checking that the sign extension bits correspond to the 47th bit
        assert!(!(0x0000_8000_0000_0000..0xffff_8000_0000_0000).contains(&address), "Invalid address: 0x{:x}", address);

        Page { number: address / PAGE_SIZE }
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start,
            end
        }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        self.number & 0o777
    }
}

pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        }
        else {
            None
        }
    }
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {

    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    pub unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub unsafe fn new_at(address: usize) -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new_at(address),
        }
    }

    pub fn with<F>(&mut self, inactive_table: &mut InactivePageTable, temporary_page: &mut TemporaryPage, f: F)
            where F: FnOnce(&mut Mapper) {
        {
            use x86_64::instructions::tlb;

            let backup = Frame::containing_address(cr3());

            // map temporary_page to current p4 table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // overwrite recursive mapping
            self.p4_mut()[511].set(inactive_table.p4_frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            f(self);

            p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(cr3()),
        };

        unsafe {
            asm!("mov cr3, {}", in(reg) new_table.p4_frame.start_address() as u64);
        }

        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable, temporary_page: &mut TemporaryPage) -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            table.zero();
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }

        temporary_page.unmap(active_table);
        InactivePageTable { p4_frame: frame }
    }
}

/// Maps the kernel structures in the higher half of virtual memory
pub fn setup_page_tables<A>(memory_map: &'static MemoryMapResponse, allocator: &mut A) -> ActivePageTable where A: FrameAllocator {
    serial_println!("mm: identity mapping kernel...");
    //let mut temporary_page = TemporaryPage::new(Page { number: 0x4000 + (*HHDM_OFFSET / PAGE_SIZE) }, allocator);

    serial_println!("table starts at {:X}", cr3());

    let mut active_table = unsafe { ActivePageTable::new_at(0x4000 + *HHDM_OFFSET) };
    /*
    let mut new_table = {
        let
            frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };*/

    // Higher half direct mapping
    let start_frame = Frame::containing_address(0);
    let end_frame = Frame::containing_address(0xFFE00000);
    for frame in Frame::range_inclusive(start_frame, end_frame) {
        let page = Page::containing_address(frame.start_address() + *HHDM_OFFSET);
        //serial_println!("MAPPING {:X} TO {:X}", page.start_address(), frame.start_address());

        active_table.map_to(page, frame, EntryFlags::WRITABLE, allocator);
    }



    // Kernel mapping
    for kernel_section in memory_map.entries().iter().filter(|entry| entry.entry_type == EntryType::KERNEL_AND_MODULES) {
        let start_frame = Frame::containing_address(kernel_section.base as usize);
        let end_frame = Frame::containing_address((kernel_section.base + kernel_section.length) as usize);
        for frame in Frame::range_inclusive(start_frame, end_frame) {
            let page = Page::containing_address(frame.start_address() + KERNEL_START_VMA_ADDRESS);
            active_table.map_to(page, frame, EntryFlags::WRITABLE, allocator);
        }
    }

    //active_table.with(&mut new_table, &mut temporary_page, |mapper| {



        /*
       let elf_sections = boot_info.elf_symbols().expect("Memory map required");

        // Remapping the kernel sections
        for section in elf_sections.section_headers() {
            if !section.is_allocated() {
                continue;
            }

            assert_eq!(section.start_address() % PAGE_SIZE, 0, "sections need to be page aligned");

            let start_frame = Frame::containing_address(section.start_address());
            let end_frame = Frame::containing_address(section.end_address() - 1);
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, EntryFlags::from_elf_section_flags(section), allocator);
            }
        }

        // Remapping the VGA buffer frame
        let vga_buffer_frame = Frame::containing_address(VGA_BUFFER_ADDRESS);
        mapper.identity_map(vga_buffer_frame, EntryFlags::WRITABLE | EntryFlags::USER_ACCESSIBLE, allocator);

        // Remapping the multiboot info
        let multiboot_start = Frame::containing_address(boot_info.start_address());
        let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);
        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            mapper.identity_map(frame, EntryFlags::PRESENT, allocator);
        }*/
    // });

    /*
    let old_table = active_table.switch(new_table);

    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);

    // TODO: Review this with regards to the higher half kernel
    ok!("mm: set up guard page at {:#X}", old_p4_page.start_address());
*/
    active_table
}
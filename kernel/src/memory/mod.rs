use core::ops::DerefMut;
use conquer_once::spin::OnceCell;
use limine::response::MemoryMapResponse;
use spin::Mutex;
use self::physical_memory::linear_frame_allocator::LinearFrameAllocator;
use self::physical_memory::buddy_allocator::BuddyAllocator;
use self::virtual_memory::paging::ActivePageTable;
use self::virtual_memory::paging::entry::EntryFlags;
use self::virtual_memory::heap_allocator::init_heap;
use crate::{serial_println};
use crate::memory::physical_memory::Frame;
use crate::memory::virtual_memory::paging::Page;

pub mod physical_memory;
pub mod virtual_memory;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub const PAGE_SIZE: usize = 4096;

pub static INSTANCE: OnceCell<Mutex<MemoryManager>> = OnceCell::uninit();
pub struct MemoryManager {
    frame_allocator: BuddyAllocator,
    active_page_table: ActivePageTable,
}

impl MemoryManager {
    pub fn init(memory_map: &'static MemoryMapResponse) {
        serial_println!("mm: init...");

        let mut linear_allocator = LinearFrameAllocator::new(memory_map);

        //let mut active_page_table = setup_page_tables(memory_map, &mut linear_allocator);
        let mut active_page_table = unsafe { ActivePageTable::new() };
        init_heap(&mut active_page_table, &mut linear_allocator);

        // Switch to the buddy allocator
        let mut buddy_allocator = BuddyAllocator::new(memory_map);
        buddy_allocator.set_allocated_frames(linear_allocator.allocated_frames());

        let memory_manager = Self {
            frame_allocator: buddy_allocator,
            active_page_table,
        };

        INSTANCE.try_init_once(|| Mutex::new(memory_manager)).expect("mm: cannot initialize memory manager more than once");
    }

    pub fn instance() -> &'static Mutex<MemoryManager> {
        INSTANCE.try_get().expect("mm: memory manager uninitialized")
    }

    pub fn vmm_alloc() {
        unimplemented!();
    }

    pub fn vmm_zero_alloc() {
        unimplemented!();
    }

    pub fn vmm_free() {
        unimplemented!();
    }

    /// Allocates enough physically contiguous identity mapped pages to cover the requested size
    pub fn pmm_alloc(size: usize, flags: EntryFlags) -> Option<PhysicalAddress> {
        let mut memory_manager = MemoryManager::instance().lock();

        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        let alloc_start = memory_manager.frame_allocator.allocate_frames(order);

        if let Some(alloc_start) = alloc_start {
            let alloc_size = 2usize.pow(order as u32);

            // Identity map the pages
            for page_number in 0..alloc_size {
                let page_address = alloc_start + PAGE_SIZE * page_number;
                let frame = Frame::containing_address(page_address);

                memory_manager.pmm_identity_map(frame, flags);
            }
        }

        alloc_start
    }

    pub fn pmm_zero_alloc(&mut self, _size: usize, _flags: EntryFlags) {
        unimplemented!();
    }

    pub fn pmm_free(size: usize, address: PhysicalAddress) {
        let mut memory_manager = MemoryManager::instance().lock();

        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        memory_manager.frame_allocator.deallocate_frames(address, order);

        let freed_size = 2usize.pow(order as u32);

        // Unmap the pages
        for page_number in 0..freed_size {
            let page_address = address + PAGE_SIZE * page_number;
            let page = Page::containing_address(page_address);

            memory_manager.active_page_table.deref_mut().unmap_no_dealloc(&page);
        }
    }

    pub fn pmm_identity_map(&mut self, frame: Frame, flags: EntryFlags) {
        self.active_page_table.identity_map(frame, flags, &mut self.frame_allocator);
    }
}
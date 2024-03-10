use core::ops::DerefMut;
use conquer_once::spin::OnceCell;
use limine::response::MemoryMapResponse;
use spin::Mutex;
use self::physical_memory::linear_frame_allocator::LinearFrameAllocator;
use self::physical_memory::buddy_allocator::BuddyAllocator;
use self::virtual_memory::paging::ActivePageTable;
use self::virtual_memory::paging::entry::EntryFlags;
use self::virtual_memory::heap_allocator::init_heap;
use crate::memory::physical_memory::{Frame, FrameAllocator};
use crate::memory::virtual_memory::paging::Page;
use crate::memory::virtual_memory::VirtualMemoryManager;

pub mod physical_memory;
pub mod virtual_memory;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub const PAGE_SIZE: usize = 4096;

pub static INSTANCE: OnceCell<Mutex<MemoryManager>> = OnceCell::uninit();
pub struct MemoryManager {
    frame_allocator: BuddyAllocator,
    active_page_table: ActivePageTable,
    virtual_memory_manager: VirtualMemoryManager,
}

impl MemoryManager {
    pub fn init(memory_map: &'static MemoryMapResponse) {
        serial_println!("mm: init...");

        let mut linear_allocator = LinearFrameAllocator::new(memory_map);

        //let mut active_page_table = setup_page_tables(memory_map, &mut linear_allocator);
        let mut active_page_table = unsafe { ActivePageTable::new() };
        init_heap(&mut linear_allocator, &mut active_page_table);

        // Switch to the buddy allocator
        let mut buddy_allocator = BuddyAllocator::new(memory_map);
        buddy_allocator.set_allocated_frames(linear_allocator.allocated_frames()).expect("mm: could not set allocated frames when switching allocators");

        let memory_manager = Self {
            frame_allocator: buddy_allocator,
            active_page_table,
            virtual_memory_manager: VirtualMemoryManager::new(),
        };

        INSTANCE.try_init_once(|| Mutex::new(memory_manager)).expect("mm: cannot initialize memory manager more than once");
    }

    pub fn instance() -> &'static Mutex<MemoryManager> {
        INSTANCE.try_get().expect("mm: memory manager uninitialized")
    }

    pub fn vmm_alloc(size: usize, flags: EntryFlags) -> Option<VirtualAddress> {
        let page_count = size.div_ceil(PAGE_SIZE);

        let mut memory_manager = MemoryManager::instance().lock();

        if let Some(virtual_alloc) = memory_manager.virtual_memory_manager.allocate_pages(page_count) {
            for i in 0..page_count {
                let page_address = virtual_alloc + i * PAGE_SIZE;
                let page = Page::containing_address(page_address);

                if let Ok(frame) =  memory_manager.frame_allocator.allocate_frame() {
                    memory_manager.vmm_map_to(page, frame, flags);
                }
                else {
                    panic!("vmm: ran out of physical memory when allocating {} pages", size);
                }
            }

            return Some(virtual_alloc)
        }

        None
    }

    pub fn vmm_zero_alloc(_size: usize, _flags: EntryFlags) -> Option<VirtualAddress> {
        unimplemented!()
    }

    pub fn vmm_free(_size: usize, _address: VirtualAddress) {
        unimplemented!();
    }

    pub fn pmm_alloc(size: usize) -> Option<PhysicalAddress> {
        let mut memory_manager = MemoryManager::instance().lock();

        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        let alloc = memory_manager.frame_allocator.allocate_frames(order).expect("pmm: could not allocate memory");
        Some(alloc)
    }

    /// Allocates enough physically contiguous identity mapped pages to cover the requested size
    pub fn pmm_identity(size: usize, flags: EntryFlags) -> Option<PhysicalAddress> {
        let mut memory_manager = MemoryManager::instance().lock();

        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm_alloc: could not allocate memory");

        let alloc_start = memory_manager.frame_allocator.allocate_frames(order).expect("pmm: could not identity map");
        let alloc_size = 2usize.pow(order as u32);

        // Identity map the pages
        for page_number in 0..alloc_size {
            let page_address = alloc_start + PAGE_SIZE * page_number;
            let frame = Frame::containing_address(page_address);

            memory_manager.pmm_identity_map(frame, flags);
        }

        Some(alloc_start)
    }

    pub fn pmm_zero_alloc(&mut self, _size: usize, _flags: EntryFlags) {
        unimplemented!();
    }

    pub fn pmm_free(size: usize, address: PhysicalAddress) {
        let mut memory_manager = MemoryManager::instance().lock();

        let page_count = size.div_ceil(PAGE_SIZE);
        let order = (0..=10).find(|&x| 2usize.pow(x as u32) >= page_count).expect("pmm: could not free memory");

        memory_manager.frame_allocator.deallocate_frames(address, order).expect("pmm: could not free memory");

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

    fn vmm_map_to(&mut self, page: Page, frame: Frame, flags: EntryFlags) {
        self.active_page_table.map_to(page, frame, flags, &mut self.frame_allocator);
    }
}
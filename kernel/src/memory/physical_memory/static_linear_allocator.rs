use core::mem::size_of;
use core::ptr;
use limine::memory_map;
use limine::memory_map::EntryType;
use linked_list_allocator::align_up;
use rlibc::memset;
use crate::{HHDM_OFFSET, test_bit};
use crate::memory::{PAGE_SIZE, PhysicalAddress};
use crate::memory::physical_memory::{Frame, FrameAllocator};

struct PmmModule {
    start_address: PhysicalAddress,

    bitmap_size: usize,
    bitmap_entry_count: usize,
    last_free: Option<usize>,
    bitmap: *mut u8,

    next: Option<*mut PmmModule>,
}
impl PmmModule {
    fn init(start_address: PhysicalAddress, size: usize, memory_maps_start: *mut u8) -> Self {
        let frame_count = size.div_ceil(PAGE_SIZE);

        let module = Self {
            start_address,

            bitmap_size: frame_count.div_ceil(8),
            bitmap_entry_count: frame_count,
            bitmap: memory_maps_start,

            last_free: None,
            next: None,
        };

        unsafe {
            memset(module.bitmap, 0, module.bitmap_size);
        }

        module
    }

    fn allocate_frame(&mut self) -> Option<PhysicalAddress> {
        if let Some(last_free) = self.last_free {
            let alloc = self.start_address + last_free * PAGE_SIZE;
            let bit_base = (alloc - self.start_address) / PAGE_SIZE;

            for i in bit_base..self.bitmap_entry_count {
                let byte_index = bit_base / 8;
                if !test_bit!(unsafe { *self.bitmap.add(byte_index) }, bit_base as usize) {
                    self.last_free = Some(bit_base + i);
                    break;
                }
            }

            return Some(alloc);
        }

        None
    }
}

struct StaticLinearAllocator {
    root_module: &'static mut PmmModule,
}
impl StaticLinearAllocator {
    pub fn init(memory_regions: &[&memory_map::Entry]) -> Result<Self, &'static str> {
        // Calculate how much memory will be necessary to accommodate the allocator
        let buffer_size = memory_regions
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .fold(0, |acc, entry|
                acc + size_of::<PmmModule>() * 2 + entry.length.div_ceil(PAGE_SIZE as u64).div_ceil(8) as usize);

        // Find an available region large enough to fit everything
        let containing_entry = memory_regions
            .iter()
            .find(|entry| entry.entry_type == EntryType::USABLE && entry.length >= buffer_size as u64)
            .ok_or("pmm: could not find a suitable memory region to hold the pmm")?;
        let buffer_start = align_up(containing_entry.base as usize + *HHDM_OFFSET, PAGE_SIZE);
        let mut meta_buffer = buffer_start as *mut u8;

        // Create modules for all regions
        let mut root_module: Option<*mut PmmModule> = None;
        memory_regions.iter().filter(|entry| entry.entry_type == EntryType::USABLE).for_each(|entry| {
            match root_module {
                None => {
                    if root_module.is_none() {
                        unsafe {
                            let module = PmmModule::init(entry.base as PhysicalAddress, entry.length as usize, meta_buffer);
                            ptr::write(meta_buffer as *mut PmmModule, module);
                            root_module = Some(&mut *(meta_buffer as *mut PmmModule));

                            meta_buffer = meta_buffer.add(size_of::<PmmModule>() * 2);
                        }
                    }
                },
                Some(mut root) => {
                    let mut node = unsafe { &mut *root };
                    while let Some(next) = node.next {
                       node = unsafe { &mut * next };
                    }

                    unsafe {
                        let module = PmmModule::init(entry.base as PhysicalAddress, entry.length as usize, meta_buffer);
                        ptr::write(meta_buffer as *mut PmmModule, module);
                        node.next = Some(meta_buffer as *mut PmmModule);

                        meta_buffer = meta_buffer.add(size_of::<PmmModule>() * 2);
                    }
                }
            }
        });

        Ok(Self {
            root_module: unsafe { &mut *root_module.unwrap() },
        })
    }
}
impl FrameAllocator for StaticLinearAllocator {
    fn allocate_frame(&mut self) -> Result<Frame, &'static str> {
        let mut module = unsafe { &mut *(self.root_module as *mut PmmModule) };
        loop {
            let alloc = module.allocate_frame();

            // Return the frame if it was found
            if let Some(alloc) = alloc {
                let frame = Frame::containing_address(alloc);
                return Ok(frame);
            }
            // Try again with the next module if it exists, otherwise fail
            else {
                if let Some(next) = module.next {
                    module = unsafe { &mut *next };
                }
                else {
                    return Err("pmm: could not allocate frame (memory full)");
                }
            }
        }
    }

    fn deallocate_frame(&mut self, frame: Frame) -> Result<(), &'static str> {
        todo!()
    }
}
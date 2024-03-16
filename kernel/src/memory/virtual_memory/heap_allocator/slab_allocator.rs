use core::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};
use core::ptr::NonNull;
use crate::memory::{MemoryManager, PAGE_SIZE, VirtualAddress};
use crate::memory::virtual_memory::paging::entry::EntryFlags;
use super::Locked;

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct ListNode {
    next: Option<&'static mut ListNode>
}

pub struct SlabAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
    allocated_bytes: usize,
}

impl SlabAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        SlabAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
            allocated_bytes: 0,
        }
    }

    pub unsafe fn init(&mut self, heap_start: VirtualAddress, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size);
    }

    unsafe fn extend_heap(&mut self) {
        self.fallback_allocator.extend(2*PAGE_SIZE);
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        if let Ok(ptr) = self.fallback_allocator.allocate_first_fit(layout) {
            return ptr.as_ptr();
        }

        // Extend the heap if the allocation failed and try again
        MemoryManager::vmm_alloc(2 * PAGE_SIZE, EntryFlags::WRITABLE);

        unsafe { self.extend_heap(); }

        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => {
                ptr.as_ptr()
            },
            Err(_) => {
                ptr::null_mut()
            }
        }
    }
}

unsafe impl GlobalAlloc for Locked<SlabAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        allocator.allocated_bytes += layout.size();
        serial_println!("Allocating {} bytes... {} bytes currently allocated", layout.size(), allocator.allocated_bytes);

        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        let block_size = BLOCK_SIZES[index];
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();

                        let alloc = allocator.fallback_alloc(layout);
                        alloc
                    }
                }
            }
            None => {
                let alloc = allocator.fallback_alloc(layout);
                alloc
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        allocator.allocated_bytes -= layout.size();
        //serial_println!("Deallocating {} bytes... {} bytes currently allocated", layout.size(), allocator.allocated_bytes);

        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}

fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}
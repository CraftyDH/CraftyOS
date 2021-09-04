use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use crate::locked_mutex::Locked;

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
fn list_index(layout: &Layout) -> Option<usize> {
    // Block size must be at least the size of the alignment
    let required_block_size = layout.size().max(layout.align());
    // Find smallest block
    BLOCK_SIZES
        .iter()
        .position(|&size| size >= required_block_size)
}

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    // Initilize the allocator with heap bounds
    //* Unsafe becuase
    //* Guarantee the this function is only called once
    //* Heap bounds must be valid and unused
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size)
    }

    fn fallboack_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => core::ptr::null_mut(),
        }
    }
}
unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // No block exists in list => allocate new block
                        let block_size = BLOCK_SIZES[index];
                        // Only works if all blocks are powers of 2
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        allocator.fallboack_alloc(layout)
                    }
                }
            }
            None => allocator.fallboack_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                // Return block to correct list size
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                // Verify that the block has correct size and alignment
                assert!(core::mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(core::mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => {
                // Just dealloc it
                let pointer = NonNull::new(ptr).unwrap();
                allocator.fallback_allocator.deallocate(pointer, layout);
            }
        }
    }
}

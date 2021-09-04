use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

use crate::locked_mutex::Locked;

use super::align_up;

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    // Create a new Bump Allocator
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    // Initalize the bump allocator
    //* Unsafe because
    //* Ensure only called once
    //* Ensure only called on unused memory
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            return ptr::null_mut(); // out of memory :(
        }

        // Increment variables to reflect to allocated block
        bump.next = alloc_end;
        bump.allocations += 1;
        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;

        // If no more allocation restart pool at 0
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}

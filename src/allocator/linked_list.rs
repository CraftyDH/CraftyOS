use core::{alloc::{self, GlobalAlloc}, mem};

use crate::{allocator::align_up, locked_mutex::Locked};

struct ListNode {
    size: usize, // Size of this free memory chunk
    next: Option<&'static mut ListNode> // Memory location of next free memory chunk
}

impl ListNode {
    const fn new(size: usize) -> Self {
        Self {
            size,
            next: None
        }
    }

    fn start_addr(&self) -> usize {
        // get the location the node is in memory
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    // First free mem location
    head: ListNode
}

impl LinkedListAllocator {
    // Creates a new Linked List Allocator
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0)
        }
    }


    // Initilize the allocator with heap bounds
    //* Unsafe becuase
    //* Guarantee the this function is only called once
    //* Heap bounds must be valid and unused
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    // Adds the new memory location to the front of the list
    unsafe fn add_free_region(&mut self, addr:usize, size: usize) {
        // Ensure that the freed region is capable of holding a ListNode
        assert_eq!(align_up(addr, core::mem::align_of::<ListNode>()), addr);
        assert!(size >= core::mem::size_of::<ListNode>());

        // Create a new list node
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();

        // Write the list node to memory
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);

        // Write the location to the next free mem location
        self.head.next = Some(&mut *node_ptr)
    }

    // Looks for a free regoin with the givin size and alignment and takes it
    // Return a tuple of the list node and start address
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // Refrence to current list node, updated for each iteration
        let mut previous_region = &mut self.head;

        while let Some(ref mut this_region) = previous_region.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&this_region, size, align) {
                // Region is suitable

                // Get memory location of the next free chunk
                let next_free = this_region.next.take();
                // Make tuple with this location and the start address of allocation
                let ret = Some((previous_region.next.take().unwrap(), alloc_start));
                // Set the previous regions pointer to the next free chunk
                previous_region.next = next_free;
                return ret;
            } else {
                previous_region = previous_region.next.as_mut().unwrap();
            }
        }

        // No suitable region found
        None
    }

    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // Region too small
            return Err(())
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // Rest of region is too small to hold a ListNode becuase the allocation splits the allocation into a used and free part
            return Err(())
        }

        Ok(alloc_start)
    }

    fn size_align(layout: alloc::Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("Adjusting alignment failed :(")
            // Pad so memory size is a multiple of Listnode
            .pad_to_align();

        // Enforce that the minumum size of an allocation is a ListNode
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: alloc::Layout) -> *mut u8 {
        // Perform layout ajustments
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            // End of allocated memory
            let alloc_end = alloc_start.checked_add(size).expect("overflow");

            // How much excess memory?
            let excess_size = region.end_addr() - alloc_end;
            // If excess memory, add it the the list
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: alloc::Layout) {
        // Perform layout ajustments
        let (size, _) = LinkedListAllocator::size_align(layout);

        self.lock().add_free_region(ptr as usize, size)
    }
}
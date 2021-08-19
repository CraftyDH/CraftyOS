pub mod page_frame_allocator;
pub mod paging;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

pub const PAGE_SIZE: usize = 4096;



impl Frame {
    pub fn containing_address(address: usize) -> Frame {
        Frame {
            number: address / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

pub fn get_memory_size_in_pages(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> u64 {
    let mut memory_size: u64 = 0;
    for describe in mmap {
        memory_size += describe.page_count;
    }
    memory_size
}

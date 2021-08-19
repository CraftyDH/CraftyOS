// Page frame allocator

use crate::bitmap::Bitmap;

pub struct PageFrameAllocator {
    free_memory: u64,
    reserved_memory: u64,
    used_memory: u64,
    page_bitmap: Bitmap,
    page_bitmap_index: usize,
}

impl PageFrameAllocator {
    pub fn new(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> PageFrameAllocator {
        let (free_segment, free_segment_size) = PageFrameAllocator::get_largest_section(mmap);
        let memory_size = crate::memory::get_memory_size(mmap);
        let bitmap_size = memory_size / 4096 / 8 + 1;

        // Init bitmap
        let bitmap = Bitmap::new(bitmap_size as usize, free_segment);

        // Lock pages of bitmap
        // Reserve pages of unusable/reserved memory

        let mut alloc = PageFrameAllocator {
            free_memory: memory_size,
            reserved_memory: 0,
            used_memory: 0,
            page_bitmap: bitmap,
            page_bitmap_index: 0,
        };

        alloc.lock_pages(
            alloc.page_bitmap.buffer,
            alloc.page_bitmap.size as u64 / 4096 + 1,
        );

        for &mut entry in mmap {
            // TODO: We should lock the kernel. However since it was loaded using UEFI loader data it should be locked.
            if entry.ty != uefi::table::boot::MemoryType::CONVENTIONAL {
                alloc.reserve_pages(entry.phys_start as *mut u8, entry.page_count)
            }
        }
        return alloc;
    }

    fn get_largest_section(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> (*mut u8, u64) {
        let mut free_segment: *mut u8 = 0 as *mut u8;
        let mut free_segment_size = 0;
        for &mut entry in mmap {
            if entry.ty == uefi::table::boot::MemoryType::CONVENTIONAL {
                if entry.page_count * 4096 > free_segment_size {
                    free_segment = entry.phys_start as *mut u8;
                    free_segment_size = entry.page_count
                }
            }
        }
        return (free_segment, free_segment_size * 4096);
    }

    pub fn request_page(&mut self) -> *mut u8 {
        for page in self.page_bitmap_index..(self.page_bitmap.size * 8) {
            if self.page_bitmap[page] == true {
                continue;
            }
            self.page_bitmap_index = page;
            let ptr = (page * 4096) as *mut u8;
            self.lock_page(ptr);
            return ptr;
        }

        //? SWAP
        return core::ptr::null_mut();
    }

    pub fn get_free_ram(&self) -> u64 {
        return self.free_memory;
    }

    pub fn get_used_ram(&self) -> u64 {
        return self.used_memory;
    }
    pub fn get_reserved_ram(&self) -> u64 {
        return self.reserved_memory;
    }
}

impl PageFrameAllocator {
    pub fn free_page(&mut self, addr: *mut u8) {
        let index = (addr as u64 / 4096) as usize;
        if self.page_bitmap[index] == false {
            return;
        };
        if self.page_bitmap.set(index as u64, false) {
            self.free_memory += 4096;
            self.used_memory -= 4096;

            if self.page_bitmap_index > index {
                self.page_bitmap_index = index
            }
        }
    }

    pub fn free_pages(&mut self, addr: *mut u8, page_count: u64) {
        for page in 0..page_count {
            self.free_page((addr as u64 + page * 4096) as *mut u8)
        }
    }

    pub fn lock_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == true {
            return;
        }
        if self.page_bitmap.set(index, true) {
            self.free_memory -= 4096;
            self.used_memory += 4096;
        }
    }

    pub fn lock_pages(&mut self, addr: *mut u8, page_count: u64) {
        for page in 0..page_count {
            self.lock_page((addr as u64 + page * 4096) as *mut u8);
        }
    }
}

impl PageFrameAllocator {
    pub fn unreserve_page(&mut self, addr: *mut u8) {
        let index = (addr as u64 / 4096) as usize;
        if self.page_bitmap[index] == false {
            return;
        };
        if self.page_bitmap.set(index as u64, false) {
            self.free_memory += 4096;
            self.reserved_memory -= 4096;
            if self.page_bitmap_index > index {
                self.page_bitmap_index = index
            }
        }
    }

    pub fn unreserve_pages(&mut self, addr: *mut u8, page_count: u64) {
        for page in 0..page_count {
            self.unreserve_page((addr as u64 + page * 4096) as *mut u8);
        }
    }

    pub fn reserve_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == true {
            return;
        };
        if self.page_bitmap.set(index, true) {
            self.free_memory -= 4096;
            self.reserved_memory += 4096;
        }
    }

    pub fn reserve_pages(&mut self, addr: *mut u8, page_count: u64) {
        let index = (addr as u64) / 4096;
        for page in 0..page_count {
            self.reserve_page((addr as u64 + page * 4096) as *mut u8);
        }
    }
}

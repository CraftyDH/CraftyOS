// Page frame allocator

use crate::bitmap::Bitmap;

pub struct PageFramAllocator {
    free_memory: u64,
    reserved_memory: u64,
    used_memory: u64,
    page_bitmap: Bitmap,
}

impl PageFramAllocator {
    pub fn new(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> PageFramAllocator {
        let (free_segment, free_segment_size) = PageFramAllocator::get_largest_section(mmap);
        let memory_size = super::memory::get_memory_size(mmap);
        let bitmap_size = memory_size / 4096 / 8 + 1;

        // Init bitmap
        let bitmap = Bitmap::new(bitmap_size as usize, free_segment);

        // Lock pages of bitmap
        // Reserve pages of unusable/reserved memory

        let mut alloc = PageFramAllocator {
            free_memory: memory_size,
            reserved_memory: 0,
            used_memory: 0,
            page_bitmap: bitmap,
        };

        alloc.lock_pages(alloc.page_bitmap.buffer, bitmap_size / 4096 + 1);

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
                if entry.page_count > free_segment_size {
                    free_segment = entry.phys_start as *mut u8;
                    free_segment_size = entry.page_count
                }
            }
        }
        return (free_segment, free_segment_size);
    }

    pub fn request_page(&mut self) -> Result<*mut u8, &str> {
        for page in 0..self.page_bitmap.size {
            if self.page_bitmap[page] == true {
                continue;
            }
            let ptr = (page * 4096) as *mut u8;

            self.lock_page(ptr);
            return Ok(ptr);
        }

        //? SWAP
        return Err("No page found");
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

impl PageFramAllocator {
    pub fn free_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == false {
            return;
        };
        self.page_bitmap.set(index, false);
        self.free_memory += 4096;
        self.used_memory -= 4096;
    }

    pub fn free_pages(&mut self, addr: *mut u8, page_count: u64) {
        let index = (addr as u64) / 4096;
        for page in 0..page_count {
            unsafe { self.free_page(addr.offset((page * 4096) as isize)) };
        }
    }

    pub fn lock_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == true {
            return;
        };
        self.page_bitmap.set(index, true);
        self.free_memory -= 4096;
        self.used_memory += 4096;
    }

    pub fn lock_pages(&mut self, addr: *mut u8, page_count: u64) {
        let index = (addr as u64) / 4096;
        for page in 0..page_count {
            unsafe { self.lock_page(addr.offset((page * 4096) as isize)) };
        }
    }
}

impl PageFramAllocator {
    pub fn unreserve_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == false {
            return;
        };
        self.page_bitmap.set(index, false);
        self.free_memory += 4096;
        self.reserved_memory -= 4096;
    }

    pub fn unreserve_pages(&mut self, addr: *mut u8, page_count: u64) {
        let index = (addr as u64) / 4096;
        for page in 0..page_count {
            unsafe { self.unreserve_page(addr.offset((page * 4096) as isize)) };
        }
    }

    pub fn reserve_page(&mut self, addr: *mut u8) {
        let index = (addr as u64) / 4096;
        if self.page_bitmap[index as usize] == true {
            return;
        };
        self.page_bitmap.set(index, true);
        self.free_memory -= 4096;
        self.reserved_memory += 4096;
    }

    pub fn reserve_pages(&mut self, addr: *mut u8, page_count: u64) {
        let index = (addr as u64) / 4096;
        for page in 0..page_count {
            unsafe { self.reserve_page(addr.offset((page * 4096) as isize)) };
        }
    }
}

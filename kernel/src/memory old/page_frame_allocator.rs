use super::{Frame, FrameAllocator};
use crate::bitmap::Bitmap;

pub struct PageFrameAllocator {
    pub free_pages: u64,
    pub used_pages: u64,
    total_pages: u64,
    page_bitmap: Bitmap,
    page_bitmap_index: usize,
}

impl PageFrameAllocator {
    pub fn new(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> PageFrameAllocator {
        let (free_segment, free_segment_size) = PageFrameAllocator::get_largest_section(mmap);
        let total_pages = crate::memory::get_memory_size_in_pages(mmap);
        let bitmap_size = total_pages / 8;

        println!("Total pages {}", total_pages);

        // Init bitmap
        let bitmap = Bitmap::new(bitmap_size as usize, free_segment.number as *mut u8);

        // Lock pages of bitmap
        // Reserve pages of unusable/reserved memory

        let mut alloc = PageFrameAllocator {
            free_pages: total_pages,
            used_pages: 0,
            total_pages: total_pages,
            page_bitmap: bitmap,
            page_bitmap_index: 0,
        };

        alloc.lock_pages(free_segment, alloc.page_bitmap.size as u64 + 1);

        for &mut entry in mmap {
            // TODO: We should lock the kernel. However since it was loaded using UEFI loader data it should be locked.
            if entry.ty != uefi::table::boot::MemoryType::CONVENTIONAL {
                alloc.lock_pages(
                    Frame::containing_address(entry.phys_start as usize),
                    entry.page_count,
                )
            }
        }
        println!("Free pages {}", alloc.free_pages);

        return alloc;
    }

    fn get_largest_section(mmap: &mut [uefi::table::boot::MemoryDescriptor]) -> (Frame, u64) {
        let mut free_segment: usize = 0;
        let mut free_segment_size = 0;
        for &mut entry in mmap {
            if entry.ty == uefi::table::boot::MemoryType::CONVENTIONAL {
                if entry.page_count > free_segment_size {
                    free_segment = entry.phys_start as usize;
                    free_segment_size = entry.page_count
                }
            }
        }

        let frame = Frame {
            number: free_segment,
        };

        return (frame, free_segment_size);
    }

    pub fn deallocate_frames(&mut self, inital_page: Frame, page_count: u64) {
        for page in 0..page_count {
            self.deallocate_frame(Frame {
                number: (inital_page.number + page as usize),
            });
        }
    }

    pub fn lock_page(&mut self, page: Frame) {
        if self.page_bitmap[page.number] == true {
            println!("Page {} allready locked", page.number);
            return;
        }

        if self.page_bitmap.set(page.number as u64, true) {
            self.free_pages -= 1;
            self.used_pages += 1;
        } else {
            panic!("Couldn't set lock page");
        }
    }

    pub fn lock_pages(&mut self, inital_page: Frame, page_count: u64) {
        for page in 0..page_count {
            self.lock_page(Frame {
                number: (inital_page.number + page as usize),
            });
        }
    }
}
impl FrameAllocator for PageFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        for page in self.page_bitmap_index..(self.total_pages as usize) {
            if self.page_bitmap[page] == true {
                continue;
            }
            if self.free_pages == 0 {
                println!("No more free pages: FIX da math");
                return None;
            }

            // println!("Allocating frame: {}, {} left", page, self.free_pages);
            self.page_bitmap_index = page;
            self.lock_page(Frame { number: page });
            let frame = Frame { number: page };
            return Some(Frame { number: page });
        }
        println!("No frame found!");
        return None;
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        if self.page_bitmap[frame.number] == false {
            return;
        };
        if self.page_bitmap.set(frame.number as u64, false) {
            self.free_pages += 1;
            self.used_pages -= 1;

            if self.page_bitmap_index > frame.number {
                self.page_bitmap_index = frame.number
            }
        }
    }
}

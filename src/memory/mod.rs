use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, PhysFrame, Size4KiB};
use x86_64::{registers::control::Cr3, structures::paging::PageTable, PhysAddr, VirtAddr};

unsafe fn active_lvl4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (lv4_table, _) = Cr3::read();

    let phys = lv4_table.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let lvl4_table = active_lvl4_table(physical_memory_offset);
    OffsetPageTable::new(lvl4_table, physical_memory_offset)
}

/// A FrameAllocator that returns usable frams from the bootloader's memory map
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        Self {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // Get all regions from memory map
        let regions = self.memory_map.iter();

        // Filter out memory regions that aren't usable
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

        // Create iterator of the range of available regions
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

        // Transform to an iterator of frame start addresses.
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
